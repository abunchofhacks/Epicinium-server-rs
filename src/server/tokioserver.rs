/* TokioServer */

use server::client::*;
use server::settings::*;

use std::error;
use std::net::SocketAddr;

use futures::{Future, Stream};
use tokio::net::TcpListener;

pub fn run_server(settings: &Settings) -> Result<(), Box<dyn error::Error>>
{
	let server = settings.get_server()?;
	let port = settings.get_port()?;
	let address: SocketAddr = format!("{}:{}", server, port).parse()?;
	let listener = TcpListener::bind(&address)?;

	println!("Listening on {}:{}", server, port);

	let server = listener
		.incoming()
		.for_each(|socket| {
			println!("Incoming connection: {:?}", socket);
			accept_client(socket)
		})
		.map_err(|e| {
			eprintln!("Incoming connection failed: {:?}", e);
		});

	// TODO signal handling

	tokio::run(server);

	Ok(())
}
