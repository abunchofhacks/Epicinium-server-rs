/* Server::Tokio */

use crate::common::coredump::enable_coredumps;
use crate::common::keycode::*;
use crate::logic::ruleset;
use crate::server::chat;
use crate::server::client;
use crate::server::killer;
use crate::server::login;
use crate::server::portal;
use crate::server::rating;
use crate::server::settings::*;

use std::error;
use std::net::SocketAddr;
use std::sync;
use std::sync::atomic;

use futures::future;
use futures::select;
use futures::FutureExt;
use futures::TryFutureExt;

use tokio::net::TcpListener;
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
) -> Result<(), Box<dyn error::Error>>
{
	enable_coredumps()?;
	increase_sockets()?;

	ruleset::initialize_collection()?;

	let login_server = login::connect(settings)?;
	let login = sync::Arc::new(login_server);

	let (rating_in, rating_out) = mpsc::channel::<rating::Update>(10000);
	let rating_task = rating::run(settings, rating_out);

	let (killcount_in, killcount_out) = watch::channel(0u8);
	let killer_task = killer::run(killcount_in).map_err(|e| e.into());

	let (state_in, state_out) = watch::channel(State::Open);
	let (client_canary_in, client_canary_out) = mpsc::channel::<()>(1);
	let (general_canary_in, general_canary_out) = mpsc::channel::<()>(1);
	let close_task = wait_for_close(
		killcount_out,
		general_canary_out,
		client_canary_out,
		state_in,
	);

	let (general_in, general_out) = mpsc::channel::<chat::Update>(10000);
	let chat_task = chat::run(general_out, general_canary_in).map(|()| Ok(()));

	let acceptance_task = accept_clients(
		settings,
		login,
		general_in,
		rating_in,
		state_out,
		client_canary_in,
	);

	let server_task = future::try_join5(
		acceptance_task,
		chat_task,
		rating_task,
		killer_task,
		close_task,
	)
	.map_ok(|((), (), (), (), ())| ());

	server_task.await
}

async fn accept_clients(
	settings: &Settings,
	login: sync::Arc<login::Server>,
	general_chat: mpsc::Sender<chat::Update>,
	ratings: mpsc::Sender<rating::Update>,
	server_state: watch::Receiver<State>,
	client_canary: mpsc::Sender<()>,
) -> Result<(), Box<dyn error::Error>>
{
	let server = settings.get_server()?;
	let ipaddress = server.to_string();
	let binding: portal::Binding = portal::bind(settings).await?;
	let port = binding.port;
	let address: SocketAddr = format!("{}:{}", ipaddress, port).parse()?;
	let listener = TcpListener::bind(&address).await?;
	binding.confirm().await?;

	println!("Listening on {}:{}", ipaddress, port);

	listen(
		listener,
		login,
		general_chat,
		ratings,
		server_state,
		client_canary,
	)
	.await;

	println!("Stopped listening.");

	binding.unbind().await
}

async fn listen(
	mut listener: TcpListener,
	login: sync::Arc<login::Server>,
	general_chat: mpsc::Sender<chat::Update>,
	ratings: mpsc::Sender<rating::Update>,
	mut server_state: watch::Receiver<State>,
	client_canary: mpsc::Sender<()>,
)
{
	let mut ticker: u64 = rand::random();
	let lobbyticker = sync::Arc::new(atomic::AtomicU64::new(rand::random()));

	loop
	{
		let socket = select! {
			listened = listener.accept().fuse() => match listened
			{
				Ok((socket, _addr)) =>
				{
					socket
				},
				Err(error) =>
				{
					println!("Failed to connect client: {:?}", error);
					continue
				}
			},
			state = server_state.recv().fuse() => match state
			{
				Some(State::Open) => continue,
				Some(State::Closing) => break,
				Some(State::Closed) => break,
				None => break,
			}
		};

		println!("Accepting incoming connection: {:?}", socket);

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
			server_state.clone(),
			client_canary.clone(),
			lobbyticker.clone(),
		);

		println!("Accepted client {}.", id);
	}
}

async fn wait_for_close(
	mut killcount: watch::Receiver<u8>,
	chat_canary: mpsc::Receiver<()>,
	client_canary: mpsc::Receiver<()>,
	server_state: watch::Sender<State>,
) -> Result<(), Box<dyn error::Error>>
{
	wait_for_kill(&mut killcount, 1).await;
	println!("Closing...");
	server_state.broadcast(State::Closing)?;

	wait_for_canary_or_kill(chat_canary, &mut killcount, 2).await;
	println!("Closed.");
	// If all clients are disconnected, no one will receive the broadcast.
	let _ = server_state.broadcast(State::Closed);

	wait_for_canary(client_canary).await;
	println!("All clients have disconnected.");
	Ok(())
}

async fn wait_for_kill(killcount: &mut watch::Receiver<u8>, threshold: u8)
{
	while let Some(x) = killcount.recv().await
	{
		if x >= threshold
		{
			break;
		}
	}
}

async fn wait_for_canary(mut canary: mpsc::Receiver<()>)
{
	while let Some(()) = canary.recv().await
	{
		// Nothing to do.
	}
}

async fn wait_for_canary_or_kill(
	canary: mpsc::Receiver<()>,
	killcount: &mut watch::Receiver<u8>,
	threshold: u8,
)
{
	select! {
		() = wait_for_canary(canary).fuse() => (),
		() = wait_for_kill(killcount, threshold).fuse() => (),
	}
}

fn increase_sockets() -> std::io::Result<()>
{
	const MAX_SOCKETS: rlimit::rlim = 16384;
	rlimit::Resource::NOFILE.set(MAX_SOCKETS, MAX_SOCKETS)
}
