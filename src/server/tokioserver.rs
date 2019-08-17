use server::settings::*;

use common::version::*;
use server::limits::*;
use server::message::*;

use std::error;
use std::io;
use std::net::SocketAddr;
use std::time;

use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;

use futures::{Future, Stream};

pub fn run_server(settings: &Settings) -> Result<(), Box<dyn error::Error>>
{
	let server = match settings.server()
	{
		Some(x) => x,
		None =>
		{
			return Err(Box::new(io::Error::new(
				io::ErrorKind::InvalidInput,
				"No ip mask (setting 'server') defined.",
			)));
		}
	};
	let port = match settings.port()
	{
		Some(x) => x,
		None =>
		{
			return Err(Box::new(io::Error::new(
				io::ErrorKind::InvalidInput,
				"No port (setting 'port') defined.",
			)));
		}
	};
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
			eprintln!("Incoming connection failed: {}", e);
		});

	// TODO

	tokio::run(server);

	Ok(())
}

fn accept_client(socket: TcpStream) -> io::Result<()>
{
	let task = futures::stream::unfold(socket, |socket| {
		let lengthbuffer = [0u8; 4];
		let future_length = tokio_io::io::read_exact(socket, lengthbuffer)
			.and_then(|(socket, lengthbuffer)| {
				let length = u32::from_le_bytes(lengthbuffer);

				let buffer = vec![0; length as usize];
				let future_data = tokio_io::io::read_exact(socket, buffer)
					.and_then(|(socket, buffer)| {
						let jsonstr = match String::from_utf8(buffer)
						{
							Ok(x) => x,
							Err(e) =>
							{
								return Err(io::Error::new(
									io::ErrorKind::InvalidData,
									e,
								));
							}
						};

						if jsonstr.len() < 200
						{
							println!("Received message: {}", jsonstr);
						}

						let message: Message = serde_json::from_str(&jsonstr)?;

						// Unfold excepts the value first and the state second.
						Ok((message, socket))
					});

				Ok(future_data)
			})
			.flatten();

		Some(future_length)
	})
	.for_each(|message: Message| {
		// TODO handle
		println!("Message: {:?}", message);
		futures::future::ok(())
	})
	.map_err(move |e| eprintln!("Error in client: {}", e));

	tokio::spawn(task);

	Ok(())
}
