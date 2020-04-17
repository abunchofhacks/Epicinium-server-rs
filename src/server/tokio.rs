/* Server::Tokio */

use crate::common::coredump::enable_coredumps;
use crate::common::keycode::*;
use crate::server::chat;
use crate::server::client::*;
use crate::server::killer;
use crate::server::login;
use crate::server::portal;
use crate::server::settings::*;

use std::error;
use std::net::SocketAddr;
use std::sync;
use std::sync::atomic;
use std::time::Duration;

use futures::{Future, Stream};
use tokio::net::TcpListener;
use tokio::prelude::*;
use tokio::sync::mpsc;
use tokio::sync::watch;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum State
{
	Open,
	Closing,
	Closed,
}

pub fn run_server(settings: &Settings) -> Result<(), Box<dyn error::Error>>
{
	enable_coredumps()?;
	increase_sockets()?;

	let server = settings.get_server()?;
	let ipaddress = server.to_string();

	let login_server = login::connect(settings)?;
	let login = sync::Arc::new(login_server);

	let server = portal::bind(settings)
		.and_then(|binding| start_server_task(ipaddress, binding, login));

	tokio::run(server);
	Ok(())
}

fn start_listening(
	ipaddress: String,
	port: u16,
) -> Result<TcpListener, Box<dyn error::Error>>
{
	let address: SocketAddr = format!("{}:{}", ipaddress, port).parse()?;
	let listener = TcpListener::bind(&address)?;

	println!("Listening on {}:{}", ipaddress, port);

	Ok(listener)
}

fn start_server_task(
	host: String,
	binding: portal::Binding,
	login: sync::Arc<login::Server>,
) -> impl Future<Item = (), Error = ()> + Send
{
	let (killcount_in, killcount_out) = watch::channel(0u8);
	let killer_task = killer::start_task(killcount_in);

	let (state_in, state_out) = watch::channel(State::Open);
	let (client_canary_in, client_canary_out) = mpsc::channel::<()>(1);
	let (general_canary_in, general_canary_out) = mpsc::channel::<()>(1);
	let close_task = start_close_task(
		killcount_out,
		general_canary_out,
		client_canary_out,
		state_in,
	);

	let (general_in, general_out) = mpsc::channel::<chat::Update>(10000);
	let chat_task = chat::start_task(general_out, general_canary_in);

	let client_task = start_acceptance_task(
		host,
		binding,
		login,
		general_in,
		state_out,
		client_canary_in,
	);

	client_task
		.join3(chat_task, killer_task)
		.map(|((), (), ())| ())
		.select(close_task)
		.map(|((), _other_future)| ())
		.map_err(|(error, _other_future)| error)
}

fn start_acceptance_task(
	host: String,
	binding: portal::Binding,
	login: sync::Arc<login::Server>,
	chat: mpsc::Sender<chat::Update>,
	server_state: watch::Receiver<State>,
	client_canary: mpsc::Sender<()>,
) -> impl Future<Item = (), Error = ()> + Send
{
	let port = binding.port;
	let mut ticker: u64 = rand::random();
	let lobbyticker = sync::Arc::new(atomic::AtomicU64::new(rand::random()));

	start_listening(host, port)
		.map_err(|error| {
			eprintln!("Failed to start listening on port {}: {:?}", port, error)
		})
		.map(|listener| binding.confirm().map(|()| listener))
		.into_future()
		.flatten()
		.map(|listener| {
			listener
				.incoming()
				.map_err(|e| {
					eprintln!("Incoming connection failed: {:?}", e);
				})
				.map(|socket| Some(socket))
		})
		.flatten_stream()
		.select(
			server_state
				.clone()
				.filter_map(|state| match state
				{
					State::Open => None,
					State::Closing => Some(None),
					State::Closed => Some(None),
				})
				.map_err(|e| eprintln!("State error while listening: {:?}", e)),
		)
		.take_while(|x| future::ok(x.is_some()))
		.filter_map(|x| x)
		.for_each(move |socket| {
			println!("Incoming connection: {:?}", socket);

			let serial = ticker;
			ticker += 1;
			let key: u16 = rand::random();
			let id = keycode(key, serial);

			accept_client(
				socket,
				id,
				login.clone(),
				chat.clone(),
				server_state.clone(),
				client_canary.clone(),
				lobbyticker.clone(),
			)
			.map_err(|e| {
				eprintln!("Accepting incoming connection failed: {:?}", e);
			})
			.map(|()| println!("Accepted client {}.", id))
		})
		.map(|()| println!("Stopped listening."))
		.then(move |result| binding.unbind().and_then(move |()| result))
}

fn start_close_task(
	killcount: watch::Receiver<u8>,
	chat_canary: mpsc::Receiver<()>,
	client_canary: mpsc::Receiver<()>,
	server_state: watch::Sender<State>,
) -> impl Future<Item = (), Error = ()> + Send
{
	wait_for_kill(killcount.clone(), server_state)
		.and_then(move |state| wait_for_close(killcount, chat_canary, state))
		.and_then(move |()| wait_for_disconnect(client_canary))
}

fn wait_for_kill(
	killcount: watch::Receiver<u8>,
	mut server_state: watch::Sender<State>,
) -> impl Future<Item = watch::Sender<State>, Error = ()> + Send
{
	killcount
		.skip_while(|&x| Ok(x < 1))
		.into_future()
		.map_err(|(error, _killcount)| {
			eprintln!("Killcount error in close task: {:?}", error)
		})
		.map(|(_current, _killcount)| println!("Closing..."))
		.and_then(move |()| {
			server_state
				.broadcast(State::Closing)
				.map_err(|error| {
					eprintln!("Broadcast error in close task: {:?}", error)
				})
				.map(|()| server_state)
		})
}

fn wait_for_close(
	killcount: watch::Receiver<u8>,
	chat_canary: mpsc::Receiver<()>,
	mut server_state: watch::Sender<State>,
) -> impl Future<Item = (), Error = ()> + Send
{
	let wait_future = chat_canary
		.for_each(|()| Ok(()))
		.map(|()| println!("All clients have left chat, closing now..."))
		.map_err(|error| {
			eprintln!("Canary error in wait_for_close: {:?}", error)
		});

	killcount
		.skip_while(|&x| Ok(x < 2))
		.into_future()
		.map_err(|(error, _)| {
			eprintln!("Killcount error in close task: {:?}", error)
		})
		.and_then(|(x, _killcount)| {
			x.ok_or_else(|| eprintln!("No killcount in close task"))
		})
		.map(|_x| println!("Closing forcefully..."))
		.select(wait_future)
		.map(|((), _other_future)| ())
		.map_err(|((), _other_future)| ())
		.and_then(move |()| {
			// There might be a Future waiting for this, or there might not be.
			server_state.broadcast(State::Closed).or(Ok(()))
		})
		.map(|()| println!("Closed."))
}

fn wait_for_disconnect(
	client_canary: mpsc::Receiver<()>,
) -> impl Future<Item = (), Error = ()> + Send
{
	client_canary
		.for_each(|()| Ok(()))
		.map(|()| println!("All clients have disconnected."))
		.map_err(|error| {
			eprintln!("Canary error in wait_for_close: {:?}", error)
		})
		.timeout(Duration::from_secs(5))
		.map_err(|error| {
			eprintln!("Timer error in wait_for_disconnect: {:?}", error)
		})
}

fn increase_sockets() -> std::io::Result<()>
{
	const MAX_SOCKETS: rlimit::rlim = 16384;
	rlimit::Resource::NOFILE.set(MAX_SOCKETS, MAX_SOCKETS)
}
