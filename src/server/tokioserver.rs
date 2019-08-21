/* TokioServer */

use server::client::*;
use server::settings::*;

use std::error;
use std::io;
use std::io::{ErrorKind, Read};
use std::net::SocketAddr;
use std::sync;

use futures::{Future, Stream};
use ring::signature::RsaKeyPair;
use tokio::net::TcpListener;

pub fn run_server(settings: &Settings) -> Result<(), Box<dyn error::Error>>
{
	let server = settings.get_server()?;
	let port = settings.get_port()?;
	let address: SocketAddr = format!("{}:{}", server, port).parse()?;
	let listener = TcpListener::bind(&address)?;

	println!("Listening on {}:{}", server, port);

	let mut buffer: Vec<u8> = Vec::new();
	let mut file = std::fs::File::open("keys/dummy_private.pem")?;
	file.read_to_end(&mut buffer)?;
	let pem = pem::parse(buffer).map_err(|e| {
		io::Error::new(ErrorKind::InvalidData, format!("pem::{}", e))
	})?;
	let pkey = RsaKeyPair::from_der(&pem.contents).map_err(|e| {
		io::Error::new(ErrorKind::InvalidData, format!("ring::{}", e))
	})?;
	let privatekey = sync::Arc::new(pkey);

	let server = start_acceptance_task(listener, privatekey);

	// TODO signal handling

	tokio::run(server);

	Ok(())
}

fn start_acceptance_task(
	listener: TcpListener,
	privatekey: sync::Arc<RsaKeyPair>,
) -> impl Future<Item = (), Error = ()>
{
	listener
		.incoming()
		.for_each(move |socket| {
			println!("Incoming connection: {:?}", socket);
			accept_client(socket, privatekey.clone())
		})
		.map_err(|e| {
			eprintln!("Incoming connection failed: {:?}", e);
		})
}
