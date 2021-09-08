/*
 * Part of epicinium_server
 * developed by A Bunch of Hacks.
 *
 * Copyright (c) 2018-2021 A Bunch of Hacks
 *
 * This library is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This library is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 * [authors:]
 * Sander in 't Veld (sander@abunchofhacks.coop)
 */

use crate::common::coredump::enable_coredumps;
use crate::common::keycode::*;
use crate::logic::challenge;
use crate::logic::ruleset;
use crate::server::chat;
use crate::server::client;
use crate::server::discord_api;
use crate::server::login;
use crate::server::logrotate;
use crate::server::portal;
use crate::server::rating;
use crate::server::settings::*;
use crate::server::slack_api;
use crate::server::terminate;

use std::net::SocketAddr;
use std::sync;
use std::sync::atomic;

use log::*;

use anyhow::anyhow;

use futures::future;
use futures::{FutureExt, StreamExt};

use tokio::net::TcpListener;
use tokio::signal::unix::SignalKind;
use tokio::sync::mpsc;
use tokio::sync::watch;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum State
{
	Open,
	Closing,
	Closed,
}

pub struct Server
{
	scoped_terminate: terminate::Setup,
	log_setup: logrotate::Setup,
	login_server: login::Server,
	portal_setup: portal::Setup,
	slack_setup: slack_api::Setup,
	discord_setup: discord_api::Setup,
	rating_database: rating::Database,
	challenge_pool: Vec<challenge::Challenge>,
	ip_address: String,
}

pub fn setup_server(
	settings: &Settings,
	log_setup: logrotate::Setup,
) -> Result<Server, anyhow::Error>
{
	enable_coredumps()?;
	increase_sockets()?;

	let scoped_terminate = terminate::setup()?;

	let server = settings
		.server
		.as_ref()
		.ok_or_else(|| anyhow!("missing 'server'"))?;
	let ip_address = server.to_string();

	ruleset::initialize_collection()?;

	let server = Server {
		scoped_terminate,
		log_setup,
		login_server: login::connect(settings)?,
		portal_setup: portal::setup(settings)?,
		slack_setup: slack_api::setup(settings)?,
		discord_setup: discord_api::setup(settings)?,
		rating_database: rating::initialize(settings)?,
		challenge_pool: challenge::load_pool()?,
		ip_address,
	};
	Ok(server)
}

#[tokio::main]
pub async fn run_server(server: Server)
{
	let Server {
		scoped_terminate,
		log_setup,
		login_server,
		portal_setup,
		slack_setup,
		discord_setup,
		rating_database,
		challenge_pool,
		ip_address,
	} = server;

	let (slack_in, slack_out) = mpsc::channel::<slack_api::Post>(10000);
	let slack_task = slack_api::run(slack_setup, slack_out);

	let (rating_in, rating_out) = mpsc::channel::<rating::Update>(10000);
	let rating_task = rating::run(rating_database, rating_out);

	let (discord_in, discord_out) = mpsc::channel::<discord_api::Post>(10000);
	let discord_task = discord_api::run(discord_setup, discord_out);

	let (state_in, state_out) = watch::channel(State::Open);
	let (client_canary_in, client_canary_out) = mpsc::channel::<()>(1);
	let (general_canary_in, general_canary_out) = mpsc::channel::<()>(1);
	let close_task =
		wait_for_close(general_canary_out, client_canary_out, state_in);

	let (general_in, general_out) = mpsc::channel::<chat::Update>(10000);
	let chat_task = chat::run(general_out, general_canary_in, &challenge_pool);

	let logrotate_task =
		logrotate::run(log_setup, state_out.clone(), slack_in.clone());

	let acceptance_task = accept_clients(
		ip_address,
		login_server,
		portal_setup,
		general_in,
		rating_in,
		slack_in,
		discord_in,
		state_out,
		client_canary_in,
	);

	let server_task = future::join4(
		acceptance_task,
		future::join(chat_task, rating_task),
		future::join3(slack_task, discord_task, logrotate_task),
		close_task,
	)
	.map(|((), ((), ()), ((), (), ()), ())| ());

	server_task.await;

	let _discarded = scoped_terminate;
}

async fn accept_clients(
	ip_address: String,
	login_server: login::Server,
	portal_setup: portal::Setup,
	general_chat: mpsc::Sender<chat::Update>,
	ratings: mpsc::Sender<rating::Update>,
	slack_api: mpsc::Sender<slack_api::Post>,
	discord_api: mpsc::Sender<discord_api::Post>,
	server_state: watch::Receiver<State>,
	client_canary: mpsc::Sender<()>,
)
{
	let binding = match portal::bind(portal_setup).await
	{
		Ok(binding) => binding,
		Err(error) =>
		{
			error!("Error running server: {}", error);
			error!("{:#?}", error);
			println!("Error running server: {}", error);
			return;
		}
	};
	// If binding succeeds, we must unbind.

	let port = binding.port;
	let address: SocketAddr = match format!("{}:{}", ip_address, port).parse()
	{
		Ok(address) => address,
		Err(error) =>
		{
			warn!("Failed to parse '{}:{}': {}", ip_address, port, error);
			let localhost = std::net::Ipv4Addr::new(127, 0, 0, 1);
			SocketAddr::new(std::net::IpAddr::V4(localhost), port)
		}
	};

	let listener = match TcpListener::bind(&address).await
	{
		Ok(listener) => match binding.confirm().await
		{
			Ok(()) => Some(listener),
			Err(error) =>
			{
				error!("Error running server: {}", error);
				error!("{:#?}", error);
				println!("Error running server: {}", error);
				None
			}
		},
		Err(error) =>
		{
			error!("Error running server: {}", error);
			error!("{:#?}", error);
			println!("Error running server: {}", error);
			None
		}
	};

	if let Some(listener) = listener
	{
		info!("Listening on {}...", address);

		listen(
			listener,
			login_server,
			general_chat,
			ratings,
			slack_api,
			discord_api,
			server_state,
			client_canary,
		)
		.await;

		info!("Stopped listening.");
	}

	match binding.unbind().await
	{
		Ok(()) => (),
		Err(error) =>
		{
			error!("Error running server: {}", error);
			error!("{:#?}", error);
			println!("Error running server: {}", error);
		}
	}
}

async fn listen(
	mut listener: TcpListener,
	login_server: login::Server,
	general_chat: mpsc::Sender<chat::Update>,
	ratings: mpsc::Sender<rating::Update>,
	slack_api: mpsc::Sender<slack_api::Post>,
	discord_api: mpsc::Sender<discord_api::Post>,
	server_state: watch::Receiver<State>,
	client_canary: mpsc::Sender<()>,
)
{
	let login = sync::Arc::new(login_server);

	let mut ticker: u64 = rand::random();
	let lobbyticker = sync::Arc::new(atomic::AtomicU64::new(rand::random()));

	let closing = wait_for_closing(server_state.clone()).boxed();
	let mut connections = listener.incoming().take_until(closing);

	while let Some(socket) = connections.next().await
	{
		let socket = match socket
		{
			Ok(socket) => socket,
			Err(error) =>
			{
				warn!("Failed to connect client: {:?}", error);
				continue;
			}
		};

		info!("Accepting incoming connection: {:?}", socket);

		let serial = ticker;
		ticker += 1;
		let key: u16 = rand::random();
		let id = keycode(key, serial);

		client::accept(
			socket,
			id,
			login.clone(),
			general_chat.clone(),
			ratings.clone(),
			slack_api.clone(),
			discord_api.clone(),
			server_state.clone(),
			client_canary.clone(),
			lobbyticker.clone(),
		);

		info!("Accepted client {}.", id);
	}
}

async fn wait_for_closing(mut server_state: watch::Receiver<State>)
{
	while let Some(state) = server_state.next().await
	{
		match state
		{
			State::Open => continue,
			State::Closing => break,
			State::Closed => break,
		}
	}
}

async fn wait_for_close(
	chat_canary: mpsc::Receiver<()>,
	client_canary: mpsc::Receiver<()>,
	server_state: watch::Sender<State>,
)
{
	let handler = match tokio::signal::unix::signal(SignalKind::terminate())
	{
		Ok(handler) => Some(handler),
		Err(error) =>
		{
			error!("Error running server: {}", error);
			error!("{:#?}", error);
			println!("Error running server: {}", error);
			None
		}
	};

	if let Some(handler) = handler
	{
		let chat_closed = wait_for_canary(chat_canary).boxed();
		let mut signals = handler.take_until(chat_closed);

		let mut is_open = true;
		while let Some(()) = signals.next().await
		{
			if is_open
			{
				info!("Closing...");
				match server_state.broadcast(State::Closing)
				{
					Ok(()) => (),
					Err(error) =>
					{
						error!("Error running server: {}", error);
						error!("{:#?}", error);
						println!("Error running server: {}", error);
					}
				}
				is_open = false;
			}
			else
			{
				break;
			}
		}
	}

	info!("Closed.");
	// If all clients are disconnected, no one will receive the broadcast.
	let _ = server_state.broadcast(State::Closed);

	wait_for_canary(client_canary).await;
	info!("All clients have disconnected.");
}

async fn wait_for_canary(mut canary: mpsc::Receiver<()>)
{
	while let Some(()) = canary.recv().await
	{
		// Nothing to do.
	}
}

fn increase_sockets() -> std::io::Result<()>
{
	if !cfg!(feature = "no-increase-sockets")
	{
		const MAX_SOCKETS: rlimit::rlim = 16384;
		rlimit::Resource::NOFILE.set(MAX_SOCKETS, MAX_SOCKETS)
	}
	else
	{
		warn!("Limited sockets available.");
		Ok(())
	}
}
