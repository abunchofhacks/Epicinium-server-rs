use server::settings::*;

use common::version::*;
use server::limits::*;
use server::message::*;

use std::error;
use std::io;
use std::net::SocketAddr;
use std::sync;
use std::sync::atomic;
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
			eprintln!("Incoming connection failed: {:?}", e);
		});

	// TODO

	tokio::run(server);

	Ok(())
}

fn accept_client(socket: TcpStream) -> io::Result<()>
{
	let maxsendbuffersize = 10000;
	let (mut sendbuffer_in, sendbuffer_out) =
		tokio::sync::mpsc::channel::<Message>(maxsendbuffersize);
	let versioned = sync::Arc::new(atomic::AtomicBool::new(false));
	let (reader, writer) = socket.split();

	let receive_versioned = versioned.clone();
	let receive_task = futures::stream::unfold(reader, move |socket| {
		let versioned: bool = receive_versioned.load(atomic::Ordering::Relaxed);
		let lengthbuffer = [0u8; 4];
		let future_length = tokio_io::io::read_exact(socket, lengthbuffer)
			.and_then(move |(socket, lengthbuffer)| {
				let length = u32::from_le_bytes(lengthbuffer);

				if length == 0
				{
					println!("Received pulse.");
					let result = Ok((Message::Pulse, socket));
					let future_data = futures::future::result(result);
					return Ok(futures::future::Either::A(future_data));
				}
				else if !versioned
					&& length as usize >= MESSAGE_SIZE_UNVERSIONED_LIMIT
				{
					println!(
						"Unversioned client tried to send \
						 very large message of length {}, \
						 which is more than MESSAGE_SIZE_UNVERSIONED_LIMIT",
						length
					);
					return Err(io::Error::new(
						io::ErrorKind::InvalidInput,
						"Message too large".to_string(),
					));
				}
				else if length as usize >= MESSAGE_SIZE_LIMIT
				{
					println!(
						"Refusing to receive very large message of length {}, \
						 which is more than MESSAGE_SIZE_LIMIT",
						length
					);
					return Err(io::Error::new(
						io::ErrorKind::InvalidInput,
						"Message too large".to_string(),
					));
				}
				else if length as usize >= MESSAGE_SIZE_WARNING_LIMIT
				{
					println!(
						"Receiving very large message of length {}",
						length
					);
				}

				println!("Receiving message of length {}...", length);

				let buffer = vec![0; length as usize];
				let future_data = tokio_io::io::read_exact(socket, buffer)
					.and_then(|(socket, buffer)| {
						println!(
							"Received message of length {}.",
							buffer.len()
						);

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

				Ok(futures::future::Either::B(future_data))
			})
			.flatten();

		Some(future_length)
	})
	.for_each(move |message: Message| {
		println!("Message: {:?}", message);

		match message
		{
			Message::Ping =>
			{
				// Pings must always be responded with pongs.
				match sendbuffer_in.try_send(Message::Pong)
				{
					Ok(()) => Ok(()),
					Err(e) =>
					{
						Err(io::Error::new(io::ErrorKind::ConnectionReset, e))
					}
				}
			}
			Message::Version { .. } =>
			{
				// TODO handle
				versioned.store(true, atomic::Ordering::Relaxed);

				Ok(())
			}
			_ => Ok(()),
		}
	});

	let send_task = sendbuffer_out
		.map_err(|e| io::Error::new(io::ErrorKind::ConnectionReset, e))
		.fold(writer, |socket, message| {
			if let Message::Pulse = message
			{
				println!("Sending pulse...");

				let zeroes = [0u8; 4];
				let buffer = zeroes.to_vec();

				let future = tokio_io::io::write_all(socket, buffer).map(
					|(socket, _)| {
						println!("Sent pulse.");
						socket
					},
				);
				return futures::future::Either::A(future);
			}

			let jsonstr = match serde_json::to_string(&message)
			{
				Ok(data) => data,
				Err(e) =>
				{
					panic!("Invalid message: {:?}", e);
				}
			};

			if jsonstr.len() >= MESSAGE_SIZE_LIMIT
			{
				panic!(
					"Cannot send message of length {}, \
					 which is larger than MESSAGE_SIZE_LIMIT.",
					jsonstr.len()
				);
			}

			let length = jsonstr.len() as u32;

			if length as usize >= MESSAGE_SIZE_WARNING_LIMIT
			{
				println!("Sending very large message of length {}", length);
			}

			println!("Sending message of length {}...", length);

			let mut buffer = length.to_le_bytes().to_vec();
			buffer.append(&mut jsonstr.into_bytes());

			let future = tokio_io::io::write_all(socket, buffer).map(
				move |(socket, _)| {
					println!("Sent message of length {}.", length);
					socket
				},
			);
			return futures::future::Either::B(future);
		})
		.map(|_socket| ());

	let task = receive_task
		.join(send_task)
		.map(|_| ())
		.map_err(move |e| eprintln!("Error in client: {:?}", e));

	tokio::spawn(task);

	Ok(())
}
