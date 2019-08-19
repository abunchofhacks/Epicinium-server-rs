use server::settings::*;

use common::version::*;
use server::limits::*;
use server::message::*;

use std::error;
use std::io;
use std::io::ErrorKind;
use std::net::SocketAddr;
use std::sync;
use std::sync::atomic;
use std::time::{Duration, Instant};

use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::timer::Interval;

use futures::future;
use futures::future::Either;
use futures::stream;
use futures::{Future, Stream};

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

	// TODO

	tokio::run(server);

	Ok(())
}

fn accept_client(socket: TcpStream) -> io::Result<()>
{
	let (mut sendbuffer_in, sendbuffer_out) = mpsc::channel::<Message>(1000);
	let mut pingbuffer = sendbuffer_in.clone();
	let mut pulsebuffer = sendbuffer_in.clone();
	let (mut timebuffer_in, timebuffer_out) = watch::channel(());
	let (mut pongbuffer_in, pongbuffer_out) = watch::channel(());
	let (mut supports_empty_in, supports_empty_out) = watch::channel(false);
	let versioned = sync::Arc::new(atomic::AtomicBool::new(false));
	let (reader, writer) = socket.split();

	let receive_versioned = versioned.clone();
	let receive_task = stream::unfold(reader, move |socket| {
		let versioned: bool = receive_versioned.load(atomic::Ordering::Relaxed);
		let lengthbuffer = [0u8; 4];
		let future_length = tokio_io::io::read_exact(socket, lengthbuffer)
			.and_then(move |(socket, lengthbuffer)| {
				let length = u32::from_le_bytes(lengthbuffer);

				if length == 0
				{
					println!("Received pulse.");

					// Unfold expects the value first and the state second.
					let result = Ok((Message::Pulse, socket));
					let future_data = future::result(result);
					return Ok(Either::A(future_data));
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
						ErrorKind::InvalidInput,
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
						ErrorKind::InvalidInput,
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
									ErrorKind::InvalidData,
									e,
								));
							}
						};

						if jsonstr.len() < 200
						{
							println!("Received message: {}", jsonstr);
						}

						let message: Message = serde_json::from_str(&jsonstr)?;

						// Unfold expects the value first and the state second.
						Ok((message, socket))
					});

				Ok(Either::B(future_data))
			})
			.flatten();

		Some(future_length)
	})
	.for_each(move |message: Message| {
		handle_message(
			message,
			&mut sendbuffer_in,
			&mut timebuffer_in,
			&mut pongbuffer_in,
			&versioned,
			&mut supports_empty_in,
		)
		.map_err(|e| io::Error::new(ErrorKind::ConnectionReset, e))
	});

	let send_task = sendbuffer_out
		.map_err(|e| io::Error::new(ErrorKind::ConnectionReset, e))
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
				return Either::A(future);
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
			return Either::B(future);
		})
		.map(|_socket| ());

	// TODO variable ping_tolerance
	let ping_tolerance = Duration::from_secs(120);
	let ping_task = timebuffer_out
		.timeout(Duration::from_secs(5))
		.filter(|_| false)
		.or_else(|e| {
			if e.is_elapsed()
			{
				Ok(())
			}
			else if e.is_timer()
			{
				Err(PingTaskError::Timer {
					error: e.into_timer().unwrap(),
				})
			}
			else
			{
				Err(PingTaskError::Recv {
					error: e.into_inner().unwrap(),
				})
			}
		})
		.and_then(move |()| {
			let pingtime = Instant::now();
			pingbuffer
				.try_send(Message::Ping)
				.map(move |()| pingtime)
				.map_err(|error| PingTaskError::Send { error })
		})
		.and_then(move |pingtime| {
			let pongbuffer = pongbuffer_out.clone();
			pongbuffer
				.skip(1)
				.into_future()
				.map_err(|(error, _)| PingTaskError::Recv { error })
				.and_then(move |(x, _)| x.ok_or(PingTaskError::NoPong))
				.map(move |()| pingtime)
		})
		.timeout(ping_tolerance)
		.and_then(move |pingtime| {
			println!("Client has {}ms ping.", pingtime.elapsed().as_millis());
			Ok(())
		})
		.or_else(|e| {
			if e.is_elapsed()
			{
				Err(PingTaskError::NoPong)
			}
			else if e.is_timer()
			{
				Err(PingTaskError::Timer {
					error: e.into_timer().unwrap(),
				})
			}
			else
			{
				Err(e.into_inner().unwrap())
			}
		})
		.or_else(|pe| match pe
		{
			PingTaskError::Timer { error } =>
			{
				eprintln!("Timer error in client pulse_task: {:?}", error);
				Ok(())
			}
			PingTaskError::Send { error } =>
			{
				Err(io::Error::new(ErrorKind::ConnectionReset, error))
			}
			PingTaskError::Recv { error } =>
			{
				Err(io::Error::new(ErrorKind::ConnectionReset, error))
			}
			PingTaskError::NoPong =>
			{
				Err(io::Error::new(ErrorKind::ConnectionReset, "no pong"))
			}
		})
		.for_each(|()| Ok(()));

	// TODO use a oneshot channel to confirm supports_empty_pulses
	let _ = supports_empty_out;
	let pulse_task = Interval::new_interval(Duration::from_secs(4))
		.or_else(|error| Err(PulseTaskError::Timer { error }))
		.for_each(move |_| {
			pulsebuffer
				.try_send(Message::Pulse)
				.or_else(|error| Err(PulseTaskError::Send { error }))
		})
		.or_else(|pe| match pe
		{
			PulseTaskError::Timer { error } =>
			{
				eprintln!("Timer error in client pulse_task: {:?}", error);
				Ok(())
			}
			PulseTaskError::Send { error } =>
			{
				Err(io::Error::new(ErrorKind::ConnectionReset, error))
			}
		});

	let task = receive_task
		.join(send_task)
		.map(|_| ())
		.select(ping_task)
		.map(|_| ())
		.map_err(|(e, _)| e)
		.select(pulse_task)
		.map(|_| ())
		.map_err(|(e, _)| e)
		.map_err(|e| eprintln!("Error in client: {:?}", e));

	tokio::spawn(task);

	Ok(())
}

#[derive(Debug)]
enum PingTaskError
{
	Timer
	{
		error: tokio::timer::Error,
	},
	Send
	{
		error: mpsc::error::TrySendError<Message>,
	},
	Recv
	{
		error: watch::error::RecvError,
	},
	NoPong,
}

enum PulseTaskError
{
	Timer
	{
		error: tokio::timer::Error
	},
	Send
	{
		error: mpsc::error::TrySendError<Message>,
	},
}

fn handle_message(
	message: Message,
	sendbuffer: &mut mpsc::Sender<Message>,
	last_receive_time: &mut watch::Sender<()>,
	pong_receive_time: &mut watch::Sender<()>,
	is_versioned: &sync::Arc<atomic::AtomicBool>,
	supports_empty_pulses: &mut watch::Sender<bool>,
) -> Result<(), mpsc::error::TrySendError<Message>>
{
	// There might be a Future tracking when we last received a message,
	// or there might not be.
	let _ = last_receive_time.broadcast(());

	match message
	{
		Message::Pulse =>
		{
			// The client just let us know that it is still breathing.
		}
		Message::Ping =>
		{
			// Pings must always be responded with pongs.
			sendbuffer.try_send(Message::Pong)?;
		}
		Message::Pong =>
		{
			// There might be a Future waiting for this pong message,
			// or there might not be.
			let _ = pong_receive_time.broadcast(());
		}
		Message::Version { version, metadata } =>
		{
			greet_client(
				sendbuffer,
				is_versioned,
				supports_empty_pulses,
				version,
				metadata,
			)?;
		}
		_ =>
		{
			// TODO handle
			println!("Unhandled message: {:?}", message);
		}
	}

	Ok(())
}

fn greet_client(
	sendbuffer: &mut mpsc::Sender<Message>,
	is_versioned: &sync::Arc<atomic::AtomicBool>,
	supports_empty_pulses: &mut watch::Sender<bool>,
	version: Version,
	metadata: Option<PlatformMetadata>,
) -> Result<(), mpsc::error::TrySendError<Message>>
{
	is_versioned.store(true, atomic::Ordering::Relaxed);
	println!("Client has version {}", version.to_string());

	if let Some(PlatformMetadata {
		platform,
		patchmode,
	}) = metadata
	{
		println!("Client has platform {:?}", platform);
		println!("Client has patchmode {:?}", patchmode);
	}

	let myversion = Version::current();
	let response = Message::Version {
		version: myversion,
		metadata: None,
	};
	sendbuffer.try_send(response)?;

	if version >= Version::exact(0, 31, 1, 0)
	{
		// There might be a Future waiting for this, or there might not be.
		let _ = supports_empty_pulses.broadcast(true);
	}

	// TODO

	Ok(())
}
