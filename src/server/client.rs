/* Server::Client */

mod limit;
mod login;
mod ping;
mod pulse;
mod receive;
mod send;

pub mod handle;

pub use handle::Handle;

use crate::common::keycode::Keycode;
use crate::common::version::*;
use crate::logic::ruleset;
use crate::server::chat;
use crate::server::discord_api;
use crate::server::game;
use crate::server::lobby;
use crate::server::login::Unlock;
use crate::server::login::UserId;
use crate::server::message::*;
use crate::server::rating;
use crate::server::slack_api;
use crate::server::tokio::State as ServerState;

use std::fmt;
use std::sync;
use std::sync::atomic;

use log::*;

use futures::future;
use futures::future::Either;
use futures::pin_mut;
use futures::stream;
use futures::{FutureExt, StreamExt, TryFutureExt};

use tokio::io::ReadHalf;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::watch;
use tokio::time::Duration;

use enumset::EnumSet;

struct Client
{
	sendbuffer: mpsc::Sender<Message>,
	last_receive_time: watch::Sender<()>,
	pong_receive_time: Option<oneshot::Sender<()>>,
	ping_tolerance: watch::Sender<Duration>,
	login: mpsc::Sender<login::Request>,
	slack_api: mpsc::Sender<slack_api::Post>,
	discord_api: mpsc::Sender<discord_api::Post>,
	handle: Handle,
	general_chat_reserve: Option<mpsc::Sender<chat::Update>>,
	general_chat: Option<mpsc::Sender<chat::Update>>,
	rating_database: mpsc::Sender<rating::Update>,
	data_for_rating: Option<(rating::Data, watch::Sender<rating::Data>)>,
	canary_for_lobbies: mpsc::Sender<()>,
	lobby_authority: sync::Arc<atomic::AtomicU64>,
	lobby: Option<mpsc::Sender<lobby::Update>>,
	bot_lobbies:
		std::collections::HashMap<Keycode, mpsc::Sender<lobby::Update>>,
	has_proper_version: bool,
	has_proper_version_a: sync::Arc<atomic::AtomicBool>,
	appears_active_according_to_notifications: bool,
	has_gracefully_disconnected: bool,

	pub id: Keycode,
	pub user_id: Option<UserId>,
	pub username: String,
	pub version: Version,
	pub unlocks: EnumSet<Unlock>,

	closing: bool,
}

impl Client
{
	fn is_bot(&self) -> bool
	{
		self.unlocks.contains(Unlock::Bot)
	}
}

impl Drop for Client
{
	fn drop(&mut self)
	{
		if let Some(user_id) = self.user_id
		{
			let update = rating::Update::Left { user_id };
			match self.rating_database.try_send(update)
			{
				Ok(()) => (),
				Err(e) => error!("Error while dropping client: {:?}", e),
			}
		}

		let mut general_chat = self.general_chat.take();

		match general_chat
		{
			Some(ref mut general_chat) =>
			{
				let update = chat::Update::Leave { client_id: self.id };
				match general_chat.try_send(update)
				{
					Ok(()) => (),
					Err(e) => error!("Error while dropping client: {:?}", e),
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
							error!("Error while dropping client: {:?}", e)
						}
					}
				}
				None =>
				{
					warn!("Expected general_chat when dropping lobby");
				}
			},
			None =>
			{}
		}

		if !self.bot_lobbies.is_empty()
		{
			match general_chat
			{
				Some(ref general_chat) =>
				{
					for (_, mut lobby) in self.bot_lobbies.drain()
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
								error!("Error while dropping client: {:?}", e)
							}
						}
					}
				}
				None =>
				{
					warn!("Expected general_chat when dropping lobby");
				}
			}
		}

		if self.appears_active_according_to_notifications
		{
			if self.has_gracefully_disconnected
			{
				info!("Someone was peeking. (v{})", self.version);
			}
			else if !self.username.is_empty()
			{
				info!("User '{}' crashed. (v{})", self.username, self.version);
			}
			else
			{
				info!("Someone crashed. (v{})", self.version);
			}
		}

		// We do nothing with slack_api, but the fact that each client has a
		// reference to it means we won't see "Server stopped" until all
		// clients have disconnected.
		let _ignored = &self.slack_api;
	}
}

pub fn accept(
	socket: TcpStream,
	id: Keycode,
	login_server: sync::Arc<login::Server>,
	chat_server: mpsc::Sender<chat::Update>,
	rating_database: mpsc::Sender<rating::Update>,
	slack_api: mpsc::Sender<slack_api::Post>,
	discord_api: mpsc::Sender<discord_api::Post>,
	server_state: watch::Receiver<ServerState>,
	canary: mpsc::Sender<()>,
	lobby_authority: sync::Arc<atomic::AtomicU64>,
)
{
	let (sendbuffer_in, sendbuffer_out) = mpsc::channel::<Message>(10000);
	let sendbuffer_pulse = sendbuffer_in.clone();
	let sendbuffer_login = sendbuffer_in.clone();
	let sendbuffer_handle = sendbuffer_in.clone();
	let (pingbuffer_in, pingbuffer_out) = mpsc::channel::<ping::Request>(1);
	let (updatebuffer_in, updatebuffer_out) = mpsc::channel::<Update>(10);
	let (poison_in, poison_out) = mpsc::channel::<handle::Poison>(1);
	let tolerance = Duration::from_secs(120);
	let (pingtolerance_in, pingtolerance_out) = watch::channel(tolerance);
	let (timebuffer_in, timebuffer_out) = watch::channel(());
	let (logindata_in, logindata_out) = mpsc::channel::<login::LoginData>(1);
	let (login_in, login_out) = mpsc::channel::<login::Request>(1);
	let (reader, writer) = tokio::io::split(socket);
	let canary_for_lobbies = canary.clone();

	let handle = Handle::Connected {
		id,
		sendbuffer: sendbuffer_handle,
		update_callback: updatebuffer_in,
		poison_callback: poison_in,
		salts: None,
	};

	let client = Client {
		sendbuffer: sendbuffer_in,
		last_receive_time: timebuffer_in,
		pong_receive_time: None,
		ping_tolerance: pingtolerance_in,
		login: login_in,
		slack_api,
		discord_api,
		handle,
		general_chat_reserve: Some(chat_server),
		general_chat: None,
		rating_database,
		data_for_rating: None,
		lobby_authority: lobby_authority,
		canary_for_lobbies,
		lobby: None,
		bot_lobbies: std::collections::HashMap::new(),
		has_proper_version: false,
		has_proper_version_a: sync::Arc::new(atomic::AtomicBool::new(false)),
		appears_active_according_to_notifications: false,
		has_gracefully_disconnected: false,

		id: id,
		user_id: None,
		username: String::new(),
		version: Version::undefined(),
		unlocks: EnumSet::empty(),

		closing: false,
	};

	let receive_task = start_receive_task(
		client,
		pingbuffer_out,
		logindata_out,
		updatebuffer_out,
		poison_out,
		server_state,
		reader,
	);
	let send_task = send::run(id, sendbuffer_out, writer).map_err(|e| e.into());
	let ping_task =
		ping::run(id, pingbuffer_in, timebuffer_out, pingtolerance_out)
			.map_err(|error| error.into());
	let pulse_task = pulse::run(sendbuffer_pulse).map_err(|e| e.into());
	let login_task =
		login::run(sendbuffer_login, logindata_in, login_out, login_server)
			.map_err(|error| error.into());

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
		.map_err(move |e| error!("Error in client {}: {:?}", id, e))
		.map(move |_result| {
			info!("Client {} has disconnected.", id);
			let _discarded = canary;
		});

	tokio::spawn(task);
}

async fn start_receive_task(
	mut client: Client,
	ping_requests: mpsc::Receiver<ping::Request>,
	login_results: mpsc::Receiver<login::LoginData>,
	server_updates: mpsc::Receiver<Update>,
	poison: mpsc::Receiver<handle::Poison>,
	server_state: watch::Receiver<ServerState>,
	socket: ReadHalf<TcpStream>,
) -> Result<(), Error>
{
	let ping_updates =
		ping_requests.map(|request| Update::PingTaskRequestsPing {
			callback: request.callback,
		});
	let login_updates = login_results.map(|data| Update::LoggedIn {
		user_id: data.user_id,
		username: data.username,
		unlocks: data.unlocks,
		rating_data: data.rating_data,
	});
	let state_updates = server_state.filter_map(|x| match x
	{
		ServerState::Open => future::ready(None),
		ServerState::Closing => future::ready(Some(Update::Closing)),
		ServerState::Closed => future::ready(Some(Update::Closed)),
	});
	let poison_updates = poison.map(|x| x.into());

	let other_updates = stream::select(
		stream::select(
			stream::select(server_updates, ping_updates),
			stream::select(login_updates, state_updates),
		),
		poison_updates,
	)
	.map(|x| Ok(x))
	.chain(stream::once(async { Err(Error::Unexpected) }));

	let receiver = receive::Client {
		socket,
		client_id: client.id,
		has_proper_version: client.has_proper_version_a.clone(),
	};
	let message_updates = stream::try_unfold(receiver, |mut x| async {
		let message = x.receive().await?;
		Ok(Some((Update::Msg(message), x)))
	});

	let updates = stream::select(message_updates, other_updates);
	pin_mut!(updates);

	while let Some(update) = updates.next().await
	{
		let update: Update = update?;
		let has_quit = handle_update(&mut client, update).await?;
		if has_quit.is_some()
		{
			break;
		}
	}

	info!("Client {} stopped receiving.", client.id);
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
		invite: Option<lobby::Invite>,
	},
	LobbyNotFound
	{
		lobby_id: Keycode,
	},
	JoinedLobby
	{
		lobby_id: Keycode,
		lobby: mpsc::Sender<lobby::Update>,
	},
	RatingAndStars,
	Closing,
	Closed,
	Poison,
	Msg(Message),
}

impl From<handle::Poison> for Update
{
	fn from(_poison: handle::Poison) -> Update
	{
		Update::Poison
	}
}

#[derive(Debug)]
enum Error
{
	Invalid,
	Unexpected,
	Poisoned,
	Io(std::io::Error),
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
	SlackApi
	{
		error: mpsc::error::SendError<slack_api::Post>,
	},
	DiscordApi
	{
		error: mpsc::error::SendError<discord_api::Post>,
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
	ReceiveTask(receive::Error),
	SendTask(send::Error),
	PingTask(ping::Error),
	PulseTask(pulse::Error),
	LoginTask(login::Error),
}

impl From<std::io::Error> for Error
{
	fn from(error: std::io::Error) -> Self
	{
		Error::Io(error)
	}
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

impl From<mpsc::error::SendError<slack_api::Post>> for Error
{
	fn from(error: mpsc::error::SendError<slack_api::Post>) -> Self
	{
		Error::SlackApi { error }
	}
}

impl From<mpsc::error::SendError<discord_api::Post>> for Error
{
	fn from(error: mpsc::error::SendError<discord_api::Post>) -> Self
	{
		Error::DiscordApi { error }
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

impl From<receive::Error> for Error
{
	fn from(error: receive::Error) -> Self
	{
		Error::ReceiveTask(error)
	}
}

impl From<send::Error> for Error
{
	fn from(error: send::Error) -> Self
	{
		Error::SendTask(error)
	}
}

impl From<ping::Error> for Error
{
	fn from(error: ping::Error) -> Self
	{
		Error::PingTask(error)
	}
}

impl From<pulse::Error> for Error
{
	fn from(error: pulse::Error) -> Self
	{
		Error::PulseTask(error)
	}
}

impl From<login::Error> for Error
{
	fn from(error: login::Error) -> Self
	{
		Error::LoginTask(error)
	}
}

impl fmt::Display for Error
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
	{
		match self
		{
			Error::Invalid => write!(f, "Invalid message received"),
			Error::Unexpected => write!(f, "Something unexpected happened"),
			Error::Poisoned => write!(f, "Poisoned by chat or lobby"),
			Error::Io(error) => error.fmt(f),
			Error::Send { error } => error.fmt(f),
			Error::TrySend { error } => error.fmt(f),
			Error::Update { error } => error.fmt(f),
			Error::Chat { error } => error.fmt(f),
			Error::Lobby { error } => error.fmt(f),
			Error::Login { error } => error.fmt(f),
			Error::SlackApi { error } => error.fmt(f),
			Error::DiscordApi { error } => error.fmt(f),
			Error::Rating { error } => error.fmt(f),
			Error::Tolerance { error } => error.fmt(f),
			Error::Watch { error } => error.fmt(f),
			Error::Recv { error } => error.fmt(f),
			Error::OneshotRecv { error } => error.fmt(f),
			Error::ReceiveTask(e) => write!(f, "Error in receive task: {}", e),
			Error::SendTask(e) => write!(f, "Error in send task: {}", e),
			Error::PingTask(e) => write!(f, "Error in ping task: {}", e),
			Error::PulseTask(e) => write!(f, "Error in pulse task: {}", e),
			Error::LoginTask(e) => write!(f, "Error in login task: {}", e),
		}
	}
}

struct HasQuit;

async fn handle_update(
	client: &mut Client,
	update: Update,
) -> Result<Option<HasQuit>, Error>
{
	match update
	{
		Update::PingTaskRequestsPing { callback } =>
		{
			client.pong_receive_time = Some(callback);
			client.sendbuffer.try_send(Message::Ping)?;
			Ok(None)
		}
		Update::BeingGhostbusted =>
		{
			let tolerance = Duration::from_secs(5);
			client.ping_tolerance.broadcast(tolerance)?;
			Ok(None)
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

			let (rating_in, rating_out) = watch::channel(rating_data);
			client.data_for_rating = Some((rating_data, rating_in));

			match &mut client.general_chat_reserve
			{
				Some(chat) =>
				{
					let request = chat::Update::Join {
						client_id: client.id,
						username: client.username.clone(),
						unlocks: client.unlocks.clone(),
						rating_data: rating_out,
						handle: client.handle.clone(),
					};
					match chat.try_send(request)
					{
						Ok(()) => Ok(None),
						Err(error) =>
						{
							error!(
								"Client {} failed to join chat: {:?}",
								client.id, error
							);
							// If the chat cannot handle more updates, it is
							// probably too busy to handle more clients.
							// FUTURE better error handling (#962)
							let message = Message::JoinServer {
								status: Some(ResponseStatus::UnknownError),
								content: None,
								sender: None,
								metadata: Default::default(),
							};
							client.sendbuffer.try_send(message)?;
							Ok(None)
						}
					}
				}
				None =>
				{
					client.sendbuffer.try_send(Message::Closing)?;
					Ok(None)
				}
			}
		}
		Update::JoinedServer => match client.general_chat_reserve.take()
		{
			Some(chat) =>
			{
				if let Some((data, sender)) = client.data_for_rating.take()
				{
					let user_id = match client.user_id
					{
						Some(user_id) => user_id,
						None =>
						{
							error!("Expected user_id");
							return Err(Error::Unexpected);
						}
					};
					let update = rating::Update::Fresh {
						user_id,
						handle: client.handle.clone(),
						data,
						sender,
					};
					client.rating_database.send(update).await?;
				}

				info!(
					"User '{}' joined. (v{})",
					client.username, client.version,
				);
				client.appears_active_according_to_notifications = true;

				client.general_chat = Some(chat);
				Ok(None)
			}
			None =>
			{
				client.sendbuffer.try_send(Message::Closing)?;
				Ok(None)
			}
		},

		Update::LobbyFound {
			lobby_id: _,
			invite,
			mut lobby_sendbuffer,
			general_chat,
		} =>
		{
			let client_user_id = match client.user_id
			{
				Some(user_id) => user_id,
				None =>
				{
					error!("Expected user_id");
					return Err(Error::Unexpected);
				}
			};
			let update = lobby::Update::Join {
				client_id: client.id,
				client_user_id,
				client_username: client.username.clone(),
				client_handle: client.handle.clone(),
				lobby_sendbuffer: lobby_sendbuffer.clone(),
				general_chat,
				desired_metadata: None,
				invite,
			};
			lobby_sendbuffer.send(update).await?;
			Ok(None)
		}
		Update::LobbyNotFound { lobby_id: _ } =>
		{
			client.sendbuffer.try_send(Message::JoinLobby {
				lobby_id: None,
				username: None,
				invite: None,
			})?;
			Ok(None)
		}
		Update::JoinedLobby { lobby_id, lobby } =>
		{
			if client.is_bot()
			{
				client.bot_lobbies.insert(lobby_id, lobby);
			}
			else
			{
				client.lobby = Some(lobby);
			}
			Ok(None)
		}

		Update::RatingAndStars =>
		{
			if let Some(chat) = &mut client.general_chat
			{
				let update = chat::Update::RatingAndStars {
					client_id: client.id,
				};
				chat.send(update).await?
			}
			Ok(None)
		}

		Update::Closing =>
		{
			client.closing = true;
			client.general_chat_reserve.take();
			client.sendbuffer.try_send(Message::Closing)?;
			Ok(None)
		}
		Update::Closed =>
		{
			client.closing = true;
			client.general_chat_reserve.take();
			client.sendbuffer.try_send(Message::Closed)?;
			Ok(None)
		}

		Update::Poison => Err(Error::Poisoned),

		Update::Msg(message) => handle_message(client, message).await,
	}
}

async fn handle_message(
	client: &mut Client,
	message: Message,
) -> Result<Option<HasQuit>, Error>
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

			// We want to know if someone is peeking.
			client.appears_active_according_to_notifications = true;
		}
		Message::Quit =>
		{
			info!("Client {} is gracefully disconnecting...", client.id);
			client.has_gracefully_disconnected = true;
			return Ok(Some(HasQuit));
		}
		Message::JoinServer { .. } if client.general_chat.is_some() =>
		{
			debug!("Ignoring message from online client: {:?}", message);
		}
		Message::JoinServer { .. } if client.closing =>
		{
			client.sendbuffer.try_send(Message::Closing)?;
		}
		Message::JoinServer {
			status: None,
			content: Some(token),
			sender: Some(account_identifier),
			metadata: JoinMetadataOrTagMetadata::JoinMetadata(metadata),
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
				// FUTURE better error handling (#962)
				let rejection = Message::LeaveServer { content: None };
				client.sendbuffer.try_send(rejection)?;
			}
			else
			{
				let request = login::Request {
					token,
					account_identifier,
					metadata,
				};
				joining_server(client, request)?;
			}
		}
		Message::JoinServer { .. } =>
		{
			warn!("Invalid message from client: {:?}", message);
			return Err(Error::Invalid);
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

				for (_, mut lobby) in client.bot_lobbies.drain()
				{
					let update = lobby::Update::Leave {
						client_id: client.id,
						general_chat: general_chat.clone(),
					};
					lobby.send(update).await?;
				}

				if let Some(user_id) = client.user_id
				{
					let update = rating::Update::Left { user_id };
					client.rating_database.send(update).await?;
				}

				let update = chat::Update::Leave {
					client_id: client.id,
				};
				general_chat.send(update).await?;

				info!("User '{}' left. (v{})", client.username, client.version,);
				client.appears_active_according_to_notifications = false;

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
		Message::LinkAccounts { metadata } =>
		{
			let AccountLinkingMetadata { discord_user_id } = metadata;

			if !client.username.is_empty()
			{
				let post = discord_api::Post::Link {
					username: client.username.clone(),
					discord_id: discord_user_id,
				};
				client.discord_api.send(post).await?;
			}
			else
			{
				debug!("Ignoring LinkAccounts from client without username");
			}
		}
		Message::JoinLobby { .. } if client.is_bot() =>
		{
			debug!("Invalid message from bot: {:?}", message);
			return Err(Error::Invalid);
		}
		Message::JoinLobby { .. } if client.closing =>
		{
			client.sendbuffer.try_send(Message::Closing)?;
		}
		Message::JoinLobby {
			lobby_id: Some(lobby_id),
			username: None,
			invite,
		} =>
		{
			if let Some(ref mut general_chat) = client.general_chat
			{
				if client.lobby.is_some()
				{
					debug!("Ignoring JoinLobby from client in lobby.");
					return Ok(None);
				}

				let update = chat::Update::FindLobby {
					lobby_id,
					handle: client.handle.clone(),
					general_chat: general_chat.clone(),
					invite,
				};
				general_chat.send(update).await?;
			}
			else
			{
				debug!("Ignoring JoinLobby from offline client.");
			}
		}
		Message::JoinLobby { .. } =>
		{
			warn!("Invalid message from client: {:?}", message);
			return Err(Error::Invalid);
		}
		Message::LeaveLobby {
			lobby_id: None,
			username: None,
		} =>
		{
			if let Some(ref mut lobby) = client.lobby.take()
			{
				let general_chat = match &client.general_chat
				{
					Some(general_chat) => general_chat.clone(),
					None =>
					{
						error!("Expected general_chat");
						return Err(Error::Unexpected);
					}
				};

				let update = lobby::Update::Leave {
					client_id: client.id,
					general_chat: general_chat,
				};
				lobby.send(update).await?;
			}
			else
			{
				debug!("Ignoring LeaveLobby from unlobbied client.");
			}
		}
		Message::LeaveLobby { .. } =>
		{
			warn!("Invalid message from client: {:?}", message);
			return Err(Error::Invalid);
		}
		Message::MakeLobby { .. } if client.is_bot() =>
		{
			debug!("Invalid message from bot: {:?}", message);
			return Err(Error::Invalid);
		}
		Message::MakeLobby { .. } if client.closing =>
		{
			client.sendbuffer.try_send(Message::Closing)?;
		}
		Message::MakeLobby { .. } if client.lobby.is_some() =>
		{
			debug!("Ignoring message from lobbied client: {:?}", message);
		}
		Message::MakeLobby { metadata } => match client.general_chat
		{
			Some(ref general_chat) =>
			{
				if let Some(metadata) = metadata
				{
					if metadata.lobby_type == lobby::LobbyType::Replay
					{
						info!("Replay lobbies are disabled");
						let message = Message::Chat {
							content: "Replays are not available at the moment."
								.to_string(),
							sender: Some("server".to_string()),
							target: ChatTarget::General,
						};
						client.sendbuffer.try_send(message)?;
						return Ok(None);
					}
				}

				let mut lobby = lobby::create(
					&mut client.lobby_authority,
					client.rating_database.clone(),
					client.discord_api.clone(),
					client.canary_for_lobbies.clone(),
				);

				let client_user_id = match client.user_id
				{
					Some(user_id) => user_id,
					None =>
					{
						error!("Expected user_id");
						return Err(Error::Unexpected);
					}
				};
				let update = lobby::Update::Join {
					client_id: client.id,
					client_user_id,
					client_username: client.username.clone(),
					client_handle: client.handle.clone(),
					lobby_sendbuffer: lobby.clone(),
					general_chat: general_chat.clone(),
					desired_metadata: metadata,
					invite: None,
				};
				lobby.send(update).await?;
				client.lobby = Some(lobby);
			}
			None =>
			{
				debug!("Ignoring message from offline client: {:?}", message);
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
						error!("Expected general_chat");
						return Err(Error::Unexpected);
					}
				};

				let update = lobby::Update::ForSetup(lobby::Sub::Save {
					lobby_sendbuffer: lobby.clone(),
					general_chat,
				});
				lobby.send(update).await?;
			}
			None =>
			{
				debug!("Ignoring message from unlobbied client: {:?}", message);
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
						error!("Expected general_chat");
						return Err(Error::Unexpected);
					}
				};

				let update =
					lobby::Update::ForSetup(lobby::Sub::Lock { general_chat });
				lobby.send(update).await?;
			}
			None =>
			{
				debug!("Ignoring message from unlobbied client: {:?}", message);
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
						error!("Expected general_chat");
						return Err(Error::Unexpected);
					}
				};

				let update = lobby::Update::ForSetup(lobby::Sub::Unlock {
					general_chat,
				});
				lobby.send(update).await?;
			}
			None =>
			{
				debug!("Ignoring UnlockLobby from unlobbied client.");
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
						error!("Expected general_chat");
						return Err(Error::Unexpected);
					}
				};

				let update = lobby::Update::ForSetup(lobby::Sub::Rename {
					lobby_name,
					general_chat,
				});
				lobby.send(update).await?;
			}
			None =>
			{
				debug!("Ignoring NameLobby message from unlobbied client");
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
						error!("Expected general_chat");
						return Err(Error::Unexpected);
					}
				};

				let update = lobby::Update::ForSetup(lobby::Sub::ClaimRole {
					general_chat,
					username,
					role,
				});
				lobby.send(update).await?;
			}
			None =>
			{
				debug!("Ignoring ClaimRole from unlobbied client");
			}
		},
		Message::ClaimColor {
			username_or_slot,
			color,
		} => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let update = lobby::Update::ForSetup(lobby::Sub::ClaimColor {
					username_or_slot,
					color,
				});
				lobby.send(update).await?;
			}
			None =>
			{
				debug!("Ignoring ClaimColor from unlobbied client");
			}
		},
		Message::ClaimVisionType {
			username_or_slot,
			visiontype,
		} => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let update =
					lobby::Update::ForSetup(lobby::Sub::ClaimVisionType {
						username_or_slot,
						visiontype,
					});
				lobby.send(update).await?;
			}
			None =>
			{
				debug!("Ignoring ClaimVisionType from unlobbied client");
			}
		},
		Message::ClaimAi {
			username_or_slot,
			ai_name,
		} => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let update = lobby::Update::ForSetup(lobby::Sub::ClaimAi {
					username_or_slot,
					ai_name,
				});
				lobby.send(update).await?;
			}
			None =>
			{
				debug!("Ignoring ClaimAi from unlobbied client");
			}
		},
		Message::ClaimDifficulty {
			username_or_slot,
			difficulty,
		} => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let update =
					lobby::Update::ForSetup(lobby::Sub::ClaimDifficulty {
						username_or_slot,
						difficulty,
					});
				lobby.send(update).await?;
			}
			None =>
			{
				debug!("Ignoring ClaimDifficulty from unlobbied client");
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
						error!("Expected general_chat");
						return Err(Error::Unexpected);
					}
				};

				let update = lobby::Update::ForSetup(lobby::Sub::PickMap {
					general_chat,
					map_name,
				});
				lobby.send(update).await?;
			}
			None =>
			{
				debug!("Ignoring PickMap from unlobbied client");
			}
		},
		Message::PickTimer { seconds } => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let update =
					lobby::Update::ForSetup(lobby::Sub::PickTimer { seconds });
				lobby.send(update).await?;
			}
			None =>
			{
				debug!("Ignoring PickTimer from unlobbied client");
			}
		},
		Message::PickRuleset { ruleset_name } => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let update = lobby::Update::ForSetup(lobby::Sub::PickRuleset {
					ruleset_name,
				});
				lobby.send(update).await?;
			}
			None =>
			{
				debug!("Ignoring PickRuleset from unlobbied client");
			}
		},
		Message::ListRuleset {
			ruleset_name,
			connected_bot: Some(ConnectedBotMetadata { lobby_id, slot }),
		} if client.is_bot() =>
		{
			if let Some(lobby) = client.bot_lobbies.get_mut(&lobby_id)
			{
				let general_chat = match &client.general_chat
				{
					Some(general_chat) => general_chat.clone(),
					None =>
					{
						error!("Expected general_chat");
						return Err(Error::Unexpected);
					}
				};

				let update =
					lobby::Update::ForSetup(lobby::Sub::BotConfirmRuleset {
						client_id: client.id,
						slot,
						general_chat,
						ruleset_name,
					});
				lobby.send(update).await?;
			}
			else
			{
				debug!("Ignoring ListRuleset from unlobbied bot");
			}
		}
		Message::ListRuleset {
			ruleset_name,
			connected_bot: _,
		} => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let general_chat = match &client.general_chat
				{
					Some(general_chat) => general_chat.clone(),
					None =>
					{
						error!("Expected general_chat");
						return Err(Error::Unexpected);
					}
				};

				let update =
					lobby::Update::ForSetup(lobby::Sub::ConfirmRuleset {
						client_id: client.id,
						general_chat,
						ruleset_name,
					});
				lobby.send(update).await?;
			}
			None =>
			{
				debug!("Ignoring ListRuleset from unlobbied client");
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
						error!("Expected general_chat");
						return Err(Error::Unexpected);
					}
				};

				let update = lobby::Update::ForSetup(lobby::Sub::AddBot {
					general_chat,
				});
				lobby.send(update).await?;
			}
			None =>
			{
				debug!("Ignoring AddBot from unlobbied client");
			}
		},
		Message::AddBot { .. } =>
		{
			warn!("Invalid message from client: {:?}", message);
			return Err(Error::Invalid);
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
						error!("Expected general_chat");
						return Err(Error::Unexpected);
					}
				};

				let update = lobby::Update::ForSetup(lobby::Sub::RemoveBot {
					general_chat,
					slot,
				});
				lobby.send(update).await?;
			}
			None =>
			{
				debug!("Ignoring RemoveBot from unlobbied client");
			}
		},
		Message::EnableCustomMaps => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let general_chat = match &client.general_chat
				{
					Some(general_chat) => general_chat.clone(),
					None =>
					{
						error!("Expected general_chat");
						return Err(Error::Unexpected);
					}
				};

				let update =
					lobby::Update::ForSetup(lobby::Sub::EnableCustomMaps {
						general_chat,
					});
				lobby.send(update).await?;
			}
			None =>
			{
				debug!("Ignoring EnableCustomMaps from unlobbied client");
			}
		},
		Message::RulesetRequest { ruleset_name } =>
		{
			handle_ruleset_request(client, ruleset_name).await?;
		}
		Message::Start => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let general_chat = match &client.general_chat
				{
					Some(general_chat) => general_chat.clone(),
					None =>
					{
						error!("Expected general_chat");
						return Err(Error::Unexpected);
					}
				};

				let update =
					lobby::Update::ForSetup(lobby::Sub::Start { general_chat });
				lobby.send(update).await?;
			}
			None =>
			{
				debug!("Ignoring Start from unlobbied client");
			}
		},
		Message::Resign {
			username: None,
			connected_bot: Some(ConnectedBotMetadata { lobby_id, slot }),
		} if client.is_bot() =>
		{
			if let Some(lobby) = client.bot_lobbies.get_mut(&lobby_id)
			{
				let update = lobby::Update::ForGame(game::Sub::BotResign {
					client_id: client.id,
					slot: slot,
				});
				lobby.send(update).await?;
			}
			else
			{
				debug!("Ignoring ListRuleset from unlobbied bot");
			}
		}
		Message::Resign {
			username: None,
			connected_bot: _,
		} => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let update = lobby::Update::ForGame(game::Sub::Resign {
					client_id: client.id,
				});
				lobby.send(update).await?
			}
			None =>
			{
				debug!("Ignoring Sync from unlobbied client");
			}
		},
		Message::Resign { .. } =>
		{
			warn!("Invalid message from client: {:?}", message);
			return Err(Error::Invalid);
		}
		Message::OrdersNew {
			orders,
			connected_bot: Some(ConnectedBotMetadata { lobby_id, slot }),
		} if client.is_bot() =>
		{
			if let Some(lobby) = client.bot_lobbies.get_mut(&lobby_id)
			{
				let update = lobby::Update::ForGame(game::Sub::BotOrders {
					client_id: client.id,
					slot: slot,
					orders,
				});
				lobby.send(update).await?;
			}
			else
			{
				debug!("Ignoring ListRuleset from unlobbied bot");
			}
		}
		Message::OrdersNew {
			orders,
			connected_bot: _,
		} => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let update = lobby::Update::ForGame(game::Sub::Orders {
					client_id: client.id,
					orders,
				});
				lobby.send(update).await?
			}
			None =>
			{
				debug!("Ignoring Sync from unlobbied client");
			}
		},
		Message::Sync {
			time_remaining_in_seconds: None,
			connected_bot: Some(ConnectedBotMetadata { lobby_id, slot }),
		} if client.is_bot() =>
		{
			if let Some(lobby) = client.bot_lobbies.get_mut(&lobby_id)
			{
				let update = lobby::Update::ForGame(game::Sub::BotSync {
					client_id: client.id,
					slot: slot,
				});
				lobby.send(update).await?;
			}
			else
			{
				debug!("Ignoring ListRuleset from unlobbied bot");
			}
		}
		Message::Sync {
			time_remaining_in_seconds: None,
			connected_bot: _,
		} => match client.lobby
		{
			Some(ref mut lobby) =>
			{
				let update = lobby::Update::ForGame(game::Sub::Sync {
					client_id: client.id,
				});
				lobby.send(update).await?
			}
			None =>
			{
				debug!("Ignoring Sync from unlobbied client");
			}
		},
		Message::Sync { .. } =>
		{
			warn!("Invalid message from client: {:?}", message);
			return Err(Error::Invalid);
		}
		Message::Chat { .. } if client.username.is_empty() =>
		{
			info!("Ignoring Chat from client without username: {:?}", message);
		}
		Message::Chat {
			content,
			sender: None,
			target: ChatTarget::General,
		} => match client.general_chat
		{
			Some(ref mut general_chat) =>
			{
				info!(
					"Client {} '{}' sent chat message: {}",
					client.id,
					client.username,
					content.escape_debug()
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
				info!("Ignoring Chat from offline client: '{:?}'", content);
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
				info!(
					"Client {} '{}' sent lobby chat message: {}",
					client.id,
					client.username,
					content.escape_debug()
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
				info!(
					"Ignoring Chat to lobby from unlobbied client: '{:?}'",
					content
				);
			}
		},
		Message::Chat { .. } =>
		{
			warn!("Invalid message from client: {:?}", message);
			return Err(Error::Invalid);
		}
		Message::Debug { content } =>
		{
			debug!("Client {} says: {}", client.id, content.escape_debug());
		}
		Message::Init
		| Message::DisbandLobby { .. }
		| Message::ListLobby { .. }
		| Message::ListChallenge { .. }
		| Message::ListAi { .. }
		| Message::ListMap { .. }
		| Message::PickChallenge { .. }
		| Message::AssignColor { .. }
		| Message::RulesetData { .. }
		| Message::RulesetUnknown { .. }
		| Message::Secrets { .. }
		| Message::Skins { .. }
		| Message::InGame { .. }
		| Message::Game { .. }
		| Message::Tutorial { .. }
		| Message::Challenge { .. }
		| Message::Briefing { .. }
		| Message::ReplayWithAnimations { .. }
		| Message::Changes { .. }
		| Message::OrdersOld { .. }
		| Message::RatingAndStars { .. }
		| Message::UpdatedRating { .. }
		| Message::RecentStars { .. }
		| Message::Closing
		| Message::Closed =>
		{
			warn!("Invalid message from client: {:?}", message);
			return Err(Error::Invalid);
		}
	}

	Ok(None)
}

fn greet_client(client: &mut Client, version: Version) -> Result<(), Error>
{
	client.version = version;
	info!("Client {} has version {}.", client.id, version);

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
	info!("Client {} is logging in...", client.id);

	match client.login.try_send(request)
	{
		Ok(()) => Ok(()),
		Err(mpsc::error::TrySendError::Full(_request)) =>
		{
			error!("Failed to enqueue for login, login task busy.");

			// We only process one login request at a time. Does it make sense
			// to respond to a second request if the first response is still
			// underway?
			// FUTURE better error handling (#962)
			let message = Message::JoinServer {
				status: Some(ResponseStatus::ConnectionFailed),
				content: None,
				sender: None,
				metadata: Default::default(),
			};
			client.sendbuffer.try_send(message)?;
			Ok(())
		}
		Err(error) => Err(error.into()),
	}
}

async fn handle_ruleset_request(
	client: &mut Client,
	ruleset_name: String,
) -> Result<(), Error>
{
	if !ruleset::exists(&ruleset_name)
	{
		let message = Message::RulesetUnknown { ruleset_name };
		client.sendbuffer.send(message).await?;
		return Ok(());
	}

	let data = ruleset::load_data(&ruleset_name).await?;
	let message = Message::RulesetData { ruleset_name, data };
	client.sendbuffer.send(message).await?;
	Ok(())
}
