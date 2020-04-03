/* Server::Lobby */

use crate::common::keycode::*;
use crate::server::chat;
use crate::server::message::*;

use std::sync;
use std::sync::atomic;

use futures::future::Future;
use futures::stream::Stream;

use tokio::sync::mpsc;

use vec_drain_where::VecDrainWhereExt;

#[derive(Debug)]
pub enum Update
{
	Save,

	Join
	{
		client_id: Keycode,
		username: String,
		sendbuffer: mpsc::Sender<Message>,
	},
	Leave
	{
		client_id: Keycode,
	},

	Lock,
	Unlock,

	Msg(Message),
}

pub fn create(
	ticker: &mut sync::Arc<atomic::AtomicU64>,
	general_chat: mpsc::Sender<chat::Update>,
) -> mpsc::Sender<Update>
{
	let key = rand::random();
	let data = ticker.fetch_add(1, atomic::Ordering::Relaxed);
	let lobby_id = keycode(key, data);

	let (updates_in, updates_out) = mpsc::channel::<Update>(1000);

	let lobby = Lobby {
		id: lobby_id,
		name: format!("Unnamed lobby ({})", lobby_id),
		num_players: 0,
		max_players: 2,
		public: true,
		sendbuffer: updates_in.clone(),
	};

	let task = start_task(lobby, general_chat, updates_out);
	tokio::spawn(task);

	updates_in
}

#[derive(Debug, Clone)]
struct Lobby
{
	id: Keycode,
	name: String,
	num_players: i32,
	max_players: i32,
	public: bool,
	sendbuffer: mpsc::Sender<Update>,
}

fn start_task(
	mut lobby: Lobby,
	mut general_chat: mpsc::Sender<chat::Update>,
	updates: mpsc::Receiver<Update>,
) -> impl Future<Item = (), Error = ()> + Send
{
	let lobby_id = lobby.id;
	let mut clients: Vec<Client> = Vec::new();

	updates
		.map_err(move |error| {
			eprintln!("Recv error in lobby {}: {:?}", lobby_id, error)
		})
		.for_each(move |update| {
			handle_update(update, &mut lobby, &mut general_chat, &mut clients)
		})
		.map(move |()| println!("Lobby {} has disbanded.", lobby_id))
}

fn handle_update(
	update: Update,
	lobby: &mut Lobby,
	general_chat: &mut mpsc::Sender<chat::Update>,
	clients: &mut Vec<Client>,
) -> Result<(), ()>
{
	match update
	{
		Update::Save =>
		{
			list_lobby(lobby, general_chat)?;
		}

		Update::Join {
			client_id,
			username,
			sendbuffer,
		} => handle_join(lobby, client_id, username, sendbuffer, clients),
		Update::Leave { client_id } =>
		{
			handle_leave(lobby, client_id, clients, general_chat)?;
		}

		Update::Lock =>
		{
			lobby.public = false;
			list_lobby(lobby, general_chat)?;
		}
		Update::Unlock =>
		{
			lobby.public = true;
			list_lobby(lobby, general_chat)?;
		}

		Update::Msg(message) =>
		{
			for client in clients.iter_mut()
			{
				client.send(message.clone());
			}
		}
	}

	Ok(())
}

fn list_lobby(
	lobby: &Lobby,
	general_chat: &mut mpsc::Sender<chat::Update>,
) -> Result<(), ()>
{
	let lobby_id = lobby.id;

	general_chat
		.try_send(chat::Update::ListLobby {
			lobby_id: lobby.id,
			description_messages: make_listing_messages(&lobby),
			sendbuffer: lobby.sendbuffer.clone(),
		})
		.map_err(|error| {
			eprintln!("Chat error in lobby {}: {:?}", lobby_id, error)
		})
}

fn make_listing_messages(lobby: &Lobby) -> Vec<Message>
{
	vec![
		Message::EditLobby { lobby_id: lobby.id },
		Message::MakeLobby {
			lobby_id: Some(lobby.id),
		},
		Message::NameLobby {
			lobby_id: Some(lobby.id),
			lobbyname: lobby.name.clone(),
		},
		Message::MaxPlayers {
			lobby_id: lobby.id,
			value: lobby.max_players,
		},
		Message::NumPlayers {
			lobby_id: lobby.id,
			value: lobby.num_players,
		},
		if lobby.public
		{
			Message::UnlockLobby {
				lobby_id: Some(lobby.id),
			}
		}
		else
		{
			Message::LockLobby {
				lobby_id: Some(lobby.id),
			}
		},
		Message::SaveLobby {
			lobby_id: Some(lobby.id),
		},
	]
}

struct Client
{
	id: Keycode,
	username: String,
	sendbuffer: mpsc::Sender<Message>,
	dead: bool,
}

impl Client
{
	fn send(&mut self, message: Message)
	{
		match self.sendbuffer.try_send(message)
		{
			Ok(()) => (),
			Err(_error) => self.dead = true,
		}
	}
}

fn handle_join(
	lobby: &mut Lobby,
	client_id: Keycode,
	username: String,
	sendbuffer: mpsc::Sender<Message>,
	clients: &mut Vec<Client>,
)
{
	// TODO joining might fail because it is full or locked etcetera

	let mut newcomer = Client {
		id: client_id,
		username,
		sendbuffer,
		dead: false,
	};

	// Tell the newcomer the maximum player count in advance,
	// so they can reserve the necessary slots in the UI.
	newcomer.send(Message::MaxPlayers {
		lobby_id: lobby.id,
		value: lobby.max_players,
	});

	// Tell the newcomer which users are already in the lobby.
	for other in clients.into_iter()
	{
		newcomer.send(Message::JoinLobby {
			lobby_id: Some(lobby.id),
			username: Some(other.username.clone()),
			metadata: None,
		});

		// TODO roles
		// TODO colors
		// TODO vision types
	}

	// TODO AI pool
	// TODO bots

	// TODO map pool if this is not a replay lobby
	// TODO other map settings
	// TODO list all recordings if this is a replay lobby
	// TODO other replay settings

	newcomer.send(Message::JoinLobby {
		lobby_id: Some(lobby.id),
		username: Some(newcomer.username.clone()),
		metadata: None,
	});

	clients.push(newcomer);
}

fn handle_leave(
	lobby: &mut Lobby,
	client_id: Keycode,
	clients: &mut Vec<Client>,
	general_chat: &mut mpsc::Sender<chat::Update>,
) -> Result<(), ()>
{
	let removed: Vec<Client> = clients
		.e_drain_where(|client| client.id == client_id)
		.collect();

	for removed_client in removed
	{
		let Client {
			id: _,
			username,
			mut sendbuffer,
			dead: _,
		} = removed_client;

		let message = Message::LeaveLobby {
			lobby_id: Some(lobby.id),
			username: Some(username),
		};

		for client in clients.iter_mut()
		{
			client.send(message.clone());
		}

		match sendbuffer.try_send(message)
		{
			Ok(()) => (),
			Err(e) => eprintln!("Send error while processing leave: {:?}", e),
		}
	}

	// TODO dont disband if rejoinable etcetera
	if clients.is_empty()
	{
		general_chat
			.try_send(chat::Update::DisbandLobby { lobby_id: lobby.id })
			.map_err(|error| {
				eprintln!("Chat error in lobby {}: {:?}", lobby.id, error)
			})?;
	}

	Ok(())
}
