/* TokioServer */

use common::keycode::*;
use server::chat::*;
use server::client::*;
use server::loginserver::*;
use server::message::*;
use server::settings::*;

use std::error;
use std::io::Read;
use std::net::SocketAddr;
use std::sync;

use futures::{Future, Stream};
use tokio::net::TcpListener;
use tokio::sync::mpsc;

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

	let login_server = LoginServer::connect(settings)?;
	let login = sync::Arc::new(login_server);

	let (general_in, general_out) = mpsc::channel::<Message>(10000);
	let chat_task = start_chat_task(general_out);
	let chat = general_in;

	let client_task = start_acceptance_task(listener, login, chat, privatekey);

	let server = client_task.join(chat_task).map(|((), ())| ());

	// TODO signal handling

	tokio::run(server);

	Ok(())
}

fn start_acceptance_task(
	listener: TcpListener,
	login: sync::Arc<LoginServer>,
	chat: mpsc::Sender<Message>,
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
				privatekey.clone(),
			)
			.map(|()| println!("Accepted client {}.", id))
		})
		.map_err(|e| {
			eprintln!("Incoming connection failed: {:?}", e);
		})
}
