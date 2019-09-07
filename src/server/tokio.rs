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

use futures::{Future, Stream};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio::sync::watch;

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

	let (killcount_in, killcount) = watch::channel(0u8);
	let killer_task = killer::start_task(killcount_in);

	let (keepalive, keepalive_out) = mpsc::channel::<()>(0);
	let close_task = start_close_task(keepalive.clone(), killcount.clone());
	let keepalive_task = start_keepalive_task(keepalive_out);

	let login_server = login::connect(settings)?;
	let login = sync::Arc::new(login_server);

	let (general_in, general_out) = mpsc::channel::<chat::Update>(10000);
	let chat_task = chat::start_task(general_out);

	let client_task = start_acceptance_task(
		listener, login, general_in, killcount, keepalive, privatekey,
	);

	let server = client_task
		.join4(chat_task, killer_task, close_task)
		.map(|((), (), (), ())| ())
		.select(keepalive_task)
		.map(|((), _other_future)| ())
		.map_err(|(error, _other_future)| error);

	tokio::run(server);

	Ok(())
}

fn start_acceptance_task(
	listener: TcpListener,
	login: sync::Arc<login::Server>,
	chat: mpsc::Sender<chat::Update>,
	killcount: watch::Receiver<u8>,
	keepalive: mpsc::Sender<()>,
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
				killcount.clone(),
				keepalive.clone(),
				privatekey.clone(),
			)
			.map(|()| println!("Accepted client {}.", id))
		})
		.map_err(|e| {
			eprintln!("Incoming connection failed: {:?}", e);
		})
}

fn start_close_task(
	keepalive: mpsc::Sender<()>,
	killcount: watch::Receiver<u8>,
) -> impl Future<Item = (), Error = ()> + Send
{
	killcount
		.skip(1)
		.into_future()
		.map_err(|(error, _)| eprintln!("Error in finish task: {:?}", error))
		.and_then(move |(_, _)| {
			// Drop the keepalive once this future is done.
			let _dropped = keepalive;
			Ok(())
		})
}

fn start_keepalive_task(
	keepalive: mpsc::Receiver<()>,
) -> impl Future<Item = (), Error = ()> + Send
{
	keepalive
		.for_each(|()| Ok(()))
		.map(|()| debug_assert!(false, "Keepalive should have 0 capacity"))
		.or_else(|_error| Ok(()))
}
