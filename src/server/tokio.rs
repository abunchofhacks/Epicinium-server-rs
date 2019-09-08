/* Server::Tokio */

use common::keycode::*;
use server::chat;
use server::client::*;
use server::killer;
use server::login;
use server::settings::*;

use std::error;
use std::io::Read;
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
	Disconnected,
}

pub fn run_server(settings: &Settings) -> Result<(), Box<dyn error::Error>>
{
	let server = settings.get_server()?;
	let port = settings.get_port()?;
	let address: SocketAddr = format!("{}:{}", server, port).parse()?;
	let listener = TcpListener::bind(&address)?;

	println!("Listening on {}:{}", server, port);

	let mut pem: Vec<u8> = Vec::new();
	let mut file = std::fs::File::open("keys/dummy_private.pem")?;
	file.read_to_end(&mut pem)?;
	let pkey: PrivateKey = openssl::pkey::PKey::private_key_from_pem(&pem)?;
	let privatekey = sync::Arc::new(pkey);

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

	let login_server = login::connect(settings)?;
	let login = sync::Arc::new(login_server);

	let (general_in, general_out) = mpsc::channel::<chat::Update>(10000);
	let chat_task = chat::start_task(general_out, closing_out, closed_in);

	let client_task = start_acceptance_task(
		listener, login, general_in, state_out, live_count, privatekey,
	);

	let server = client_task
		.join3(chat_task, killer_task)
		.map(|((), (), ())| ())
		.select(close_task)
		.map(|((), _other_future)| ())
		.map_err(|(error, _other_future)| error);

	tokio::run(server);

	Ok(())
}

fn start_acceptance_task(
	listener: TcpListener,
	login: sync::Arc<login::Server>,
	chat: mpsc::Sender<chat::Update>,
	server_state: watch::Receiver<State>,
	live_count: sync::Arc<atomic::AtomicUsize>,
	privatekey: sync::Arc<PrivateKey>,
) -> impl Future<Item = (), Error = ()> + Send
{
	let mut ticker: u64 = rand::random();

	listener
		.incoming()
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
				privatekey.clone(),
			)
			.map(|()| println!("Accepted client {}.", id))
		})
		.map_err(|e| {
			eprintln!("Incoming connection failed: {:?}", e);
		})
}

fn start_close_task(
	killcount: watch::Receiver<u8>,
	live_count: sync::Arc<atomic::AtomicUsize>,
	chat_closed: oneshot::Receiver<()>,
	chat_closing: oneshot::Sender<()>,
	server_state: watch::Sender<State>,
) -> impl Future<Item = (), Error = ()> + Send
{
	killcount
		.into_future()
		.map_err(|(error, _)| {
			eprintln!("Killcount error in close task: {:?}", error)
		})
		.and_then(move |(_current, killcount)| {
			// TODO broadcast State::CLOSING
			chat_closing
				.send(())
				.map_err(|error| {
					eprintln!("Chat send error in close task: {:?}", error)
				})
				.map(move |()| killcount)
		})
		.and_then(move |killcount| {
			killcount
				.filter(|&x| x >= 2)
				.map(|_x| State::Closed)
				.into_future()
				.map_err(|(error, _)| {
					eprintln!("Killcount error in close task: {:?}", error)
				})
				.and_then(|(x, _killcount)| {
					x.ok_or_else(|| eprintln!("No killcount in close task"))
				})
				.select(chat_closed.map(|()| State::Closed).map_err(|error| {
					eprintln!("Chat recv error in close task: {:?}", error)
				}))
				.map(|(state, _other_future)| state)
				.map_err(|((), _other_future)| ())
		})
		.map(move |state: State| {
			let timer = Interval::new_interval(Duration::from_millis(500))
				.skip_while(move |_instant| {
					Ok(live_count.load(atomic::Ordering::Relaxed) > 0)
				})
				.map(|_instant| State::Disconnected)
				.timeout(Duration::from_secs(5))
				.map_err(|error| {
					eprintln!("Timer error in close task: {:?}", error)
				});

			Ok(state).into_future().into_stream().chain(timer)
		})
		.into_stream()
		.flatten()
		.take_while(|&state| Ok(state != State::Disconnected))
		.for_each(|state| {
			server_state.broadcast(state).map_err(|error| {
				eprintln!("Broadcast error in close task: {:?}", error)
			})
		})
}
