/* Server::Client */

use crate::common::keycode::Keycode;
use crate::common::version::*;
use crate::server::login;
use crate::server::message::*;
use crate::server::tokio::State as ServerState;

use std::fmt;
use std::io;
use std::io::ErrorKind;
use std::sync;
use std::sync::atomic;

use futures::future;
use futures::select;
use futures::FutureExt;
use futures::StreamExt;
use futures::TryFutureExt;

use tokio::io::{ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::time::{Duration, Instant};

use enumset::EnumSet;

const MESSAGE_SIZE_LIMIT: usize = 524288;
const MESSAGE_SIZE_UNVERSIONED_LIMIT: usize = 201;
const MESSAGE_SIZE_WARNING_LIMIT: usize = 65537;

struct Client
{
	sendbuffer: mpsc::Sender<Message>,
	pingbuffer: watch::Sender<()>,
	last_receive_time: watch::Sender<()>,
	pong_receive_time: watch::Sender<()>,
	login: mpsc::Sender<login::Request>,
	has_proper_version: bool,
	has_proper_version_a: sync::Arc<atomic::AtomicBool>,

	pub id: Keycode,
	pub username: String,
	pub version: Version,
	pub unlocks: EnumSet<Unlock>,

	closing: bool,
}

pub fn accept(
	socket: TcpStream,
	id: Keycode,
	login_server: sync::Arc<login::Server>,
	server_state: watch::Receiver<ServerState>,
)
{
	let (sendbuffer_in, sendbuffer_out) = mpsc::channel::<Message>(1000);
	let sendbuffer_ping = sendbuffer_in.clone();
	let sendbuffer_pulse = sendbuffer_in.clone();
	let sendbuffer_login = sendbuffer_in.clone();
	let (joinedbuffer_in, joinedbuffer_out) = mpsc::channel::<Update>(1);
	let (pingbuffer_in, pingbuffer_out) = watch::channel(());
	let (timebuffer_in, timebuffer_out) = watch::channel(());
	let (pongbuffer_in, pongbuffer_out) = watch::channel(());
	let (loginbuffer_in, loginbuffer_out) = mpsc::channel::<login::Request>(1);
	let (reader, writer) = tokio::io::split(socket);

	let client = Client {
		sendbuffer: sendbuffer_in,
		pingbuffer: pingbuffer_in,
		last_receive_time: timebuffer_in,
		pong_receive_time: pongbuffer_in,
		login: loginbuffer_in,
		has_proper_version: false,
		has_proper_version_a: sync::Arc::new(atomic::AtomicBool::new(false)),

		id: id,
		username: String::new(),
		version: Version::undefined(),
		unlocks: EnumSet::empty(),

		closing: false,
	};

	let receive_task =
		start_receive_task(client, joinedbuffer_out, server_state, reader);
	let send_task = start_send_task(id, sendbuffer_out, writer);
	let ping_task = start_ping_task(
		id,
		sendbuffer_ping,
		timebuffer_out,
		pingbuffer_out,
		pongbuffer_out,
	);
	let pulse_task = start_pulse_task(sendbuffer_pulse);
	let login_task = start_login_task(
		sendbuffer_login,
		joinedbuffer_in,
		loginbuffer_out,
		login_server,
	);

	let task = future::try_join5(
		receive_task,
		send_task,
		ping_task,
		pulse_task,
		login_task,
	)
	.map_ok(|((), (), (), (), ())| ())
	.map_err(move |e| eprintln!("Error in client {}: {:?}", id, e))
	.map(move |_result| {
		//let _discarded = canary;
		println!("Client {} done.", id);
	});

	tokio::spawn(task);
}

async fn start_receive_task(
	mut client: Client,
	mut server_updates: mpsc::Receiver<Update>,
	server_state: watch::Receiver<ServerState>,
	mut socket: ReadHalf<TcpStream>,
) -> Result<(), Error>
{
	let mut state_updates = server_state.filter_map(|x| match x
	{
		ServerState::Open => future::ready(None),
		ServerState::Closing => future::ready(Some(Update::Closing)),
		ServerState::Closed => future::ready(Some(Update::Closed)),
	});

	let mut has_quit = false;
	while !has_quit
	{
		let versioned: bool = client.has_proper_version;
		let update = select! {
			x = receive_message(&mut socket, versioned).fuse() => x?,
			x = server_updates.next().fuse() =>
			{
				x.ok_or_else(|| Error::Unexpected)?
			}
			x = state_updates.next().fuse() =>
			{
				x.ok_or_else(|| Error::Unexpected)?
			}
		};
		has_quit = handle_update(&mut client, update)?;
	}

	println!("Client {} stopped receiving.", client.id);
	Ok(())
}

async fn receive_message(
	socket: &mut ReadHalf<TcpStream>,
	versioned: bool,
) -> Result<Update, Error>
{
	let mut lengthbuffer = [0u8; 4];
	socket.read_exact(&mut lengthbuffer).await?;

	let length = u32::from_be_bytes(lengthbuffer);
	if length == 0
	{
		/*verbose*/
		println!("Received pulse.");

		// Unfold expects the value first and the state second.
		return Ok(Update::Msg(Message::Pulse));
	}
	else if !versioned && length as usize >= MESSAGE_SIZE_UNVERSIONED_LIMIT
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
		)
		.into());
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
		)
		.into());
	}
	else if length as usize >= MESSAGE_SIZE_WARNING_LIMIT
	{
		println!("Receiving very large message of length {}", length);
	}

	/*verbose*/
	println!("Receiving message of length {}...", length);

	let mut buffer = vec![0; length as usize];
	socket.read_exact(&mut buffer).await?;

	/*verbose*/
	println!("Received message of length {}.", buffer.len());
	let message = parse_message(buffer)?;

	// Unfold expects the value first and the state second.
	Ok(Update::Msg(message))
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
		/*verbose*/
		println!("Received message: {}", jsonstr);
	}

	let message: Message = serde_json::from_str(&jsonstr)?;

	Ok(message)
}

async fn start_send_task(
	client_id: Keycode,
	mut sendbuffer: mpsc::Receiver<Message>,
	mut socket: WriteHalf<TcpStream>,
) -> Result<(), Error>
{
	while let Some(message) = sendbuffer.next().await
	{
		let buffer = prepare_message(message);
		send_bytes(&mut socket, buffer).await?;
	}

	println!("Client {} stopped sending.", client_id);
	Ok(())
}

async fn send_bytes(
	socket: &mut WriteHalf<TcpStream>,
	buffer: Vec<u8>,
) -> Result<(), io::Error>
{
	socket.write_all(&buffer).await?;

	/*verbose*/
	println!("Sent {} bytes.", buffer.len());
	Ok(())
}

fn prepare_message(message: Message) -> Vec<u8>
{
	if let Message::Pulse = message
	{
		/*verbose*/
		println!("Sending pulse...");

		let zeroes = [0u8; 4];
		return zeroes.to_vec();
	}

	let (jsonstr, length) = prepare_message_data(message);

	let mut buffer = length.to_be_bytes().to_vec();
	buffer.append(&mut jsonstr.into_bytes());

	buffer
}

fn prepare_message_data(message: Message) -> (String, u32)
{
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

	/*verbose*/
	println!("Sending message of length {}...", length);

	if length < 200
	{
		/*verbose*/
		println!("Sending message: {}", jsonstr);
	}

	(jsonstr, length)
}

async fn start_ping_task(
	client_id: Keycode,
	mut sendbuffer: mpsc::Sender<Message>,
	timebuffer: watch::Receiver<()>,
	pingbuffer: watch::Receiver<()>,
	pongbuffer: watch::Receiver<()>,
) -> Result<(), Error>
{
	Ok(())
}

async fn start_pulse_task(
	mut sendbuffer: mpsc::Sender<Message>,
) -> Result<(), Error>
{
	let start = Instant::now() + Duration::from_secs(4);
	let mut interval = tokio::time::interval_at(start, Duration::from_secs(4));

	loop
	{
		interval.tick().await;
		sendbuffer.send(Message::Pulse).await?;
	}
}

async fn start_login_task(
	sendbuffer: mpsc::Sender<Message>,
	mut joinedbuffer: mpsc::Sender<Update>,
	requestbuffer: mpsc::Receiver<login::Request>,
	login_server: sync::Arc<login::Server>,
) -> Result<(), Error>
{
	Ok(())
}

#[derive(Debug)]
pub enum Update
{
	JoinedServer
	{
		username: String,
		unlocks: EnumSet<Unlock>,
	},
	Closing,
	Closed,
	Msg(Message),
}

#[derive(Debug)]
enum Error
{
	Illegal,
	Unexpected,
	Send
	{
		error: mpsc::error::SendError<Message>,
	},
	TrySend
	{
		error: mpsc::error::TrySendError<Message>,
	},
	Login
	{
		error: mpsc::error::TrySendError<login::Request>,
	},
	Watch
	{
		error: watch::error::SendError<()>,
	},
	Recv
	{
		error: mpsc::error::RecvError,
	},
	Io
	{
		error: io::Error,
	},
}

impl From<mpsc::error::SendError<Message>> for Error
{
	fn from(error: mpsc::error::SendError<Message>) -> Self
	{
		Error::Send { error }
	}
}

impl From<mpsc::error::TrySendError<Message>> for Error
{
	fn from(error: mpsc::error::TrySendError<Message>) -> Self
	{
		Error::TrySend { error }
	}
}

impl From<mpsc::error::TrySendError<login::Request>> for Error
{
	fn from(error: mpsc::error::TrySendError<login::Request>) -> Self
	{
		Error::Login { error }
	}
}

impl From<watch::error::SendError<()>> for Error
{
	fn from(error: watch::error::SendError<()>) -> Self
	{
		Error::Watch { error }
	}
}

impl From<mpsc::error::RecvError> for Error
{
	fn from(error: mpsc::error::RecvError) -> Self
	{
		Error::Recv { error }
	}
}

impl From<io::Error> for Error
{
	fn from(error: io::Error) -> Self
	{
		Error::Io { error }
	}
}

impl fmt::Display for Error
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
	{
		match self
		{
			Error::Illegal => write!(f, "Illegal message received"),
			Error::Unexpected => write!(f, "Something unexpected happened"),
			Error::Send { error } => error.fmt(f),
			Error::TrySend { error } => error.fmt(f),
			Error::Login { error } => error.fmt(f),
			Error::Watch { error } => error.fmt(f),
			Error::Recv { error } => error.fmt(f),
			Error::Io { error } => error.fmt(f),
		}
	}
}

fn handle_update(client: &mut Client, update: Update) -> Result<bool, Error>
{
	match update
	{
		Update::JoinedServer { .. } => unimplemented!(),

		Update::Closing =>
		{
			client.closing = true;
			client.sendbuffer.try_send(Message::Closing)?;
			Ok(false)
		}
		Update::Closed =>
		{
			client.closing = true;
			client.sendbuffer.try_send(Message::Closed)?;
			Ok(false)
		}

		Update::Msg(message) => handle_message(client, message),
	}
}

fn handle_message(client: &mut Client, message: Message)
	-> Result<bool, Error>
{
	client.last_receive_time.broadcast(())?;

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
			client.pong_receive_time.broadcast(())?;
		}
		Message::Version { version } =>
		{
			greet_client(client, version)?;
		}
		Message::Quit =>
		{
			println!("Client {} gracefully disconnected.", client.id);
			client.sendbuffer.try_send(Message::Quit)?;
			return Ok(true);
		}
		_ =>
		{
			println!("Invalid message from client: {:?}", message);
			return Err(Error::Illegal);
		}
	}

	Ok(false)
}

fn greet_client(client: &mut Client, version: Version) -> Result<(), Error>
{
	client.version = version;
	/*verbose*/
	println!("Client {} has version {}.", client.id, version.to_string());

	let myversion = Version::current();
	let response = Message::Version { version: myversion };
	client.sendbuffer.try_send(response)?;

	if version.major != myversion.major || version == Version::undefined()
	{
		// The client does not have a proper version.
		return Ok(());
	}
	else if version < Version::exact(0, 33, 0, 0)
	{
		// Clients older than 0.33.0 should connect to the C++ server;
		// this server should not be connected to a port that isn't behind
		// a portal page. We have sent them our version so they should know
		// that a patch is available.
		debug_assert!(false, "Cannot serve clients older than v0.33.0!");

		// We treat the client as if they do not have a proper version,
		// because we do not want to receive any more messages.
		return Ok(());
	}
	else if client.closing
	{
		client.sendbuffer.try_send(Message::Closed)?;

		// We treat the client as if they do not have a proper version,
		// because we do not want to receive any more messages.
		return Ok(());
	}

	// If we got this far, the client has a proper version.
	client.has_proper_version = true;
	client
		.has_proper_version_a
		.store(true, atomic::Ordering::Relaxed);

	// TODO enable compression

	// Send a ping message, just to get an estimated ping.
	client.pingbuffer.broadcast(())?;

	Ok(())
}

fn joining_server(
	client: &mut Client,
	token: String,
	account_id: String,
) -> Result<(), Error>
{
	println!(
		"Client {} is logging in with account id {}",
		client.id, &account_id
	);

	match client.login.try_send(login::Request { token, account_id })
	{
		Ok(()) => Ok(()),
		Err(mpsc::error::TrySendError::Full(_request)) =>
		{
			eprintln!("Failed to enqueue for login, queue full");

			// TODO better error handling (#962)
			let message = Message::JoinServer {
				status: Some(ResponseStatus::ConnectionFailed),
				content: None,
				sender: None,
				metadata: None,
			};
			client.sendbuffer.try_send(message)?;
			Ok(())
		}
		Err(error) => Err(error.into()),
	}
}
