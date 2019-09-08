/* Server::Client */

use common::base32;
use common::keycode::Keycode;
use common::version::*;
use server::chat;
use server::limits::*;
use server::login;
use server::message::*;
use server::notice;
use server::patch::*;
use server::tokio::State as ServerState;

use std::io;
use std::io::ErrorKind;
use std::path::PathBuf;
use std::sync;
use std::sync::atomic;
use std::time::{Duration, Instant};

use tokio::io::{ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::timer::Interval;

use futures::future;
use futures::future::Either;
use futures::stream;
use futures::{Future, Stream};

use enumset::EnumSet;

pub type PrivateKey = openssl::pkey::PKey<openssl::pkey::Private>;

struct Client
{
	sendbuffer: mpsc::Sender<Message>,
	pingbuffer: watch::Sender<()>,
	last_receive_time: watch::Sender<()>,
	pong_receive_time: watch::Sender<()>,
	trigger_notice: mpsc::Sender<()>,
	requests: mpsc::Sender<String>,
	login: mpsc::Sender<login::Request>,
	general_chat: Option<mpsc::Sender<chat::Update>>,
	has_proper_version: bool,
	has_proper_version_a: sync::Arc<atomic::AtomicBool>,
	supports_empty_pulses: mpsc::Sender<bool>,

	pub id: Keycode,
	pub username: String,
	pub version: Version,
	pub platform: Platform,
	pub patchmode: Patchmode,
	pub unlocks: EnumSet<Unlock>,

	closing: bool,
}

impl Client
{
	fn is_unversioned(&self) -> bool
	{
		!self.has_proper_version
	}
}

impl Drop for Client
{
	fn drop(&mut self)
	{
		match leave_general_chat(self)
		{
			Ok(()) => (),
			Err(e) => eprintln!("Error while dropping client: {:?}", e),
		}
	}
}

fn leave_general_chat(
	client: &mut Client,
) -> Result<(), mpsc::error::TrySendError<chat::Update>>
{
	match client.general_chat.take()
	{
		Some(mut general_chat) => general_chat.try_send(chat::Update::Leave {
			client_id: client.id,
		}),
		None => Ok(()),
	}
}

pub fn accept_client(
	socket: TcpStream,
	id: Keycode,
	login_server: sync::Arc<login::Server>,
	chat_server: mpsc::Sender<chat::Update>,
	server_state: watch::Receiver<ServerState>,
	live_count: sync::Arc<atomic::AtomicUsize>,
	privatekey: sync::Arc<PrivateKey>,
) -> io::Result<()>
{
	live_count.fetch_add(1, atomic::Ordering::Relaxed);

	let (sendbuffer_in, sendbuffer_out) = mpsc::channel::<Message>(1000);
	let sendbuffer_ping = sendbuffer_in.clone();
	let sendbuffer_pulse = sendbuffer_in.clone();
	let sendbuffer_notice = sendbuffer_in.clone();
	let sendbuffer_request = sendbuffer_in.clone();
	let sendbuffer_login = sendbuffer_in.clone();
	let (dlbuffer_in, dlbuffer_out) = mpsc::channel::<(Message, Vec<u8>)>(1);
	let (joinedbuffer_in, joinedbuffer_out) = mpsc::channel::<Update>(1);
	let (pingbuffer_in, pingbuffer_out) = watch::channel(());
	let (timebuffer_in, timebuffer_out) = watch::channel(());
	let (pongbuffer_in, pongbuffer_out) = watch::channel(());
	let (supports_empty_in, supports_empty_out) = mpsc::channel::<bool>(1);
	let (noticebuffer_in, noticebuffer_out) = mpsc::channel::<()>(1);
	let (requestbuffer_in, requestbuffer_out) = mpsc::channel::<String>(10);
	let (loginbuffer_in, loginbuffer_out) = mpsc::channel::<login::Request>(1);
	let (reader, writer) = socket.split();

	let client = Client {
		sendbuffer: sendbuffer_in,
		pingbuffer: pingbuffer_in,
		last_receive_time: timebuffer_in,
		pong_receive_time: pongbuffer_in,
		trigger_notice: noticebuffer_in,
		requests: requestbuffer_in,
		login: loginbuffer_in,
		general_chat: None,
		has_proper_version: false,
		has_proper_version_a: sync::Arc::new(atomic::AtomicBool::new(false)),
		supports_empty_pulses: supports_empty_in,

		id: id,
		username: String::new(),
		version: Version::undefined(),
		platform: Platform::Unknown,
		patchmode: Patchmode::None,
		unlocks: EnumSet::empty(),

		closing: false,
	};

	let receive_task =
		start_receive_task(client, joinedbuffer_out, server_state, reader);
	let send_task = start_send_task(id, sendbuffer_out, dlbuffer_out, writer);
	let ping_task = start_ping_task(
		id,
		sendbuffer_ping,
		timebuffer_out,
		pingbuffer_out,
		pongbuffer_out,
	);
	let pulse_task = start_pulse_task(sendbuffer_pulse, supports_empty_out);
	let notice_task = start_notice_task(sendbuffer_notice, noticebuffer_out);
	let request_task = start_request_task(
		sendbuffer_request,
		dlbuffer_in,
		requestbuffer_out,
		privatekey,
	);
	let login_task = start_login_task(
		id,
		sendbuffer_login,
		joinedbuffer_in,
		loginbuffer_out,
		login_server,
		chat_server,
	);

	let support_task = login_task
		.join5(ping_task, pulse_task, notice_task, request_task)
		.map(|((), (), (), (), ())| ())
		.map(|()| debug_assert!(false, "All support tasks dropped"));

	let task = receive_task
		.select(support_task)
		.map(|((), _other_future)| ())
		.map_err(|(error, _other_future)| error)
		.join(send_task)
		.map(|((), ())| ())
		.map_err(move |e| eprintln!("Error in client {}: {:?}", id, e))
		.then(move |result| {
			live_count.fetch_sub(1, atomic::Ordering::Relaxed);
			result
		});

	tokio::spawn(task);

	Ok(())
}

fn start_receive_task(
	mut client: Client,
	servermessages: mpsc::Receiver<Update>,
	server_state: watch::Receiver<ServerState>,
	socket: ReadHalf<TcpStream>,
) -> impl Future<Item = (), Error = io::Error> + Send
{
	let client_id = client.id;
	let receive_versioned = client.has_proper_version_a.clone();

	let killcount_updates = server_state
		.filter_map(|x| match x
		{
			ServerState::Open => None,
			ServerState::Closing => Some(Update::Closing),
			ServerState::Closed => Some(Update::Quit),
		})
		.map_err(|error| ReceiveTaskError::Killcount { error });

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
	.map(|message| Update::Msg(message))
	.select(servermessages.map_err(|error| ReceiveTaskError::Server { error }))
	.select(killcount_updates)
	.for_each(move |update: Update| handle_update(&mut client, update))
	.or_else(|error| -> io::Result<()> { error.into() })
	.map(move |()| println!("Client {} stopped receiving.", client_id))
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
	client_id: Keycode,
	sendbuffer: mpsc::Receiver<Message>,
	downloadbuffer: mpsc::Receiver<(Message, Vec<u8>)>,
	socket: WriteHalf<TcpStream>,
) -> impl Future<Item = (), Error = io::Error> + Send
{
	let messages = sendbuffer
		.map_err(|e| io::Error::new(ErrorKind::ConnectionReset, e))
		.map(prepare_message);

	let downloads = downloadbuffer
		.map_err(|e| io::Error::new(ErrorKind::ConnectionReset, e))
		.map(|(message, data)| prepare_download(message, data));

	messages
		.select(downloads)
		.fold(socket, send_bytes)
		.map_err(|error| {
			eprintln!("Error in send_task: {:?}", error);
			error
		})
		.map(move |_socket| println!("Client {} stopped sending.", client_id))
}

fn send_bytes(
	socket: WriteHalf<TcpStream>,
	buffer: Vec<u8>,
) -> impl Future<Item = WriteHalf<TcpStream>, Error = io::Error> + Send
{
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

	let (jsonstr, length) = prepare_message_data(message);

	let mut buffer = length.to_le_bytes().to_vec();
	buffer.append(&mut jsonstr.into_bytes());

	buffer
}

fn prepare_download(message: Message, mut buffer: Vec<u8>) -> Vec<u8>
{
	let (jsonstr, length) = prepare_message_data(message);
	let size = prepare_buffer_size(&buffer);

	buffer.append(&mut length.to_le_bytes().to_vec());
	buffer.append(&mut jsonstr.into_bytes());
	buffer.append(&mut size.to_le_bytes().to_vec());

	// The buffer contained `size` bytes, and we have appended 4 bytes of
	// `length`, `length` bytes of `jsonstr` and 4 bytes of `size`.  We need
	// to rotate the `size` original bytes to the end of the buffer.
	buffer.rotate_left(size as usize);

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

	println!("Sending message of length {}...", length);

	if length < 200
	{
		println!("Sending message: {}", jsonstr);
	}

	(jsonstr, length)
}

fn prepare_buffer_size(buffer: &Vec<u8>) -> u32
{
	if buffer.len() >= MESSAGE_SIZE_LIMIT
	{
		panic!(
			"Cannot send chunk of size {}, \
			 which is larger than MESSAGE_SIZE_LIMIT.",
			buffer.len()
		);
	}

	let size = buffer.len() as u32;

	if size as usize > SEND_FILE_CHUNK_SIZE
	{
		println!(
			"Queueing chunk of size {} \
			 which is larger than SEND_FILE_CHUNK_SIZE.",
			size
		);
	}

	println!("And sending chunk of size {}...", size);

	size
}

fn start_ping_task(
	client_id: Keycode,
	mut sendbuffer: mpsc::Sender<Message>,
	timebuffer: watch::Receiver<()>,
	pingbuffer: watch::Receiver<()>,
	pongbuffer: watch::Receiver<()>,
) -> impl Future<Item = (), Error = io::Error> + Send
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
			println!(
				"Client {} has {}ms ping.",
				client_id,
				pingtime.elapsed().as_millis()
			);
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
) -> impl Future<Item = (), Error = io::Error> + Send
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

fn start_notice_task(
	mut sendbuffer: mpsc::Sender<Message>,
	noticebuffer: mpsc::Receiver<()>,
) -> impl Future<Item = (), Error = io::Error> + Send
{
	noticebuffer
		.map_err(|error| {
			eprintln!("Recv error in notice_task: {:?}", error);
			io::Error::new(ErrorKind::ConnectionReset, error)
		})
		.and_then(|()| notice::load().map(|x| Some(x)).or_else(|_| Ok(None)))
		.filter_map(|x| x)
		.for_each(move |notice| {
			sendbuffer
				.try_send(Message::Stamp { metadata: notice })
				.map_err(|error| {
					eprintln!("Send error in notice_task: {:?}", error);
					io::Error::new(ErrorKind::ConnectionReset, error)
				})
		})
}

fn start_request_task(
	mut sendbuffer: mpsc::Sender<Message>,
	downloadbuffer: mpsc::Sender<(Message, Vec<u8>)>,
	requestbuffer: mpsc::Receiver<String>,
	privatekey: sync::Arc<PrivateKey>,
) -> impl Future<Item = (), Error = io::Error> + Send
{
	requestbuffer
		.map_err(|error| {
			eprintln!("Recv error in request_task: {:?}", error);
			io::Error::new(ErrorKind::ConnectionReset, error)
		})
		.and_then(move |name| {
			fulfil_request(downloadbuffer.clone(), name, privatekey.clone())
		})
		.for_each(move |response| match sendbuffer.try_send(response)
		{
			Ok(()) => Ok(()),
			Err(error) =>
			{
				eprintln!("Send error in request_task: {:?}", error);
				Err(io::Error::new(ErrorKind::ConnectionReset, error))
			}
		})
}

fn start_login_task(
	client_id: Keycode,
	sendbuffer: mpsc::Sender<Message>,
	mut joinedbuffer: mpsc::Sender<Update>,
	requestbuffer: mpsc::Receiver<login::Request>,
	login_server: sync::Arc<login::Server>,
	mut chat_server: mpsc::Sender<chat::Update>,
) -> impl Future<Item = (), Error = io::Error> + Send
{
	let mut sendbuffer_for_login_failure = sendbuffer.clone();
	let mut sendbuffer_for_access_failure = sendbuffer.clone();

	requestbuffer
		.map_err(|error| {
			eprintln!("Recv error in login_task: {:?}", error);
			io::Error::new(ErrorKind::ConnectionReset, error)
		})
		.and_then(move |request| {
			login_server
				.login(request)
				.map(|logindata| Ok(logindata))
				.or_else(|status| Ok(Err(status)))
		})
		.and_then(move |result| match result
		{
			Ok(logindata) => Ok(Some(logindata)),
			Err(status) =>
			{
				eprintln!("Login failed with {:?}", status);
				let message = Message::JoinServer {
					status: Some(status),
					content: None,
					sender: None,
					metadata: None,
				};
				sendbuffer_for_login_failure
					.try_send(message)
					.map_err(|error| {
						eprintln!("Send error in login_task: {:?}", error);
						io::Error::new(ErrorKind::ConnectionReset, error)
					})
					.map(|()| None)
			}
		})
		.filter_map(|x| x)
		.and_then(move |logindata| {
			let mut unlocks = EnumSet::<Unlock>::empty();
			for &x in &logindata.unlocks
			{
				unlocks.insert(unlock_from_unlock_id(x));
			}

			if !unlocks.contains(Unlock::Access)
			{
				println!("Login failed due to insufficient access");
				return sendbuffer_for_access_failure
					.try_send(Message::JoinServer {
						status: Some(ResponseStatus::KeyRequired),
						content: None,
						sender: None,
						metadata: None,
					})
					.map_err(|error| {
						eprintln!("Send error in login_task: {:?}", error);
						io::Error::new(ErrorKind::ConnectionReset, error)
					})
					.map(|()| None);
			}

			Ok(Some((logindata.username, unlocks)))
		})
		.filter_map(|x| x)
		.and_then(move |(username, unlocks)| {
			chat_server
				.try_send(chat::Update::Join {
					client_id,
					username: username.clone(),
					unlocks,
					sendbuffer: sendbuffer.clone(),
				})
				.map_err(|error| {
					eprintln!("Joining chat error in login_task: {:?}", error);
					io::Error::new(ErrorKind::ConnectionReset, error)
				})
				.map(|()| Update::Join {
					username: username,
					unlocks: unlocks,
					general_chat: chat_server.clone(),
				})
		})
		.for_each(move |update| {
			joinedbuffer.try_send(update).map_err(|error| {
				eprintln!("Send error in login_task: {:?}", error);
				io::Error::new(ErrorKind::ConnectionReset, error)
			})
		})
}

#[derive(Debug)]
enum Update
{
	Join
	{
		username: String,
		unlocks: EnumSet<Unlock>,
		general_chat: mpsc::Sender<chat::Update>,
	},
	Closing,
	Quit,
	Msg(Message),
}

enum ReceiveTaskError
{
	Quit,
	Illegal,
	Send
	{
		error: mpsc::error::TrySendError<Message>,
	},
	Notice
	{
		error: mpsc::error::TrySendError<String>,
	},
	Login
	{
		error: mpsc::error::TrySendError<login::Request>,
	},
	Chat
	{
		error: mpsc::error::TrySendError<chat::Update>,
	},
	Server
	{
		error: mpsc::error::RecvError,
	},
	Killcount
	{
		error: watch::error::RecvError,
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

impl From<mpsc::error::TrySendError<chat::Update>> for ReceiveTaskError
{
	fn from(error: mpsc::error::TrySendError<chat::Update>) -> Self
	{
		ReceiveTaskError::Chat { error }
	}
}

impl Into<io::Result<()>> for ReceiveTaskError
{
	fn into(self) -> io::Result<()>
	{
		match self
		{
			ReceiveTaskError::Quit => Ok(()),
			ReceiveTaskError::Illegal => Err(io::Error::new(
				ErrorKind::ConnectionReset,
				"Illegal message received",
			)),
			ReceiveTaskError::Send { error } =>
			{
				eprintln!("Send error in receive_task: {:?}", error);
				Err(io::Error::new(ErrorKind::ConnectionReset, error))
			}
			ReceiveTaskError::Notice { error } =>
			{
				eprintln!("Notice error in receive_task: {:?}", error);
				Err(io::Error::new(ErrorKind::ConnectionReset, error))
			}
			ReceiveTaskError::Login { error } =>
			{
				eprintln!("Login error in receive_task: {:?}", error);
				Err(io::Error::new(ErrorKind::ConnectionReset, error))
			}
			ReceiveTaskError::Chat { error } =>
			{
				eprintln!("Chat error in receive_task: {:?}", error);
				Err(io::Error::new(ErrorKind::ConnectionReset, error))
			}
			ReceiveTaskError::Server { error } =>
			{
				eprintln!("Server error in receive_task: {:?}", error);
				Err(io::Error::new(ErrorKind::ConnectionReset, error))
			}
			ReceiveTaskError::Killcount { error } =>
			{
				eprintln!("Killcount error in receive_task: {:?}", error);
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

fn handle_update(
	client: &mut Client,
	update: Update,
) -> Result<(), ReceiveTaskError>
{
	match update
	{
		Update::Join {
			username,
			unlocks,
			general_chat,
		} =>
		{
			client.username = username;
			client.unlocks = unlocks;
			client.general_chat = Some(general_chat);
			Ok(())
		}

		Update::Closing =>
		{
			client.closing = true;
			client.sendbuffer.try_send(Message::Closing)?;
			Ok(())
		}
		Update::Quit =>
		{
			client.closing = true;
			client.sendbuffer.try_send(Message::Quit)?;
			Ok(())
		}

		Update::Msg(message) => handle_message(client, message),
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
			println!("Client {} gracefully disconnected.", client.id);
			return Err(ReceiveTaskError::Quit);
		}
		Message::Request { .. } | Message::JoinServer { .. }
			if client.is_unversioned() =>
		{
			eprintln!("Illegal message without version: {:?}", message);
			return Err(ReceiveTaskError::Illegal);
		}
		Message::Request { .. } if client.platform == Platform::Unknown =>
		{
			eprintln!("Illegal message without platform: {:?}", message);
			return Err(ReceiveTaskError::Illegal);
		}
		Message::Request { content: name } =>
		{
			handle_request(client, name)?;
		}
		Message::JoinServer { .. } if client.general_chat.is_some() =>
		{
			println!("Ignoring message from online client: {:?}", message);
		}
		Message::JoinServer { .. } if client.closing =>
		{
			client.sendbuffer.try_send(Message::Closing)?;
		}
		Message::JoinServer {
			status: None,
			content: Some(ref token),
			sender: Some(_),
			metadata: _,
		} if token == "%discord2018" =>
		{
			// This session code is now deprecated.
			client.sendbuffer.try_send(Message::JoinServer {
				status: Some(ResponseStatus::CredsInvalid),
				content: None,
				sender: None,
				metadata: None,
			})?;
		}
		Message::JoinServer {
			status: None,
			content: Some(token),
			sender: Some(account_id),
			metadata: _,
		} =>
		{
			let curver = Version::current();
			if client.version.major != curver.major
				|| client.version.minor != curver.minor
			{
				// Why is this LEAVE_SERVER {} and not
				// JOIN_SERVER {}? Maybe it has something
				// to do with MainMenu. Well, let's leave
				// it until we do proper error handling.
				// TODO #962
				let rejection = Message::LeaveServer { content: None };
				client.sendbuffer.try_send(rejection)?;
			}
			else
			{
				joining_server(client, token, account_id)?;
			}
		}
		Message::JoinServer { .. } =>
		{
			println!("Invalid message from client: {:?}", message);
			return Err(ReceiveTaskError::Illegal);
		}
		Message::LeaveServer { content: _ } => match client.general_chat.take()
		{
			Some(mut general_chat) =>
			{
				general_chat.try_send(chat::Update::Leave {
					client_id: client.id,
				})?;
			}
			None =>
			{
				println!("Invalid message from offline client: {:?}", message);
				return Err(ReceiveTaskError::Illegal);
			}
		},
		Message::Init => match client.general_chat
		{
			Some(ref mut general_chat) =>
			{
				if client.closing
				{
					client.sendbuffer.try_send(Message::Closing)?;
				}
				general_chat.try_send(chat::Update::Init {
					sendbuffer: client.sendbuffer.clone(),
				})?;
			}
			None =>
			{
				println!("Invalid message from offline client: {:?}", message);
				return Err(ReceiveTaskError::Illegal);
			}
		},
		Message::Chat { .. } if client.username.is_empty() =>
		{
			println!("Invalid message from client: {:?}", message);
			return Err(ReceiveTaskError::Illegal);
		}
		Message::Chat {
			content,
			sender: None,
			target: ChatTarget::General,
		} => match client.general_chat
		{
			Some(ref mut general_chat) =>
			{
				println!(
					"Client {} '{}' sent chat message: {}",
					client.id, client.username, content
				);
				general_chat.try_send(chat::Update::Msg(Message::Chat {
					content: content,
					sender: Some(client.username.clone()),
					target: ChatTarget::General,
				}))?;
			}
			None =>
			{
				let message = Message::Chat {
					content,
					sender: None,
					target: ChatTarget::General,
				};
				println!("Invalid message from offline client: {:?}", message);
				return Err(ReceiveTaskError::Illegal);
			}
		},
		// TODO lobby chat
		Message::Chat { .. } =>
		{
			println!("Invalid message from client: {:?}", message);
			return Err(ReceiveTaskError::Illegal);
		}
		Message::Closing
		| Message::Stamp { .. }
		| Message::Download { .. }
		| Message::RequestDenied { .. }
		| Message::RequestFulfilled { .. } =>
		{
			println!("Invalid message from client: {:?}", message);
			return Err(ReceiveTaskError::Illegal);
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
	println!("Client {} has version {}.", client.id, version.to_string());

	if let Some(PlatformMetadata {
		platform,
		patchmode,
	}) = metadata
	{
		client.platform = platform;
		println!("Client {} has platform {:?}.", client.id, platform);
		client.patchmode = patchmode;
		println!("Client {} has patchmode {:?}.", client.id, patchmode);
	}

	let myversion = Version::current();
	let response = Message::Version {
		version: myversion,
		metadata: None,
	};
	client.sendbuffer.try_send(response)?;

	if version.major != myversion.major || version == Version::undefined()
	{
		// The client does not have a proper version.
		return Ok(());
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

		// We treat the client as if they do not have a proper version,
		// because we do not want to receive any more messages.
		return Ok(());
	}
	else if client.closing
	{
		client.sendbuffer.try_send(Message::Quit)?;

		// We treat the client as if they do not have a proper version,
		// because we do not want to receive any more messages.
		return Ok(());
	}

	// If we got this far, the client has a proper version.
	client.has_proper_version = true;
	client
		.has_proper_version_a
		.store(true, atomic::Ordering::Relaxed);

	let epsupport = version >= Version::exact(0, 31, 1, 0);
	{
		// There might be a Future waiting for this, or there might not be.
		let _ = client.supports_empty_pulses.try_send(epsupport);
	}

	if version >= Version::exact(0, 32, 0, 0)
	{
		// TODO enable compression
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

	match client.trigger_notice.try_send(())
	{
		Ok(()) => (),
		Err(e) => eprintln!("Failed to enqueue for notice: {:?}", e),
	}

	// TODO mention patches

	Ok(())
}

fn joining_server(
	client: &mut Client,
	token: String,
	account_id: String,
) -> Result<(), ReceiveTaskError>
{
	println!(
		"Client {} is logging in with account id {}",
		client.id, &account_id
	);

	match client.login.try_send(login::Request { token, account_id })
	{
		Ok(()) => Ok(()),
		Err(error) =>
		{
			eprintln!("Failed to enqueue for login: {:?}", error);

			if error.is_full()
			{
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
			else
			{
				Err(ReceiveTaskError::Login { error })
			}
		}
	}
}

fn handle_request(
	client: &mut Client,
	name: String,
) -> Result<(), ReceiveTaskError>
{
	match client.requests.try_send(name.clone())
	{
		Ok(()) => Ok(()),
		Err(error) =>
		{
			eprintln!("Failed to enqueue for request: {:?}", error);

			if error.is_full()
			{
				let message = Message::RequestDenied {
					content: name,
					metadata: DenyMetadata {
						reason: "Too many requests.".to_string(),
					},
				};
				client.sendbuffer.try_send(message)?;
				Ok(())
			}
			else
			{
				Err(ReceiveTaskError::Notice { error })
			}
		}
	}
}

fn fulfil_request(
	downloadbuffer: mpsc::Sender<(Message, Vec<u8>)>,
	name: String,
	key: sync::Arc<PrivateKey>,
) -> impl Future<Item = Message, Error = io::Error> + Send
{
	let path = PathBuf::from(&name);
	if !is_requestable(&path)
	{
		let message = Message::RequestDenied {
			content: name,
			metadata: DenyMetadata {
				reason: "File not requestable.".to_string(),
			},
		};

		return Either::A(future::ok(message));
	}

	let future = send_file(downloadbuffer, name, path, key).map(|sentfile| {
		// TODO
		let signature = base32::encode(&sentfile.signature);

		Message::RequestFulfilled {
			content: sentfile.name.clone(),
			metadata: DownloadMetadata {
				name: Some(sentfile.name),
				offset: None,
				signature: Some(signature),
				compressed: sentfile.compressed,
				executable: sentfile.executable,
				symbolic: false,
				progressmask: None,
			},
		}
	});

	Either::B(future)
}

fn send_file(
	downloadbuffer: mpsc::Sender<(Message, Vec<u8>)>,
	name: String,
	filepath: PathBuf,
	key: sync::Arc<PrivateKey>,
) -> impl Future<Item = SentFile, Error = io::Error> + Send
{
	tokio::fs::File::open(filepath)
		.and_then(|file| file.metadata())
		.and_then(|(file, metadata)| {
			let filesize = metadata.len() as usize;
			if filesize >= SEND_FILE_SIZE_LIMIT
			{
				panic!(
					"Cannot send file of size {}, \
					 which is larger than SEND_FILE_SIZE_LIMIT.",
					filesize
				);
			}
			else if filesize >= SEND_FILE_SIZE_WARNING_LIMIT
			{
				println!("Sending very large file of size {}...", filesize);
			}

			// This is used clientside to generate the sourcefilename.
			// TODO compression
			let compressed = false;

			// This is unused since 0.32.0 but needed for earlier versions.
			// TODO implement is_file_executable?
			let executable = false;

			let chunks = chunk_file(
				file,
				name.clone(),
				filesize,
				compressed,
				executable,
			);

			send_chunks(downloadbuffer, key, chunks).map(move |signature| {
				SentFile {
					name: name,
					compressed: compressed,
					executable: executable,
					signature: signature,
				}
			})
		})
}

fn chunk_file(
	file: tokio::fs::File,
	name: String,
	filesize: usize,
	compressed: bool,
	executable: bool,
) -> impl Stream<Item = (Message, Vec<u8>), Error = io::Error> + Send
{
	stream::unfold((file, 0), move |(file, offset)| {
		if offset >= filesize
		{
			return None;
		}

		let chunksize = if offset + SEND_FILE_CHUNK_SIZE <= filesize
		{
			SEND_FILE_CHUNK_SIZE
		}
		else
		{
			filesize - offset
		};

		// This is just for aesthetics.
		let progressmask = ((0xFFFF * offset) / filesize) as u16;

		let message = Message::Download {
			content: name.clone(),
			metadata: DownloadMetadata {
				name: None,
				offset: Some(offset),
				signature: None,
				compressed: compressed,
				executable: (offset == 0 && executable),
				symbolic: false,
				progressmask: Some(progressmask),
			},
		};

		let buffer = vec![0u8; chunksize];

		Some(tokio_io::io::read_exact(file, buffer).map(
			move |(file, buffer)| {
				let chunk = (message, buffer);
				let nextstate = (file, offset + SEND_FILE_CHUNK_SIZE);
				(chunk, nextstate)
			},
		))
	})
}

fn send_chunks(
	downloadbuffer: mpsc::Sender<(Message, Vec<u8>)>,
	privatekey: sync::Arc<PrivateKey>,
	chunks: impl Stream<Item = (Message, Vec<u8>), Error = io::Error> + Send,
) -> impl Future<Item = Vec<u8>, Error = io::Error> + Send
{
	get_signer(privatekey)
		.into_future()
		.and_then(|signer| {
			send_chunks_with_signer(downloadbuffer, signer, chunks)
		})
		.and_then(|signer| {
			signer
				.sign_to_vec()
				.map_err(|error| io::Error::new(ErrorKind::Other, error))
		})
}

fn get_signer(privatekey: sync::Arc<PrivateKey>) -> Result<Signer, io::Error>
{
	owning_ref::OwningHandle::try_new(privatekey, |privatekey| {
		openssl::sign::Signer::new(
			openssl::hash::MessageDigest::sha512(),
			unsafe { privatekey.as_ref().unwrap() },
		)
		.map_err(|error| io::Error::new(ErrorKind::Other, error))
		.map(|signer| Box::new(signer))
	})
}

type Signer = owning_ref::OwningHandle<
	sync::Arc<PrivateKey>,
	Box<openssl::sign::Signer<'static>>,
>;

fn send_chunks_with_signer(
	downloadbuffer: mpsc::Sender<(Message, Vec<u8>)>,
	signer: Signer,
	chunks: impl Stream<Item = (Message, Vec<u8>), Error = io::Error> + Send,
) -> impl Future<Item = Signer, Error = io::Error> + Send
{
	chunks
		.fold((signer, downloadbuffer), |state, (message, buffer)| {
			let (mut signer, downloadbuffer) = state;
			let signing = signer
				.update(&buffer)
				.map_err(|error| io::Error::new(ErrorKind::Other, error))
				.into_future();
			let downloading = downloadbuffer
				.send((message, buffer))
				.map_err(|error| {
					eprintln!("Send error while sending chunks: {:?}", error);
					io::Error::new(ErrorKind::ConnectionReset, error)
				})
				.map(|downloadbuffer| (signer, downloadbuffer));
			signing.and_then(|()| downloading)
		})
		.map(|(signer, _downloadbuffer)| signer)
}

struct SentFile
{
	name: String,
	compressed: bool,
	executable: bool,
	signature: Vec<u8>,
}
