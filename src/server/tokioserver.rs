/* TokioServer */

use server::client::*;
use server::loginserver::*;
use server::settings::*;

use std::error;
use std::io::Read;
use std::net::SocketAddr;
use std::sync;

use futures::{Future, Stream};
use tokio::net::TcpListener;

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

	let server = start_acceptance_task(listener, login, privatekey);

	// TODO signal handling

	tokio::run(server);

	Ok(())
}

fn start_acceptance_task(
	listener: TcpListener,
	login: sync::Arc<LoginServer>,
	privatekey: sync::Arc<PrivateKey>,
) -> impl Future<Item = (), Error = ()> + Send
{
	listener
		.incoming()
		.for_each(move |socket| {
			println!("Incoming connection: {:?}", socket);
			accept_client(socket, login.clone(), privatekey.clone())
		})
		.map_err(|e| {
			eprintln!("Incoming connection failed: {:?}", e);
		})
}
