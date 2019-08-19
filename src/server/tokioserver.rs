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
	let mut sendbuffer_ping = sendbuffer_in.clone();
	let mut sendbuffer_pulse = sendbuffer_in.clone();
	let (mut pingbuffer_in, pingbuffer_out) = watch::channel(());
	let (mut timebuffer_in, timebuffer_out) = watch::channel(());
	let (mut pongbuffer_in, pongbuffer_out) = watch::channel(());
	let (mut quitbuffer_in, quitbuffer_out) = watch::channel(());
	let (mut supports_empty_in, supports_empty_out) = mpsc::channel::<bool>(1);
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
	.map_err(|error| ReceiveTaskError::Recv { error })
	.for_each(move |message: Message| {
		handle_message(
			message,
			&mut sendbuffer_in,
			&mut pingbuffer_in,
			&mut timebuffer_in,
			&mut pongbuffer_in,
			&versioned,
			&mut supports_empty_in,
			&mut quitbuffer_in,
		)
	})
	.or_else(|re| match re
	{
		ReceiveTaskError::Quit => Ok(()),
		ReceiveTaskError::Goodbye => Ok(()),
		ReceiveTaskError::Send { error } =>
		{
			println!("Send error in receive_task: {:?}", error);
			Err(io::Error::new(ErrorKind::ConnectionReset, error))
		}
		ReceiveTaskError::Recv { error } =>
		{
			println!("Recv error in receive_task: {:?}", error);
			Err(error)
		}
	})
	.map(|()| println!("Stopped receiving."));

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
		.map(|_socket| ())
		.map_err(|error| {
			println!("Error in send_task: {:?}", error);
			error
		})
		.map(|()| println!("Stopped sending."));

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
		.select(
			pingbuffer_out
				.skip(1)
				.map_err(|error| PingTaskError::Recv { error }),
		)
		.and_then(move |()| {
			let pingtime = Instant::now();
			sendbuffer_ping
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
		.map_err(|e| {
			if e.is_elapsed()
			{
				PingTaskError::NoPong
			}
			else if e.is_timer()
			{
				PingTaskError::Timer {
					error: e.into_timer().unwrap(),
				}
			}
			else
			{
				e.into_inner().unwrap()
			}
		})
		.or_else(|pe| match pe
		{
			PingTaskError::Timer { error } =>
			{
				eprintln!("Timer error in client ping_task: {:?}", error);
				Ok(())
			}
			PingTaskError::Send { error } =>
			{
				println!("Send error in ping_task: {:?}", error);
				Err(io::Error::new(ErrorKind::ConnectionReset, error))
			}
			PingTaskError::Recv { error } =>
			{
				println!("Recv error in ping_task: {:?}", error);
				Err(io::Error::new(ErrorKind::ConnectionReset, error))
			}
			PingTaskError::NoPong =>
			{
				println!("Client failed to respond to ping.");
				Err(io::Error::new(ErrorKind::ConnectionReset, "no pong"))
			}
		})
		.for_each(|()| Ok(()));

	let pulse_task = supports_empty_out
		.into_future()
		.map_err(|(error, _)| PulseTaskError::Recv { error })
		.and_then(move |(supported, _)| match supported
		{
			Some(true) => Ok(Instant::now()),
			Some(false) => Err(PulseTaskError::Unsupported),
			None => Err(PulseTaskError::Dropped),
		})
		.into_stream()
		.map(|starttime| {
			Interval::new(starttime, Duration::from_secs(4))
				.map_err(|error| PulseTaskError::Timer { error })
		})
		.flatten()
		.for_each(move |_| {
			sendbuffer_pulse
				.try_send(Message::Pulse)
				.map_err(|error| PulseTaskError::Send { error })
		})
		.or_else(|pe| match pe
		{
			PulseTaskError::Unsupported => Ok(()),
			PulseTaskError::Dropped => Ok(()),
			PulseTaskError::Timer { error } =>
			{
				eprintln!("Timer error in client pulse_task: {:?}", error);
				Ok(())
			}
			PulseTaskError::Send { error } =>
			{
				println!("Send error in pulse_task: {:?}", error);
				Err(io::Error::new(ErrorKind::ConnectionReset, error))
			}
			PulseTaskError::Recv { error } =>
			{
				println!("Recv error in pulse_task: {:?}", error);
				Err(io::Error::new(ErrorKind::ConnectionReset, error))
			}
		});

	let quit_task = quitbuffer_out
		.skip(1)
		.into_future()
		.map(|(_, _)| println!("Client gracefully disconnected."))
		.map_err(|(error, _)| {
			println!("Error in quit_task: {:?}", error);
			io::Error::new(ErrorKind::ConnectionReset, error)
		});

	let task = receive_task
		.join3(ping_task, pulse_task)
		.map(|((), (), ())| ())
		.select(quit_task)
		.map(|((), _)| ())
		.map_err(|(e, _)| e)
		.join(send_task)
		.map(|((), ())| ())
		.map_err(|e| eprintln!("Error in client: {:?}", e));

	tokio::spawn(task);

	Ok(())
}

enum ReceiveTaskError
{
	Quit,
	Goodbye,
	Send
	{
		error: mpsc::error::TrySendError<Message>,
	},
	Recv
	{
		error: io::Error,
	},
}

impl From<mpsc::error::TrySendError<Message>> for ReceiveTaskError
{
	fn from(error: mpsc::error::TrySendError<Message>) -> Self
	{
		ReceiveTaskError::Send { error }
	}
}

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
	Unsupported,
	Dropped,
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
		error: mpsc::error::RecvError,
	},
}

fn handle_message(
	message: Message,
	sendbuffer: &mut mpsc::Sender<Message>,
	pingbuffer: &mut watch::Sender<()>,
	last_receive_time: &mut watch::Sender<()>,
	pong_receive_time: &mut watch::Sender<()>,
	is_versioned: &sync::Arc<atomic::AtomicBool>,
	supports_empty_pulses: &mut mpsc::Sender<bool>,
	quitbuffer: &mut watch::Sender<()>,
) -> Result<(), ReceiveTaskError>
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
			// There might be a Future waiting for this, or there might not be.
			let _ = pong_receive_time.broadcast(());
		}
		Message::Version { version, metadata } =>
		{
			greet_client(
				sendbuffer,
				pingbuffer,
				is_versioned,
				supports_empty_pulses,
				version,
				metadata,
			)?;
		}
		Message::Quit =>
		{
			// There might be a Future waiting for this, or there might not be.
			let _ = quitbuffer.broadcast(());

			return Err(ReceiveTaskError::Quit);
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
	pingbuffer: &mut watch::Sender<()>,
	is_versioned: &sync::Arc<atomic::AtomicBool>,
	supports_empty_pulses: &mut mpsc::Sender<bool>,
	version: Version,
	metadata: Option<PlatformMetadata>,
) -> Result<(), ReceiveTaskError>
{
	is_versioned.store(true, atomic::Ordering::Relaxed);
	println!("Client has version {}", version.to_string());

	let (platform, patchmode) = match metadata
	{
		Some(PlatformMetadata {
			platform,
			patchmode,
		}) =>
		{
			println!("Client has platform {:?}", platform);
			println!("Client has patchmode {:?}", patchmode);
			(platform, patchmode)
		}
		None => (Platform::Unknown, Patchmode::None),
	};

	let myversion = Version::current();
	let response = Message::Version {
		version: myversion,
		metadata: None,
	};
	sendbuffer.try_send(response)?;

	if version.major != myversion.major || version == Version::undefined()
	{
		return Err(ReceiveTaskError::Goodbye);
	}
	else if (patchmode == Patchmode::Itchio
		|| patchmode == Patchmode::Gamejolt)
		&& version < Version::exact(0, 29, 0, 0)
	{
		// Version 0.29.0 was the first closed beta
		// version, which means clients with non-server
		// patchmodes (itch or gamejolt) cannot patch.
		// It is also the first version with keys.
		// Older versions do not properly display the
		// warning that joining failed because of
		// ResponseStatus::KEY_REQUIRED. Instead, we
		// overwrite the 'Version mismatch' message.
		let message = Message::Chat {
			content: "The Open Beta has ended. \
			          Join our Discord community at \
			          www.epicinium.nl/discord \
			          to qualify for access to the \
			          Closed Beta."
				.to_string(),
			sender: Some("server".to_string()),
			target: ChatTarget::General,
		};
		sendbuffer.try_send(message)?;

		return Err(ReceiveTaskError::Goodbye);
	}
	// TODO is_closing
	else if false
	{
		sendbuffer.try_send(Message::Closing)?;
		return Err(ReceiveTaskError::Goodbye);
	}

	let epsupport = version >= Version::exact(0, 31, 1, 0);
	{
		// There might be a Future waiting for this, or there might not be.
		let _ = supports_empty_pulses.try_send(epsupport);
	}

	if version >= Version::exact(0, 31, 1, 0)
	{
		match platform
		{
			Platform::Unknown | Platform::Windows32 | Platform::Windows64 =>
			{}
			Platform::Osx32
			| Platform::Osx64
			| Platform::Debian32
			| Platform::Debian64 =>
			{
				// TODO supports_constructed_symlinks = true;
			}
		}
	}

	if version >= Version::exact(0, 31, 1, 0)
	{
		// TODO supports_gzipped_downloads = true;
	}

	if version >= Version::exact(0, 31, 1, 0)
	{
		// TODO supports_manifest_files = true;
	}

	// Send a ping message, just to get an estimated ping.
	// There might be a Future waiting for this, or there might not be.
	let _ = pingbuffer.broadcast(());

	// TODO async load_notice

	Ok(())
}
