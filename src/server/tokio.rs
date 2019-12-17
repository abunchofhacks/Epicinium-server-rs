/* Server::Tokio */

use common::coredump::enable_coredumps;
use common::keycode::*;
use server::chat;
use server::client::*;
use server::killer;
use server::login;
use server::portal;
use server::settings::*;

use std::error;
use std::net::SocketAddr;
use std::sync;
use std::sync::atomic;
use std::time::Duration;

use futures::{Future, Stream};
use tokio::net::TcpListener;
use tokio::prelude::*;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::watch;
use tokio::timer::Interval;

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
	let live_count = sync::Arc::new(atomic::AtomicUsize::new(0));
	let (closing_in, closing_out) = oneshot::channel::<()>();
	let (closed_in, closed_out) = oneshot::channel::<()>();
	let close_task = start_close_task(
		killcount_out,
		live_count.clone(),
		closed_out,
		closing_in,
		state_in,
	);

	let (general_in, general_out) = mpsc::channel::<chat::Update>(10000);
	let chat_task = chat::start_task(general_out, closing_out, closed_in);

	let client_task = start_acceptance_task(
		host, binding, login, general_in, state_out, live_count,
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
	live_count: sync::Arc<atomic::AtomicUsize>,
) -> impl Future<Item = (), Error = ()> + Send
{
	let port = binding.port;
	let mut ticker: u64 = rand::random();

	start_listening(host, port)
		.map_err(|error| {
			eprintln!("Failed to start listening on port {}: {}", port, error)
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
				live_count.clone(),
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
	live_count: sync::Arc<atomic::AtomicUsize>,
	chat_closed: oneshot::Receiver<()>,
	chat_closing: oneshot::Sender<()>,
	server_state: watch::Sender<State>,
) -> impl Future<Item = (), Error = ()> + Send
{
	wait_for_kill(killcount.clone(), chat_closing, server_state)
		.and_then(move |state| wait_for_close(killcount, chat_closed, state))
		.and_then(move |()| wait_for_disconnect(live_count))
}

fn wait_for_kill(
	killcount: watch::Receiver<u8>,
	chat_closing: oneshot::Sender<()>,
	mut server_state: watch::Sender<State>,
) -> impl Future<Item = watch::Sender<State>, Error = ()> + Send
{
	killcount
		.skip_while(|&x| Ok(x < 1))
		.into_future()
		.map_err(|(error, _killcount)| {
			eprintln!("Killcount error in close task: {:?}", error)
		})
		.and_then(move |(_current, _killcount)| {
			chat_closing.send(()).map_err(|error| {
				eprintln!("Chat send error in close task: {:?}", error)
			})
		})
		.inspect(|()| println!("Closing..."))
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
	chat_closed: oneshot::Receiver<()>,
	mut server_state: watch::Sender<State>,
) -> impl Future<Item = (), Error = ()> + Send
{
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
		.select(chat_closed.map_err(|error| {
			eprintln!("Chat recv error in close task: {:?}", error)
		}))
		.map(|((), _other_future)| ())
		.map_err(|((), _other_future)| ())
		.and_then(move |()| {
			// There might be a Future waiting for this, or there might not be.
			server_state.broadcast(State::Closed).or(Ok(()))
		})
}

fn wait_for_disconnect(
	live_count: sync::Arc<atomic::AtomicUsize>,
) -> impl Future<Item = (), Error = ()> + Send
{
	Interval::new_interval(Duration::from_millis(25))
		.skip_while(move |_instant| {
			Ok(live_count.load(atomic::Ordering::Relaxed) > 0)
		})
		.into_future()
		.map(|(_instant, _interval)| println!("All clients have disconnected."))
		.map_err(|(error, _interval)| error)
		.timeout(Duration::from_secs(5))
		.map_err(|error| eprintln!("Timer error in close task: {:?}", error))
}

fn increase_sockets() -> std::io::Result<()>
{
	const MAX_SOCKETS: rlimit::rlim = 16384;
	rlimit::Resource::NOFILE.set(MAX_SOCKETS, MAX_SOCKETS)
}
