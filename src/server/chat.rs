/* Server::Chat */

use crate::common::keycode::*;
use crate::server::client;
use crate::server::lobby;
use crate::server::message::*;

use tokio::sync::mpsc;

use enumset::*;
use vec_drain_where::VecDrainWhereExt;

#[derive(Debug)]
pub enum Update
{
	Join
	{
		client_id: Keycode,
		username: String,
		unlocks: EnumSet<Unlock>,
		sendbuffer: mpsc::Sender<Message>,
	},
	Init
	{
		sendbuffer: mpsc::Sender<Message>,
	},
	Leave
	{
		client_id: Keycode,
	},

	ListLobby
	{
		lobby_id: Keycode,
		description_messages: Vec<Message>,
		sendbuffer: mpsc::Sender<lobby::Update>,
	},
	DescribeLobby
	{
		lobby_id: Keycode,
		description_messages: Vec<Message>,
	},
	DisbandLobby
	{
		lobby_id: Keycode,
	},

	FindLobby
	{
		lobby_id: Keycode,
		callback: mpsc::Sender<client::Update>,
		general_chat: mpsc::Sender<Update>,
	},

	Msg(Message),
}

pub async fn run(mut updates: mpsc::Receiver<Update>, canary: mpsc::Sender<()>)
{
	let mut clients: Vec<Client> = Vec::new();
	let mut lobbies: Vec<Lobby> = Vec::new();

	while let Some(update) = updates.recv().await
	{
		handle_update(update, &mut clients, &mut lobbies);
	}

	println!("General chat has disbanded.");
	let _discard = canary;
}

fn handle_update(
	update: Update,
	clients: &mut Vec<Client>,
	lobbies: &mut Vec<Lobby>,
)
{
	match update
	{
		Update::Join {
			client_id,
			username,
			unlocks,
			sendbuffer,
		} => handle_join(
			client_id, username, unlocks, sendbuffer, clients, lobbies,
		),
		Update::Init { sendbuffer } =>
		{
			handle_init(sendbuffer, clients, lobbies)
		}
		Update::Leave { client_id } => handle_leave(client_id, clients),

		Update::ListLobby {
			lobby_id,
			description_messages,
			sendbuffer,
		} =>
		{
			let lobby = Lobby {
				id: lobby_id,
				description_messages,
				sendbuffer,
				dead: false,
			};
			handle_list_lobby(lobby, clients, lobbies)
		}
		Update::DescribeLobby {
			lobby_id,
			description_messages,
		} => handle_describe_lobby(
			lobby_id,
			description_messages,
			clients,
			lobbies,
		),
		Update::DisbandLobby { lobby_id } =>
		{
			handle_disband_lobby(lobby_id, clients, lobbies)
		}

		Update::FindLobby {
			lobby_id,
			callback,
			general_chat,
		} =>
		{
			handle_find_lobby(lobbies, lobby_id, callback, general_chat);
		}

		Update::Msg(message) =>
		{
			for client in clients.iter_mut()
			{
				client.send(message.clone());
			}
		}
	}
}

struct Client
{
	id: Keycode,
	username: String,
	join_metadata: Option<JoinMetadata>,
	sendbuffer: mpsc::Sender<Message>,
	hidden: bool,
	dead: bool,
}

impl Client
{
	fn send(&mut self, message: Message)
	{
		match self.sendbuffer.try_send(message)
		{
			Ok(()) => (),
			// TODO filter dead clients somehow
			Err(error) =>
			{
				println!("Error sending to client {}: {:?}", self.id, error);
				self.dead = true
			}
		}
	}
}

#[derive(Debug, Clone)]
struct Lobby
{
	id: Keycode,
	description_messages: Vec<Message>,
	sendbuffer: mpsc::Sender<lobby::Update>,
	dead: bool,
}

fn handle_join(
	id: Keycode,
	username: String,
	unlocks: EnumSet<Unlock>,
	sendbuffer: mpsc::Sender<Message>,
	clients: &mut Vec<Client>,
	lobbies: &Vec<Lobby>,
)
{
	// TODO ghostbusting

	let join_metadata = generate_join_metadata(&unlocks);
	let hidden = username.starts_with("#");

	let mut newcomer = Client {
		id: id,
		username,
		join_metadata,
		sendbuffer,
		hidden: hidden,
		dead: false,
	};

	// Confirm to the newcomer that they have joined.
	let message = Message::JoinServer {
		status: None,
		content: Some(newcomer.username.clone()),
		sender: None,
		metadata: newcomer.join_metadata,
	};
	newcomer.send(message.clone());

	// Tell everyone who the newcomer is.
	if !newcomer.hidden
	{
		for other in clients.iter_mut()
		{
			other.send(message.clone());
		}

		// Tell everyone the rating and stars of the newcomer.
		// TODO rating and stars
	}

	// Let the client know which lobbies there are.
	for lobby in lobbies.iter()
	{
		for message in lobby.description_messages.iter()
		{
			newcomer.send(message.clone())
		}
	}

	// Let the client know who else is online.
	for other in clients.iter()
	{
		if !other.hidden
		{
			newcomer.send(Message::JoinServer {
				status: None,
				content: Some(other.username.clone()),
				sender: None,
				metadata: other.join_metadata,
			});

			// TODO rating
			// TODO stars
			// TODO join_lobby
			// TODO in_game
		}
	}

	// Tell the newcomer that they are online.
	// TODO this is weird (#1411)
	newcomer.send(message);

	// Let the client know we are done initializing.
	newcomer.send(Message::Init);

	// Show them a welcome message, if any.
	welcome_client(&mut newcomer);

	clients.push(newcomer);
}

fn welcome_client(_client: &mut Client)
{
	// No welcome message at the moment.
}

fn generate_join_metadata(unlocks: &EnumSet<Unlock>) -> Option<JoinMetadata>
{
	let mut metadata: JoinMetadata = Default::default();
	if unlocks.contains(Unlock::Dev)
	{
		metadata.dev = true;
	}
	if unlocks.contains(Unlock::Guest)
	{
		metadata.guest = true;
	}

	if metadata == Default::default()
	{
		None
	}
	else
	{
		Some(metadata)
	}
}

fn handle_init(
	sendbuffer: mpsc::Sender<Message>,
	clients: &Vec<Client>,
	lobbies: &Vec<Lobby>,
)
{
	match do_init(sendbuffer, clients, lobbies)
	{
		Ok(()) => (),
		Err(e) => eprintln!("Send error while processing init: {:?}", e),
	}
}

fn do_init(
	mut sendbuffer: mpsc::Sender<Message>,
	clients: &Vec<Client>,
	lobbies: &Vec<Lobby>,
) -> Result<(), mpsc::error::TrySendError<Message>>
{
	// Let the client know which lobbies there are.
	for lobby in lobbies.iter()
	{
		for message in lobby.description_messages.iter()
		{
			sendbuffer.try_send(message.clone())?;
		}
	}

	// Let the client know who else is online.
	for client in clients
	{
		if !client.hidden
		{
			sendbuffer.try_send(Message::JoinServer {
				status: None,
				content: Some(client.username.clone()),
				sender: None,
				metadata: client.join_metadata,
			})?;

			// TODO rating
			// TODO stars
			// TODO join_lobby
			// TODO in_game
		}
	}

	// Let the client know we are done initializing.
	sendbuffer.try_send(Message::Init)
}

fn handle_leave(client_id: Keycode, clients: &mut Vec<Client>)
{
	let removed: Vec<Client> = clients
		.e_drain_where(|client| client.id == client_id)
		.collect();

	for removed_client in removed
	{
		let Client {
			id: _,
			username,
			join_metadata: _,
			mut sendbuffer,
			hidden,
			dead: _,
		} = removed_client;

		let message = Message::LeaveServer {
			content: Some(username),
		};

		if !hidden
		{
			for client in clients.iter_mut()
			{
				client.send(message.clone());
			}
		}

		match sendbuffer.try_send(message)
		{
			Ok(()) => (),
			Err(e) => eprintln!("Send error while processing leave: {:?}", e),
		}
	}
}

fn handle_list_lobby(
	newlobby: Lobby,
	clients: &mut Vec<Client>,
	lobbies: &mut Vec<Lobby>,
)
{
	lobbies.retain(|lobby| lobby.id != newlobby.id);

	for client in clients.iter_mut()
	{
		for message in newlobby.description_messages.iter()
		{
			client.send(message.clone())
		}
	}

	lobbies.push(newlobby);
}

fn handle_describe_lobby(
	lobby_id: Keycode,
	description_messages: Vec<Message>,
	clients: &mut Vec<Client>,
	lobbies: &mut Vec<Lobby>,
)
{
	for client in clients.iter_mut()
	{
		for message in description_messages.iter()
		{
			client.send(message.clone())
		}
	}

	for lobby in lobbies
	{
		if lobby.id == lobby_id
		{
			lobby.description_messages = description_messages;
			return;
		}
	}
}

fn handle_disband_lobby(
	lobby_id: Keycode,
	clients: &mut Vec<Client>,
	lobbies: &mut Vec<Lobby>,
)
{
	lobbies.retain(|lobby| lobby.id != lobby_id);

	let message = Message::DisbandLobby { lobby_id };
	for client in clients.iter_mut()
	{
		client.send(message.clone())
	}
}

fn handle_find_lobby(
	lobbies: &mut Vec<Lobby>,
	lobby_id: Keycode,
	mut client_callback: mpsc::Sender<client::Update>,
	general_chat: mpsc::Sender<Update>,
)
{
	for lobby in lobbies.iter_mut()
	{
		if lobby.id == lobby_id
		{
			let update = client::Update::LobbyFound {
				lobby_id,
				lobby_sendbuffer: lobby.sendbuffer.clone(),
				general_chat,
			};
			match client_callback.try_send(update)
			{
				Ok(()) => (),
				Err(e) => eprintln!("Send error while finding lobby: {:?}", e),
			}
			return;
		}
	}

	let update = client::Update::LobbyNotFound { lobby_id };
	match client_callback.try_send(update)
	{
		Ok(()) => (),
		Err(e) => eprintln!("Send error while finding lobby: {:?}", e),
	}
}
