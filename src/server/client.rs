/* Server::Client */

use crate::common::keycode::Keycode;
use crate::common::version::*;
use crate::server::chat;
use crate::server::game;
use crate::server::lobby;
use crate::server::login;
use crate::server::login::Unlock;
use crate::server::login::UserId;
use crate::server::message::*;
use crate::server::rating;
use crate::server::tokio::State as ServerState;

use std::fmt;
use std::io;
use std::io::ErrorKind;
use std::sync;
use std::sync::atomic;

use futures::future;
use futures::future::Either;
use futures::{pin_mut, select};
use futures::{FutureExt, StreamExt, TryFutureExt};

use tokio::io::{ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::watch;
use tokio::time as timer;
use tokio::time::{Duration, Instant};

use enumset::EnumSet;

const MESSAGE_SIZE_LIMIT: usize = 524288;
const MESSAGE_SIZE_UNVERSIONED_LIMIT: usize = 201;
const MESSAGE_SIZE_WARNING_LIMIT: usize = 65537;

struct Client
{
	sendbuffer: mpsc::Sender<Message>,
	last_receive_time: watch::Sender<()>,
	pong_receive_time: Option<oneshot::Sender<()>>,
	ping_tolerance: watch::Sender<Duration>,
	login: mpsc::Sender<login::Request>,
	general_chat_reserve: Option<mpsc::Sender<chat::Update>>,
	general_chat: Option<mpsc::Sender<chat::Update>>,
	general_chat_callback: Option<mpsc::Sender<Update>>,
	rating_database: mpsc::Sender<rating::Update>,
	latest_rating_data: Option<rating::Data>,
	canary_for_lobbies: mpsc::Sender<()>,
	lobby_authority: sync::Arc<atomic::AtomicU64>,
	lobby_callback: Option<mpsc::Sender<Update>>,
	lobby: Option<mpsc::Sender<lobby::Update>>,
	has_proper_version: bool,
	has_proper_version_a: sync::Arc<atomic::AtomicBool>,

	pub id: Keycode,
	pub user_id: Option<UserId>,
	pub username: String,
	pub version: Version,
	pub unlocks: EnumSet<Unlock>,

	closing: bool,
}

impl Drop for Client
{
	fn drop(&mut self)
	{
		let mut general_chat = self.general_chat.take();

		match general_chat
		{
			Some(ref mut general_chat) =>
			{
				let update = chat::Update::Leave { client_id: self.id };
				match general_chat.try_send(update)
				{
					Ok(()) => (),
					Err(e) => eprintln!("Error while dropping client: {:?}", e),
				}
			}
			None =>
			{}
		}

		match self.lobby.take()
		{
			Some(mut lobby) => match general_chat
			{
				Some(ref general_chat) =>
				{
					let update = lobby::Update::Leave {
						client_id: self.id,
						general_chat: general_chat.clone(),
					};
					match lobby.try_send(update)
					{
						Ok(()) => (),
						Err(e) =>
						{
							eprintln!("Error while dropping client: {:?}", e)
						}
					}
				}
				None =>
				{
					eprintln!("Expected general_chat when dropping lobby");
				}
			},
			None =>
			{}
		}
	}
}

pub fn accept(
	socket: TcpStream,
	id: Keycode,
	login_server: sync::Arc<login::Server>,
	chat_server: mpsc::Sender<chat::Update>,
	rating_database: mpsc::Sender<rating::Update>,
	server_state: watch::Receiver<ServerState>,
	canary: mpsc::Sender<()>,
	lobby_authority: sync::Arc<atomic::AtomicU64>,
)
{
	let (sendbuffer_in, sendbuffer_out) = mpsc::channel::<Message>(1000);
	let sendbuffer_pulse = sendbuffer_in.clone();
	let sendbuffer_login = sendbuffer_in.clone();
	let (updatebuffer_in, updatebuffer_out) = mpsc::channel::<Update>(10);
	let updatebuffer_ping = updatebuffer_in.clone();
	let updatebuffer_chat = updatebuffer_in.clone();
	let updatebuffer_lobby = updatebuffer_in.clone();
	let tolerance = Duration::from_secs(120);
	let (pingtolerance_in, pingtolerance_out) = watch::channel(tolerance);
	let (timebuffer_in, timebuffer_out) = watch::channel(());
	let (loginbuffer_in, loginbuffer_out) = mpsc::channel::<login::Request>(1);
	let (reader, writer) = tokio::io::split(socket);
	let canary_for_lobbies = canary.clone();

	let client = Client {
		sendbuffer: sendbuffer_in,
		last_receive_time: timebuffer_in,
		pong_receive_time: None,
		ping_tolerance: pingtolerance_in,
		login: loginbuffer_in,
		general_chat_reserve: Some(chat_server),
		general_chat: None,
		general_chat_callback: Some(updatebuffer_chat),
		rating_database,
		latest_rating_data: None,
		lobby_authority: lobby_authority,
		lobby_callback: Some(updatebuffer_lobby),
		canary_for_lobbies,
		lobby: None,
		has_proper_version: false,
		has_proper_version_a: sync::Arc::new(atomic::AtomicBool::new(false)),

		id: id,
		user_id: None,
		username: String::new(),
		version: Version::undefined(),
		unlocks: EnumSet::empty(),

		closing: false,
	};

	let receive_task =
		start_receive_task(client, updatebuffer_out, server_state, reader);
	let send_task = start_send_task(id, sendbuffer_out, writer);
	let ping_task = start_ping_task(
		id,
		updatebuffer_ping,
		timebuffer_out,
		pingtolerance_out,
	);
	let pulse_task = start_pulse_task(sendbuffer_pulse);
	let login_task = start_login_task(
		sendbuffer_login,
		updatebuffer_in,
		loginbuffer_out,
		login_server,
	);

	// The support task cannot finish because the pulse_task never finishes,
	// although one of the tasks might return an error.
	let support_task = future::try_join3(ping_task, pulse_task, login_task)
		.map_ok(|((), (), ())| ())
		.fuse();

	// The main task finishes as soon as the receive task does, aborting the
	// support task in the process.
	let main_task = async {
		pin_mut!(receive_task);
		pin_mut!(support_task);
		future::select(receive_task, support_task)
			.map(|either| match either
			{
				Either::Left((result, _support_task)) => result,
				Either::Right((Err(error), _)) => Err(error),
				Either::Right((Ok(()), _)) => Err(Error::Unexpected),
			})
			.await
	};

	// If the receive task finishes, the send task will eventually run out
	// of things to send and finish as well.
	let task = future::try_join(main_task, send_task)
		.map_ok(|((), ())| ())
		.map_err(move |e| eprintln!("Error in client {}: {:?}", id, e))
		.map(move |_result| {
			println!("Client {} has disconnected.", id);
			let _discarded = canary;
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
		has_quit = handle_update(&mut client, update).await?;
	}

	println!("Client {} stopped receiving.", client.id);
	Ok(())
}

async fn receive_message(
	socket: &mut ReadHalf<TcpStream>,
	versioned: bool,
) -> Result<Update, Error>
{
	let length = socket.read_u32().await?;
	if length == 0
	{
		/*verbose*/
		println!("Received pulse.");

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
	mut sendbuffer: mpsc::Sender<Update>,
	mut activity: watch::Receiver<()>,
	mut ping_tolerance: watch::Receiver<Duration>,
) -> Result<(), Error>
{
	loop
	{
		let (callback_in, callback_out) = oneshot::channel::<()>();
		let request = Update::PingTaskRequestsPing {
			callback: callback_in,
		};
		sendbuffer.send(request).await?;

		wait_for_pong(client_id, callback_out, &mut ping_tolerance).await?;

		wait_for_inactivity(&mut activity, &mut ping_tolerance).await?;
	}
}

async fn wait_for_inactivity(
	activity: &mut watch::Receiver<()>,
	trigger: &mut watch::Receiver<Duration>,
) -> Result<(), Error>
{
	loop
	{
		let threshold = Duration::from_secs(5);
		let activity_event = activity.recv().map(|x| match x
		{
			Some(()) => Ok(PingEvent::Activity),
			None => Err(Error::Unexpected),
		});
		let trigger_event = trigger.recv().map(|x| match x
		{
			Some(_) => Ok(PingEvent::Forced),
			None => Err(Error::Unexpected),
		});
		let timeout_event =
			timer::delay_for(threshold).map(|()| PingEvent::Timeout);
		let event = select! {
			x = activity_event.fuse() => x?,
			x = trigger_event.fuse() => x?,
			x = timeout_event.fuse() => x,
		};
		match event
		{
			PingEvent::Activity => continue,
			PingEvent::Forced => return Ok(()),
			PingEvent::Timeout => return Ok(()),
		}
	}
}

async fn wait_for_pong(
	client_id: Keycode,
	callback: oneshot::Receiver<()>,
	tolerance_updates: &mut watch::Receiver<Duration>,
) -> Result<(), Error>
{
	let sendtime = Instant::now();
	let mut tolerance: Duration = *tolerance_updates.borrow();

	let mut received_event = callback.map_ok(|()| PongEvent::Received).fuse();
	loop
	{
		let tolerance_event = tolerance_updates.recv().map(|x| match x
		{
			Some(value) => Ok(PongEvent::NewTolerance { value }),
			None => Err(Error::Unexpected),
		});
		let timeout_event = timer::delay_until(sendtime + tolerance)
			.map(|()| PongEvent::Timeout);
		let event = select! {
			x = tolerance_event.fuse() => x?,
			x = received_event => x?,
			x = timeout_event.fuse() => x,
		};
		match event
		{
			PongEvent::NewTolerance { value } =>
			{
				tolerance = value;
			}
			PongEvent::Timeout =>
			{
				eprintln!("Disconnecting inactive client {}", client_id);
				// TODO slack
				return Err(Error::Timeout);
			}
			PongEvent::Received => break,
		}
	}

	println!(
		"Client {} has {}ms ping",
		client_id,
		sendtime.elapsed().as_millis()
	);

	Ok(())
}

enum PingEvent
{
	Activity,
	Timeout,
	Forced,
}

enum PongEvent
{
	NewTolerance
	{
		value: Duration,
	},
	Timeout,
	Received,
}

async fn start_pulse_task(
	mut sendbuffer: mpsc::Sender<Message>,
) -> Result<(), Error>
{
	let start = Instant::now() + Duration::from_secs(4);
	let mut interval = timer::interval_at(start, Duration::from_secs(4));

	loop
	{
		interval.tick().await;
		sendbuffer.send(Message::Pulse).await?;
	}
}

async fn start_login_task(
	mut sendbuffer: mpsc::Sender<Message>,
	mut joinedbuffer: mpsc::Sender<Update>,
	mut requestbuffer: mpsc::Receiver<login::Request>,
	login_server: sync::Arc<login::Server>,
) -> Result<(), Error>
{
	while let Some(request) = requestbuffer.recv().await
	{
		match login_server.login(request).await
		{
			Ok(logindata) =>
			{
				if logindata.unlocks.contains(Unlock::BetaAccess)
				{
					let update = Update::LoggedIn {
						user_id: logindata.user_id,
						username: logindata.username,
						unlocks: logindata.unlocks,
						rating_data: logindata.rating_data,
					};
					joinedbuffer.send(update).await?;
				}
				else
				{
					println!("Login failed due to insufficient access");
					let message = Message::JoinServer {
						status: Some(ResponseStatus::KeyRequired),
						content: None,
						sender: None,
						metadata: None,
					};
					sendbuffer.send(message).await?;
				}
			}
			Err(responsestatus) =>
			{
				eprintln!("Login failed with {:?}", responsestatus);
				let message = Message::JoinServer {
					status: Some(responsestatus),
					content: None,
					sender: None,
					metadata: None,
				};
				sendbuffer.send(message).await?;
			}
		}
	}

	Ok(())
}

#[derive(Debug)]
pub enum Update
{
	PingTaskRequestsPing
	{
		callback: oneshot::Sender<()>,
	},
	BeingGhostbusted,
	LoggedIn
	{
		user_id: UserId,
		username: String,
		unlocks: EnumSet<Unlock>,
		rating_data: rating::Data,
	},
	JoinedServer,
	LobbyFound
	{
		lobby_id: Keycode,
		lobby_sendbuffer: mpsc::Sender<lobby::Update>,
		general_chat: mpsc::Sender<chat::Update>,
	},
	LobbyNotFound
	{
		lobby_id: Keycode,
	},
	JoinedLobby
	{
		lobby: mpsc::Sender<lobby::Update>,
	},
	Closing,
	Closed,
	Msg(Message),
}

#[derive(Debug)]
enum Error
{
	Illegal,
	Timeout,
	Unexpected,
	Send
	{
		error: mpsc::error::SendError<Message>,
	},
	TrySend
	{
		error: mpsc::error::TrySendError<Message>,
	},
	Update
	{
		error: mpsc::error::SendError<Update>,
	},
	Chat
	{
		error: mpsc::error::SendError<chat::Update>,
	},
	Lobby
	{
		error: mpsc::error::SendError<lobby::Update>,
	},
	Login
	{
		error: mpsc::error::TrySendError<login::Request>,
	},
	Rating
	{
		error: mpsc::error::SendError<rating::Update>,
	},
	Tolerance
	{
		error: watch::error::SendError<Duration>,
	},
	Watch
	{
		error: watch::error::SendError<()>,
	},
	Recv
	{
		error: mpsc::error::RecvError,
	},
	OneshotRecv
	{
		error: oneshot::error::RecvError,
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

impl From<mpsc::error::SendError<Update>> for Error
{
	fn from(error: mpsc::error::SendError<Update>) -> Self
	{
		Error::Update { error }
	}
}

impl From<mpsc::error::SendError<chat::Update>> for Error
{
	fn from(error: mpsc::error::SendError<chat::Update>) -> Self
	{
		Error::Chat { error }
	}
}

impl From<mpsc::error::SendError<lobby::Update>> for Error
{
	fn from(error: mpsc::error::SendError<lobby::Update>) -> Self
	{
		Error::Lobby { error }
	}
}

impl From<mpsc::error::TrySendError<login::Request>> for Error
{
	fn from(error: mpsc::error::TrySendError<login::Request>) -> Self
	{
		Error::Login { error }
	}
}

impl From<mpsc::error::SendError<rating::Update>> for Error
{
	fn from(error: mpsc::error::SendError<rating::Update>) -> Self
	{
		Error::Rating { error }
	}
}

impl From<watch::error::SendError<Duration>> for Error
{
	fn from(error: watch::error::SendError<Duration>) -> Self
	{
		Error::Tolerance { error }
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

impl From<oneshot::error::RecvError> for Error
{
	fn from(error: oneshot::error::RecvError) -> Self
	{
		Error::OneshotRecv { error }
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
			Error::Timeout => write!(f, "Failed to respond"),
			Error::Unexpected => write!(f, "Something unexpected happened"),
			Error::Send { error } => error.fmt(f),
			Error::TrySend { error } => error.fmt(f),
			Error::Update { error } => error.fmt(f),
			Error::Chat { error } => error.fmt(f),
			Error::Lobby { error } => error.fmt(f),
			Error::Login { error } => error.fmt(f),
			Error::Rating { error } => error.fmt(f),
			Error::Tolerance { error } => error.fmt(f),
			Error::Watch { error } => error.fmt(f),
			Error::Recv { error } => error.fmt(f),
			Error::OneshotRecv { error } => error.fmt(f),
			Error::Io { error } => error.fmt(f),
		}
	}
}

async fn handle_update(
	client: &mut Client,
	update: Update,
) -> Result<bool, Error>
{
	match update
	{
		Update::PingTaskRequestsPing { callback } =>
		{
			client.pong_receive_time = Some(callback);
			client.sendbuffer.try_send(Message::Ping)?;
			Ok(false)
		}
		Update::BeingGhostbusted =>
		{
			let tolerance = Duration::from_secs(5);
			client.ping_tolerance.broadcast(tolerance)?;
			Ok(false)
		}

		Update::LoggedIn {
			user_id,
			username,
			unlocks,
			rating_data,
		} =>
		{
			client.user_id = Some(user_id);
			client.username = username;
			client.unlocks = unlocks;
			client.latest_rating_data = Some(rating_data);

			match &mut client.general_chat_reserve
			{
				Some(chat) =>
				{
					let callback = match &client.general_chat_callback
					{
						Some(callback) => callback.clone(),
						None =>
						{
							eprintln!("Expected lobby_callback");
							return Err(Error::Unexpected);
						}
					};
					let request = chat::Update::Join {
						client_id: client.id,
						username: client.username.clone(),
						unlocks: client.unlocks.clone(),
						sendbuffer: client.sendbuffer.clone(),
						callback,
					};
					match chat.try_send(request)
					{
						Ok(()) => Ok(false),
						Err(error) =>
						{
							eprintln!(
								"Client {} failed to join chat: {:?}",
								client.id, error
							);
							// If the chat cannot handle more updates, it is
							// probably too busy to handle more clients.
							// TODO better error handling (#962)
							let message = Message::JoinServer {
								status: Some(ResponseStatus::UnknownError),
								content: None,
								sender: None,
								metadata: None,
							};
							client.sendbuffer.try_send(message)?;
							Ok(false)
						}
					}
				}
				None =>
				{
					client.sendbuffer.try_send(Message::Closing)?;
					Ok(false)
				}
			}
		}
		Update::JoinedServer => match client.general_chat_reserve.take()
		{
			Some(chat) =>
			{
				if let Some(data) = client.latest_rating_data.take()
				{
					let user_id = match client.user_id
					{
						Some(user_id) => user_id,
						None =>
						{
							eprintln!("Expected user_id");
							return Err(Error::Unexpected);
						}
					};
					let update = rating::Update::Fresh { user_id, data };
					client.rating_database.send(update).await?;
				}

				client.general_chat = Some(chat);
				Ok(false)
			}
			None =>
			{
				client.sendbuffer.try_send(Message::Closing)?;
				Ok(false)
			}
		},

		Update::LobbyFound {
			lobby_id: _,
			mut lobby_sendbuffer,
			general_chat,
		} =>
		{
			let lobby_callback = match &client.lobby_callback
			{
				Some(callback) => callback.clone(),
				None =>
				{
					eprintln!("Expected lobby_callback");
					return Err(Error::Unexpected);
				}
			};
			let client_user_id = match client.user_id
			{
				Some(user_id) => user_id,
				None =>
				{
					eprintln!("Expected user_id");
					return Err(Error::Unexpected);
				}
			};
			let update = lobby::Update::Join {
				client_id: client.id,
				client_user_id,
				client_username: client.username.clone(),
				client_sendbuffer: client.sendbuffer.clone(),
				client_callback: lobby_callback,
				lobby_sendbuffer: lobby_sendbuffer.clone(),
				general_chat,
			};
			lobby_sendbuffer.send(update).await?;
			Ok(false)
		}
		Update::LobbyNotFound { lobby_id: _ } =>
		{
			client.sendbuffer.try_send(Message::JoinLobby {
				lobby_id: None,
				username: None,
				metadata: None,
			})?;
			Ok(false)
		}
		Update::JoinedLobby { lobby } =>
		{
			// TODO what else to do here?
			client.lobby = Some(lobby);
			Ok(false)
		}

		Update::Closing =>
		{
			client.closing = true;
			client.general_chat_reserve.take();
			client.lobby_callback.take();
			client.sendbuffer.try_send(Message::Closing)?;
			Ok(false)
		}
		Update::Closed =>
		{
			client.closing = true;
			client.general_chat_reserve.take();
			client.lobby_callback.take();
			client.sendbuffer.try_send(Message::Closed)?;
			Ok(false)
		}

		Update::Msg(message) => handle_message(client, message).await,
	}
}

async fn handle_message(
	client: &mut Client,
	message: Message,
) -> Result<bool, Error>
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
			if let Some(one) = client.pong_receive_time.take()
			{
				one.send(()).map_err(|()| Error::Unexpected)?;
			}

			if let Some(chat) = &mut client.general_chat
			{
				let update = chat::Update::StillAlive {
					client_id: client.id,
				};
				chat.send(update).await?
			}
		}
		Message::Version { version } =>
		{
			greet_client(client, version)?;
		}
		Message::Quit =>
		{
			println!("Client {} is gracefully disconnecting...", client.id);
			client.sendbuffer.try_send(Message::Quit)?;
			return Ok(true);
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
			content: Some(token),
			sender: Some(account_id_as_string),
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
				let request = login::Request {
					token,
					account_id_as_string,
				};
				joining_server(client, request)?;
			}
		}
		Message::JoinServer { .. } =>
		{
			println!("Invalid message from client: {:?}", message);
			return Err(Error::Illegal);
		}
		Message::LeaveServer { content: _ } => match client.general_chat.take()
		{
			Some(mut general_chat) =>
			{
				match client.lobby.take()
				{
					Some(ref mut lobby) =>
					{
						let update = lobby::Update::Leave {
							client_id: client.id,
							general_chat: general_chat.clone(),
						};
						lobby.send(update).await?;
					}
					None =>
					{}
				}

				let update = chat::Update::Leave {
					client_id: client.id,
				};
				general_chat.send(update).await?;

				if !client.closing
				{
					client.general_chat_reserve = Some(general_chat);
				}
			}
			None =>
			{
				// When the client is ghostbusting, if they exit the game
				// it sends a LEAVE_SERVER message. We just ignore it.
			}
		},
		Message::JoinLobby { .. } if client.closing =>
		{
			client.sendbuffer.try_send(Message::Closing)?;
		}
		Message::JoinLobby {
			lobby_id: Some(lobby_id),
			username: None,
			metadata: _,
		} => match client.general_chat
		{
			Some(ref mut general_chat) =>
			{
				// TODO refuse if already in lobby

				let lobby_callback = match &client.lobby_callback
				{
					Some(callback) => callback.clone(),
					None =>
					{
						eprintln!("Expected lobby_callback");
						return Err(Error::Unexpected);
					}
				};

				let update = chat::Update::FindLobby {
					lobby_id,
					callback: lobby_callback,
					general_chat: general_chat.clone(),
				};
				general_chat.send(update).await?;
			}
			None =>
			{
				println!("Invalid message from offline client: {:?}", message);
				return Err(Error::Illegal);
			}
		},
		Message::JoinLobby { .. } =>
		{
			println!("Invalid message from client: {:?}", message);
			return Err(Error::Illegal);
		}
		Message::LeaveLobby {
			lobby_id: None,
			username: None,
		} => match client.lobby.take()
		{
			Some(ref mut lobby) =>
			{
				let general_chat = match &client.general_chat
				{
					Some(general_chat) => general_chat.clone(),
					None =>
					{
						eprintln!("Expected general_chat");
						return Err(Error::Unexpected);
					}
				};

				let update = lobby::Update::Leave {
					client_id: client.id,
					general_chat: general_chat,
				};
				lobby.send(update).await?;
			}
			None =>
			{
				println!(
					"Invalid message from unlobbied client: {:?}",
					message
				);
				return Err(Error::Illegal);
			}
		},
		Message::LeaveLobby { .. } =>
		{
			println!("Invalid message from client: {:?}", message);
			return Err(Error::Illegal);
		}
		Message::MakeLobby {} if client.closing =>
		{
			client.sendbuffer.try_send(Message::Closing)?;
		}
		Message::MakeLobby {} if client.lobby.is_some() =>
		{
			println!("Ignoring message from lobbied client: {:?}", message);
		}
		Message::MakeLobby {} => match client.general_chat
		{
			Some(ref general_chat) =>
			{
				let lobby_callback = match &client.lobby_callback
				{
					Some(callback) => callback.clone(),
					None =>
					{
						eprintln!("Expected lobby_callback");
						return Err(Error::Unexpected);
					}
				};
				let mut lobby = lobby::create(
					&mut client.lobby_authority,
					client.rating_database.clone(),
					client.canary_for_lobbies.clone(),
				);

				let client_user_id = match client.user_id
				{
					Some(user_id) => user_id,
					None =>
					{
						eprintln!("Expected user_id");
						return Err(Error::Unexpected);
					}
				};
				let update = lobby::Update::Join {
					client_id: client.id,
					client_user_id,
					client_username: client.username.clone(),
					client_sendbuffer: client.sendbuffer.clone(),
					client_callback: lobby_callback,
					lobby_sendbuffer: lobby.clone(),
					general_chat: general_chat.clone(),
				};
				lobby.send(update).await?;
				client.lobby = Some(lobby);
			}
			None =>
			{
				println!("Invalid message from offline client: {:?}", message);
				return Err(Error::Illegal);
			}
		},
		Message::SaveLobby {} if client.closing =>
		{
			client.sendbuffer.try_send(Message::Closing)?;
		}
		Message::SaveLobby {} => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let general_chat = match &client.general_chat
				{
					Some(general_chat) => general_chat.clone(),
					None =>
					{
						eprintln!("Expected general_chat");
						return Err(Error::Unexpected);
					}
				};

				let update = lobby::Update::Save {
					lobby_sendbuffer: lobby.clone(),
					general_chat,
				};
				lobby.send(update).await?;
			}
			None =>
			{
				println!(
					"Invalid message from unlobbied client: {:?}",
					message
				);
				return Err(Error::Illegal);
			}
		},
		Message::LockLobby {} => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let general_chat = match &client.general_chat
				{
					Some(general_chat) => general_chat.clone(),
					None =>
					{
						eprintln!("Expected general_chat");
						return Err(Error::Unexpected);
					}
				};

				let update = lobby::Update::Lock { general_chat };
				lobby.send(update).await?;
			}
			None =>
			{
				println!(
					"Invalid message from unlobbied client: {:?}",
					message
				);
				return Err(Error::Illegal);
			}
		},
		Message::UnlockLobby {} => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let general_chat = match &client.general_chat
				{
					Some(general_chat) => general_chat.clone(),
					None =>
					{
						eprintln!("Expected general_chat");
						return Err(Error::Unexpected);
					}
				};

				let update = lobby::Update::Unlock { general_chat };
				lobby.send(update).await?;
			}
			None =>
			{
				println!(
					"Invalid message from unlobbied client: {:?}",
					message
				);
				return Err(Error::Illegal);
			}
		},
		Message::NameLobby { lobby_name } => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let general_chat = match &client.general_chat
				{
					Some(general_chat) => general_chat.clone(),
					None =>
					{
						eprintln!("Expected general_chat");
						return Err(Error::Unexpected);
					}
				};

				let update = lobby::Update::Rename {
					lobby_name,
					general_chat,
				};
				lobby.send(update).await?;
			}
			None =>
			{
				println!("Invalid NameLobby message from unlobbied client");
				return Err(Error::Illegal);
			}
		},
		Message::ClaimRole { username, role } => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let general_chat = match &client.general_chat
				{
					Some(general_chat) => general_chat.clone(),
					None =>
					{
						eprintln!("Expected general_chat");
						return Err(Error::Unexpected);
					}
				};

				let update = lobby::Update::ClaimRole {
					general_chat,
					username,
					role,
				};
				lobby.send(update).await?;
			}
			None =>
			{
				println!("Invalid ClaimRole message from unlobbied client");
				return Err(Error::Illegal);
			}
		},
		Message::ClaimColor {
			username_or_slot,
			color,
		} => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let update = lobby::Update::ClaimColor {
					username_or_slot,
					color,
				};
				lobby.send(update).await?;
			}
			None =>
			{
				println!("Invalid ClaimColor message from unlobbied client");
				return Err(Error::Illegal);
			}
		},
		Message::ClaimVisionType {
			username_or_slot,
			visiontype,
		} => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let update = lobby::Update::ClaimVisionType {
					username_or_slot,
					visiontype,
				};
				lobby.send(update).await?;
			}
			None =>
			{
				println!("Invalid vision message from unlobbied client");
				return Err(Error::Illegal);
			}
		},
		Message::ClaimAi { slot, ai_name } => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let update = lobby::Update::ClaimAi { slot, ai_name };
				lobby.send(update).await?;
			}
			None =>
			{
				println!("Invalid ClaimAi message from unlobbied client");
				return Err(Error::Illegal);
			}
		},
		Message::ClaimDifficulty { slot, difficulty } => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let update =
					lobby::Update::ClaimDifficulty { slot, difficulty };
				lobby.send(update).await?;
			}
			None =>
			{
				println!("Invalid ClaimAi message from unlobbied client");
				return Err(Error::Illegal);
			}
		},
		Message::PickMap { map_name } => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let general_chat = match &client.general_chat
				{
					Some(general_chat) => general_chat.clone(),
					None =>
					{
						eprintln!("Expected general_chat");
						return Err(Error::Unexpected);
					}
				};

				let update = lobby::Update::PickMap {
					general_chat,
					map_name,
				};
				lobby.send(update).await?;
			}
			None =>
			{
				println!("Invalid PickMap message from unlobbied client");
				return Err(Error::Illegal);
			}
		},
		Message::PickTimer { seconds } => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let update = lobby::Update::PickTimer { seconds };
				lobby.send(update).await?;
			}
			None =>
			{
				println!("Invalid PickTimer message from unlobbied client");
				return Err(Error::Illegal);
			}
		},
		Message::PickRuleset { ruleset_name } => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let update = lobby::Update::PickRuleset { ruleset_name };
				lobby.send(update).await?;
			}
			None =>
			{
				println!("Invalid PickRuleset message from unlobbied client");
				return Err(Error::Illegal);
			}
		},
		Message::ListRuleset { ruleset_name } => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let general_chat = match &client.general_chat
				{
					Some(general_chat) => general_chat.clone(),
					None =>
					{
						eprintln!("Expected general_chat");
						return Err(Error::Unexpected);
					}
				};

				let update = lobby::Update::ConfirmRuleset {
					client_id: client.id,
					general_chat,
					ruleset_name,
					lobby_sendbuffer: lobby.clone(),
				};
				lobby.send(update).await?;
			}
			None =>
			{
				println!("Invalid ListRuleset message from unlobbied client");
				return Err(Error::Illegal);
			}
		},
		Message::AddBot { slot: None } => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let general_chat = match &client.general_chat
				{
					Some(general_chat) => general_chat.clone(),
					None =>
					{
						eprintln!("Expected general_chat");
						return Err(Error::Unexpected);
					}
				};

				let update = lobby::Update::AddBot { general_chat };
				lobby.send(update).await?;
			}
			None =>
			{
				println!("Invalid AddBot message from unlobbied client");
				return Err(Error::Illegal);
			}
		},
		Message::AddBot { .. } =>
		{
			println!("Invalid message from client: {:?}", message);
			return Err(Error::Illegal);
		}
		Message::RemoveBot { slot } => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let general_chat = match &client.general_chat
				{
					Some(general_chat) => general_chat.clone(),
					None =>
					{
						eprintln!("Expected general_chat");
						return Err(Error::Unexpected);
					}
				};

				let update = lobby::Update::RemoveBot { general_chat, slot };
				lobby.send(update).await?;
			}
			None =>
			{
				println!("Invalid RemoveBot message from unlobbied client");
				return Err(Error::Illegal);
			}
		},
		Message::Game {
			role: None,
			player: None,
			ruleset_name: None,
			timer_in_seconds: None,
		} => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let general_chat = match &client.general_chat
				{
					Some(general_chat) => general_chat.clone(),
					None =>
					{
						eprintln!("Expected general_chat");
						return Err(Error::Unexpected);
					}
				};

				let update = lobby::Update::Start {
					general_chat,
					lobby_sendbuffer: lobby.clone(),
				};
				lobby.send(update).await?;
			}
			None =>
			{
				println!("Invalid Game message from unlobbied client");
				return Err(Error::Illegal);
			}
		},
		Message::Game { .. } =>
		{
			println!("Invalid message from client: {:?}", message);
			return Err(Error::Illegal);
		}
		Message::Tutorial {
			role: None,
			player: None,
			ruleset_name: None,
			timer_in_seconds: None,
		} => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let general_chat = match &client.general_chat
				{
					Some(general_chat) => general_chat.clone(),
					None =>
					{
						eprintln!("Expected general_chat");
						return Err(Error::Unexpected);
					}
				};

				let update = lobby::Update::PickTutorial {
					general_chat: general_chat.clone(),
				};
				lobby.send(update).await?;

				let update = lobby::Update::Start {
					general_chat,
					lobby_sendbuffer: lobby.clone(),
				};
				lobby.send(update).await?;
			}
			None =>
			{
				println!("Invalid Tutorial message from unlobbied client");
				return Err(Error::Illegal);
			}
		},
		Message::Tutorial { .. } =>
		{
			println!("Invalid message from client: {:?}", message);
			return Err(Error::Illegal);
		}
		Message::Challenge => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let general_chat = match &client.general_chat
				{
					Some(general_chat) => general_chat.clone(),
					None =>
					{
						eprintln!("Expected general_chat");
						return Err(Error::Unexpected);
					}
				};

				let update = lobby::Update::PickChallenge {
					general_chat: general_chat.clone(),
				};
				lobby.send(update).await?;

				let update = lobby::Update::Start {
					general_chat,
					lobby_sendbuffer: lobby.clone(),
				};
				lobby.send(update).await?;
			}
			None =>
			{
				println!("Invalid Challenge message from unlobbied client");
				return Err(Error::Illegal);
			}
		},
		Message::Resign { username: None } => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let update = game::Update::Resign {
					client_id: client.id,
				};
				let update = lobby::Update::ForwardToGame(update);
				lobby.send(update).await?
			}
			None =>
			{
				println!("Invalid Sync message from unlobbied client");
				return Err(Error::Illegal);
			}
		},
		Message::Resign { .. } =>
		{
			println!("Invalid message from client: {:?}", message);
			return Err(Error::Illegal);
		}
		Message::OrdersNew { orders } => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let update = game::Update::Orders {
					client_id: client.id,
					orders,
				};
				let update = lobby::Update::ForwardToGame(update);
				lobby.send(update).await?
			}
			None =>
			{
				println!("Invalid Sync message from unlobbied client");
				return Err(Error::Illegal);
			}
		},
		Message::Sync {
			planning_time_in_seconds: None,
		} => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let update = game::Update::Sync {
					client_id: client.id,
				};
				let update = lobby::Update::ForwardToGame(update);
				lobby.send(update).await?
			}
			None =>
			{
				println!("Invalid Sync message from unlobbied client");
				return Err(Error::Illegal);
			}
		},
		Message::Sync { .. } =>
		{
			println!("Invalid message from client: {:?}", message);
			return Err(Error::Illegal);
		}
		Message::Init => match client.general_chat
		{
			Some(ref mut general_chat) =>
			{
				if client.closing
				{
					client.sendbuffer.try_send(Message::Closing)?;
				}

				let update = chat::Update::Init {
					sendbuffer: client.sendbuffer.clone(),
				};
				general_chat.send(update).await?;
			}
			None =>
			{
				println!("Invalid message from offline client: {:?}", message);
				return Err(Error::Illegal);
			}
		},
		Message::Chat { .. } if client.username.is_empty() =>
		{
			println!("Invalid message from client: {:?}", message);
			return Err(Error::Illegal);
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

				let update = chat::Update::Msg(Message::Chat {
					content: content,
					sender: Some(client.username.clone()),
					target: ChatTarget::General,
				});
				general_chat.send(update).await?;
			}
			None =>
			{
				let message = Message::Chat {
					content,
					sender: None,
					target: ChatTarget::General,
				};
				println!("Invalid message from offline client: {:?}", message);
				return Err(Error::Illegal);
			}
		},
		Message::Chat {
			content,
			sender: None,
			target: ChatTarget::Lobby,
		} => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				println!(
					"Client {} '{}' sent lobby chat message: {}",
					client.id, client.username, content
				);

				let update = lobby::Update::Msg(Message::Chat {
					content: content,
					sender: Some(client.username.clone()),
					target: ChatTarget::Lobby,
				});
				lobby.send(update).await?;
			}
			None =>
			{
				let message = Message::Chat {
					content,
					sender: None,
					target: ChatTarget::Lobby,
				};
				println!(
					"Invalid message from unlobbied client: {:?}",
					message
				);
				return Err(Error::Illegal);
			}
		},
		Message::Chat { .. } =>
		{
			println!("Invalid message from client: {:?}", message);
			return Err(Error::Illegal);
		}
		Message::DisbandLobby { .. }
		| Message::ListLobby { .. }
		| Message::ListChallenge { .. }
		| Message::ListAi { .. }
		| Message::ListMap { .. }
		| Message::PickChallenge { .. }
		| Message::InGame { .. }
		| Message::Changes { .. }
		| Message::OrdersOld { .. }
		| Message::Closing
		| Message::Closed =>
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

	Ok(())
}

fn joining_server(
	client: &mut Client,
	request: login::Request,
) -> Result<(), Error>
{
	println!("Client {} is logging in...", client.id);

	match client.login.try_send(request)
	{
		Ok(()) => Ok(()),
		Err(mpsc::error::TrySendError::Full(_request)) =>
		{
			eprintln!("Failed to enqueue for login, login task busy.");

			// We only process one login request at a time. Does it make sense
			// to respond to a second request if the first response is still
			// underway?
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
