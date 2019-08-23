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

	let (login_req_in, login_req_out) = mpsc::channel::<LoginRequest>(1000);
	let login_task = start_login_task(login_req_out);
	let client_task = start_acceptance_task(listener, login_req_in, privatekey);

	let server = login_task.join(client_task).map(|((), ())| ());

	// TODO signal handling

	tokio::run(server);

	Ok(())
}

fn start_acceptance_task(
	listener: TcpListener,
	login: mpsc::Sender<LoginRequest>,
	privatekey: sync::Arc<PrivateKey>,
) -> impl Future<Item = (), Error = ()>
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
