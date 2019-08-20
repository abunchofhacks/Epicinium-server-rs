/* TokioServer */

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

use tokio::io::{ReadHalf, WriteHalf};
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

	// TODO signal handling

	tokio::run(server);

	Ok(())
}

struct Client
{
	sendbuffer: mpsc::Sender<Message>,
	pingbuffer: watch::Sender<()>,
	last_receive_time: watch::Sender<()>,
	pong_receive_time: watch::Sender<()>,
	is_versioned: sync::Arc<atomic::AtomicBool>,
	supports_empty_pulses: mpsc::Sender<bool>,
	quitbuffer: watch::Sender<()>,

	pub version: Version,
	pub platform: Platform,
	pub patchmode: Patchmode,
}

fn accept_client(socket: TcpStream) -> io::Result<()>
{
	let (sendbuffer_in, sendbuffer_out) = mpsc::channel::<Message>(1000);
	let sendbuffer_ping = sendbuffer_in.clone();
	let sendbuffer_pulse = sendbuffer_in.clone();
	let (pingbuffer_in, pingbuffer_out) = watch::channel(());
	let (timebuffer_in, timebuffer_out) = watch::channel(());
	let (pongbuffer_in, pongbuffer_out) = watch::channel(());
	let (quitbuffer_in, quitbuffer_out) = watch::channel(());
	let (supports_empty_in, supports_empty_out) = mpsc::channel::<bool>(1);
	let (reader, writer) = socket.split();

	let client = Client {
		sendbuffer: sendbuffer_in,
		pingbuffer: pingbuffer_in,
		last_receive_time: timebuffer_in,
		pong_receive_time: pongbuffer_in,
		is_versioned: sync::Arc::new(atomic::AtomicBool::new(false)),
		supports_empty_pulses: supports_empty_in,
		quitbuffer: quitbuffer_in,

		version: Version::undefined(),
		platform: Platform::Unknown,
		patchmode: Patchmode::None,
	};

	let receive_task = start_recieve_task(client, reader);
	let send_task = start_send_task(sendbuffer_out, writer);
	let ping_task = start_ping_task(
		sendbuffer_ping,
		timebuffer_out,
		pingbuffer_out,
		pongbuffer_out,
	);
	let pulse_task = start_pulse_task(sendbuffer_pulse, supports_empty_out);
	let quit_task = start_quit_task(quitbuffer_out);

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

fn start_recieve_task(
	mut client: Client,
	socket: ReadHalf<TcpStream>,
) -> impl Future<Item = (), Error = io::Error>
{
	let receive_versioned = client.is_versioned.clone();

	stream::unfold(socket, move |socket| {
		let versioned: bool = receive_versioned.load(atomic::Ordering::Relaxed);
		let lengthbuffer = [0u8; 4];
		let future_length = tokio_io::io::read_exact(socket, lengthbuffer)
			.and_then(move |(socket, lengthbuffer)| {
				let length = u32::from_le_bytes(lengthbuffer);
				receive_message(socket, length, versioned)
			});

		Some(future_length)
	})
	.map_err(|error| ReceiveTaskError::Recv { error })
	.for_each(move |message: Message| handle_message(&mut client, message))
	.or_else(|error| -> io::Result<()> { error.into() })
	.map(|()| println!("Stopped receiving."))
}

fn receive_message(
	socket: ReadHalf<TcpStream>,
	length: u32,
	versioned: bool,
) -> impl Future<Item = (Message, ReadHalf<TcpStream>), Error = io::Error>
{
	if length == 0
	{
		println!("Received pulse.");

		// Unfold expects the value first and the state second.
		let result = Ok((Message::Pulse, socket));
		let future_data = future::result(result);
		return Either::A(future_data);
	}
	else if !versioned && length as usize >= MESSAGE_SIZE_UNVERSIONED_LIMIT
	{
		println!(
			"Unversioned client tried to send \
			 very large message of length {}, \
			 which is more than MESSAGE_SIZE_UNVERSIONED_LIMIT",
			length
		);
		return Either::A(future::err(io::Error::new(
			ErrorKind::InvalidInput,
			"Message too large".to_string(),
		)));
	}
	else if length as usize >= MESSAGE_SIZE_LIMIT
	{
		println!(
			"Refusing to receive very large message of length {}, \
			 which is more than MESSAGE_SIZE_LIMIT",
			length
		);
		return Either::A(future::err(io::Error::new(
			ErrorKind::InvalidInput,
			"Message too large".to_string(),
		)));
	}
	else if length as usize >= MESSAGE_SIZE_WARNING_LIMIT
	{
		println!("Receiving very large message of length {}", length);
	}

	println!("Receiving message of length {}...", length);

	let buffer = vec![0; length as usize];
	let future_data = tokio_io::io::read_exact(socket, buffer).and_then(
		|(socket, buffer)| {
			println!("Received message of length {}.", buffer.len());
			let message = parse_message(buffer)?;

			// Unfold expects the value first and the state second.
			Ok((message, socket))
		},
	);

	Either::B(future_data)
}

fn parse_message(buffer: Vec<u8>) -> io::Result<Message>
{
	let jsonstr = match String::from_utf8(buffer)
	{
		Ok(x) => x,
		Err(e) =>
		{
			return Err(io::Error::new(ErrorKind::InvalidData, e));
		}
	};

	if jsonstr.len() < 200
	{
		println!("Received message: {}", jsonstr);
	}

	let message: Message = serde_json::from_str(&jsonstr)?;

	Ok(message)
}

fn start_send_task(
	sendbuffer: mpsc::Receiver<Message>,
	socket: WriteHalf<TcpStream>,
) -> impl Future<Item = (), Error = io::Error>
{
	sendbuffer
		.map_err(|e| io::Error::new(ErrorKind::ConnectionReset, e))
		.fold(socket, send_message)
		.map_err(|error| {
			eprintln!("Error in send_task: {:?}", error);
			error
		})
		.map(|_socket| println!("Stopped sending."))
}

fn send_message(
	socket: WriteHalf<TcpStream>,
	message: Message,
) -> impl Future<Item = WriteHalf<TcpStream>, Error = io::Error>
{
	let buffer = prepare_message(message);

	tokio_io::io::write_all(socket, buffer).map(move |(socket, buffer)| {
		println!("Sent {} bytes.", buffer.len());
		socket
	})
}

fn prepare_message(message: Message) -> Vec<u8>
{
	if let Message::Pulse = message
	{
		println!("Sending pulse...");

		let zeroes = [0u8; 4];
		return zeroes.to_vec();
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

	buffer
}

fn start_ping_task(
	mut sendbuffer: mpsc::Sender<Message>,
	timebuffer: watch::Receiver<()>,
	pingbuffer: watch::Receiver<()>,
	pongbuffer: watch::Receiver<()>,
) -> impl Future<Item = (), Error = io::Error>
{
	// TODO variable ping_tolerance
	let ping_tolerance = Duration::from_secs(120);

	timebuffer
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
			pingbuffer
				.skip(1)
				.map_err(|error| PingTaskError::Recv { error }),
		)
		.and_then(move |()| {
			let pingtime = Instant::now();
			sendbuffer
				.try_send(Message::Ping)
				.map(move |()| pingtime)
				.map_err(|error| PingTaskError::Send { error })
		})
		.and_then(move |pingtime| {
			pongbuffer
				.clone()
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
		.or_else(|error| -> io::Result<()> { error.into() })
		.for_each(|()| Ok(()))
}

fn start_pulse_task(
	mut sendbuffer: mpsc::Sender<Message>,
	supported: mpsc::Receiver<bool>,
) -> impl Future<Item = (), Error = io::Error>
{
	supported
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
			sendbuffer
				.try_send(Message::Pulse)
				.map_err(|error| PulseTaskError::Send { error })
		})
		.or_else(|error| -> io::Result<()> { error.into() })
}

fn start_quit_task(
	quitbuffer: watch::Receiver<()>,
) -> impl Future<Item = (), Error = io::Error>
{
	quitbuffer
		.skip(1)
		.into_future()
		.map(|(_, _)| println!("Client gracefully disconnected."))
		.map_err(|(error, _)| {
			eprintln!("Error in quit_task: {:?}", error);
			io::Error::new(ErrorKind::ConnectionReset, error)
		})
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

impl Into<io::Result<()>> for ReceiveTaskError
{
	fn into(self) -> io::Result<()>
	{
		match self
		{
			ReceiveTaskError::Quit => Ok(()),
			ReceiveTaskError::Goodbye => Ok(()),
			ReceiveTaskError::Send { error } =>
			{
				eprintln!("Send error in receive_task: {:?}", error);
				Err(io::Error::new(ErrorKind::ConnectionReset, error))
			}
			ReceiveTaskError::Recv { error } =>
			{
				eprintln!("Recv error in receive_task: {:?}", error);
				Err(error)
			}
		}
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

impl Into<io::Result<()>> for PingTaskError
{
	fn into(self) -> io::Result<()>
	{
		match self
		{
			PingTaskError::Timer { error } =>
			{
				eprintln!("Timer error in client ping_task: {:?}", error);
				Ok(())
			}
			PingTaskError::Send { error } =>
			{
				eprintln!("Send error in ping_task: {:?}", error);
				Err(io::Error::new(ErrorKind::ConnectionReset, error))
			}
			PingTaskError::Recv { error } =>
			{
				eprintln!("Recv error in ping_task: {:?}", error);
				Err(io::Error::new(ErrorKind::ConnectionReset, error))
			}
			PingTaskError::NoPong =>
			{
				println!("Client failed to respond to ping.");
				Err(io::Error::new(ErrorKind::ConnectionReset, "no pong"))
			}
		}
	}
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

impl Into<io::Result<()>> for PulseTaskError
{
	fn into(self) -> io::Result<()>
	{
		match self
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
				eprintln!("Send error in pulse_task: {:?}", error);
				Err(io::Error::new(ErrorKind::ConnectionReset, error))
			}
			PulseTaskError::Recv { error } =>
			{
				eprintln!("Recv error in pulse_task: {:?}", error);
				Err(io::Error::new(ErrorKind::ConnectionReset, error))
			}
		}
	}
}

fn handle_message(
	client: &mut Client,
	message: Message,
) -> Result<(), ReceiveTaskError>
{
	// There might be a Future tracking when we last received a message,
	// or there might not be.
	let _ = client.last_receive_time.broadcast(());

	match message
	{
		Message::Pulse =>
		{
			// The client just let us know that it is still breathing.
		}
		Message::Ping =>
		{
			// Pings must always be responded with pongs.
			client.sendbuffer.try_send(Message::Pong)?;
		}
		Message::Pong =>
		{
			// There might be a Future waiting for this, or there might not be.
			let _ = client.pong_receive_time.broadcast(());
		}
		Message::Version { version, metadata } =>
		{
			greet_client(client, version, metadata)?;
		}
		Message::Quit =>
		{
			// There might be a Future waiting for this, or there might not be.
			let _ = client.quitbuffer.broadcast(());

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
	client: &mut Client,
	version: Version,
	metadata: Option<PlatformMetadata>,
) -> Result<(), ReceiveTaskError>
{
	client.version = version;
	client.is_versioned.store(true, atomic::Ordering::Relaxed);
	println!("Client has version {}", version.to_string());

	if let Some(PlatformMetadata {
		platform,
		patchmode,
	}) = metadata
	{
		client.platform = platform;
		println!("Client has platform {:?}", platform);
		client.patchmode = patchmode;
		println!("Client has patchmode {:?}", patchmode);
	}

	let myversion = Version::current();
	let response = Message::Version {
		version: myversion,
		metadata: None,
	};
	client.sendbuffer.try_send(response)?;

	if version.major != myversion.major || version == Version::undefined()
	{
		return Err(ReceiveTaskError::Goodbye);
	}
	else if (client.patchmode == Patchmode::Itchio
		|| client.patchmode == Patchmode::Gamejolt)
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
		client.sendbuffer.try_send(message)?;

		return Err(ReceiveTaskError::Goodbye);
	}
	// TODO is_closing
	else if false
	{
		client.sendbuffer.try_send(Message::Closing)?;
		return Err(ReceiveTaskError::Goodbye);
	}

	let epsupport = version >= Version::exact(0, 31, 1, 0);
	{
		// There might be a Future waiting for this, or there might not be.
		let _ = client.supports_empty_pulses.try_send(epsupport);
	}

	if version >= Version::exact(0, 31, 1, 0)
	{
		match client.platform
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
	let _ = client.pingbuffer.broadcast(());

	// TODO async load_notice

	// TODO mention patches

	Ok(())
}
