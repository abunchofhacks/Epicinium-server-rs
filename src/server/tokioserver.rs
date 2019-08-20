/* TokioServer */

use server::client::*;
use server::notice::*;
use server::settings::*;

use std::error;
use std::net::SocketAddr;

use futures::future;
use futures::{Future, Stream};
use tokio::net::TcpListener;

pub fn run_server(settings: &Settings) -> Result<(), Box<dyn error::Error>>
{
	let server = settings.get_server()?;
	let port = settings.get_port()?;
	let address: SocketAddr = format!("{}:{}", server, port).parse()?;
	let listener = TcpListener::bind(&address)?;

	println!("Listening on {}:{}", server, port);

	let server = future::ok(())
		.and_then(|()| run_notice_service())
		.map_err(|e| eprintln!("Failed to start notice service: {:?}", e))
		.and_then(|notice_service| {
			start_acceptance_task(listener, notice_service)
		});

	// TODO signal handling

	tokio::run(server);

	Ok(())
}

fn start_acceptance_task(
	listener: TcpListener,
	notice_service: NoticeService,
) -> impl Future<Item = (), Error = ()>
{
	listener
		.incoming()
		.for_each(move |socket| {
			println!("Incoming connection: {:?}", socket);
			accept_client(socket, notice_service.clone())
		})
		.map_err(|e| {
			eprintln!("Incoming connection failed: {:?}", e);
		})
}
