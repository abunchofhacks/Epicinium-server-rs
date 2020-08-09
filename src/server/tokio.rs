/* Server::Tokio */

use crate::common::coredump::enable_coredumps;
use crate::common::keycode::*;
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
use futures::{FutureExt, StreamExt, TryFutureExt};

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

#[tokio::main]
pub async fn run_server(
	settings: &Settings,
	log_setup: logrotate::Setup,
) -> Result<(), anyhow::Error>
{
	enable_coredumps()?;
	increase_sockets()?;

	let _scoped_terminate = terminate::setup()?;

	ruleset::initialize_collection()?;

	let (slack_in, slack_out) = mpsc::channel::<slack_api::Post>(10000);
	let slack_task = slack_api::run(settings, slack_out);

	let login_server = login::connect(settings)?;
	let login = sync::Arc::new(login_server);

	let (rating_in, rating_out) = mpsc::channel::<rating::Update>(10000);
	let rating_task = rating::run(settings, rating_out);

	let (discord_in, discord_out) = mpsc::channel::<discord_api::Post>(10000);
	let discord_task = discord_api::run(settings, discord_out);

	let (state_in, state_out) = watch::channel(State::Open);
	let (client_canary_in, client_canary_out) = mpsc::channel::<()>(1);
	let (general_canary_in, general_canary_out) = mpsc::channel::<()>(1);
	let close_task =
		wait_for_close(general_canary_out, client_canary_out, state_in);

	let (general_in, general_out) = mpsc::channel::<chat::Update>(10000);
	let chat_task = chat::run(general_out, general_canary_in).map(|()| Ok(()));

	let logrotate_task =
		logrotate::run(log_setup, state_out.clone(), slack_in.clone())
			.map(|()| Ok(()));

	let acceptance_task = accept_clients(
		settings,
		login,
		general_in,
		rating_in,
		slack_in,
		discord_in,
		state_out,
		client_canary_in,
	);

	let server_task = future::try_join4(
		acceptance_task,
		future::try_join(chat_task, rating_task),
		future::try_join3(slack_task, discord_task, logrotate_task),
		close_task,
	)
	.map_ok(|((), ((), ()), ((), (), ()), ())| ());

	server_task.await
}

async fn accept_clients(
	settings: &Settings,
	login: sync::Arc<login::Server>,
	general_chat: mpsc::Sender<chat::Update>,
	ratings: mpsc::Sender<rating::Update>,
	slack_api: mpsc::Sender<slack_api::Post>,
	discord_api: mpsc::Sender<discord_api::Post>,
	server_state: watch::Receiver<State>,
	client_canary: mpsc::Sender<()>,
) -> Result<(), anyhow::Error>
{
	let server = settings
		.server
		.as_ref()
		.ok_or_else(|| anyhow!("missing 'server'"))?;
	let ipaddress = server.to_string();
	let binding: portal::Binding = portal::bind(settings).await?;
	let port = binding.port;
	let address: SocketAddr = format!("{}:{}", ipaddress, port).parse()?;
	let listener = TcpListener::bind(&address).await?;
	binding.confirm().await?;

	info!("Listening on {}:{}...", ipaddress, port);

	listen(
		listener,
		login,
		general_chat,
		ratings,
		slack_api,
		discord_api,
		server_state,
		client_canary,
	)
	.await;

	info!("Stopped listening.");

	binding.unbind().await
}

async fn listen(
	mut listener: TcpListener,
	login: sync::Arc<login::Server>,
	general_chat: mpsc::Sender<chat::Update>,
	ratings: mpsc::Sender<rating::Update>,
	slack_api: mpsc::Sender<slack_api::Post>,
	discord_api: mpsc::Sender<discord_api::Post>,
	server_state: watch::Receiver<State>,
	client_canary: mpsc::Sender<()>,
)
{
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
) -> Result<(), anyhow::Error>
{
	let handler = tokio::signal::unix::signal(SignalKind::terminate())?;
	let chat_closed = wait_for_canary(chat_canary).boxed();
	let mut signals = handler.take_until(chat_closed);

	let mut is_open = true;
	while let Some(()) = signals.next().await
	{
		if is_open
		{
			info!("Closing...");
			server_state.broadcast(State::Closing)?;
			is_open = false;
		}
		else
		{
			break;
		}
	}

	info!("Closed.");
	// If all clients are disconnected, no one will receive the broadcast.
	let _ = server_state.broadcast(State::Closed);

	wait_for_canary(client_canary).await;
	info!("All clients have disconnected.");
	Ok(())
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
