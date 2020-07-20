/* Server::Chat */

use crate::common::keycode::*;
use crate::logic::challenge;
use crate::logic::challenge::Challenge;
use crate::server::client;
use crate::server::lobby;
use crate::server::login::Unlock;
use crate::server::message::*;

use std::collections::HashMap;

use log::*;

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
		handle: client::Handle,
	},
	Init
	{
		handle: client::Handle,
	},
	StillAlive
	{
		client_id: Keycode,
	},
	Leave
	{
		client_id: Keycode,
	},

	ListLobby
	{
		lobby_id: Keycode,
		description_message: Message,
		sendbuffer: mpsc::Sender<lobby::Update>,
	},
	DescribeLobby
	{
		lobby_id: Keycode,
		description_message: Message,
	},
	DisbandLobby
	{
		lobby_id: Keycode,
	},

	FindLobby
	{
		lobby_id: Keycode,
		handle: client::Handle,
		general_chat: mpsc::Sender<Update>,
	},

	InGame
	{
		lobby_id: Keycode,
		client_id: Keycode,
		role: Role,
	},

	Msg(Message),
}

pub async fn run(mut updates: mpsc::Receiver<Update>, canary: mpsc::Sender<()>)
{
	let current_challenge = challenge::load_current();
	let mut clients: Vec<Client> = Vec::new();
	let mut ghostbusters: HashMap<Keycode, Ghostbuster> = HashMap::new();
	let mut lobbies: Vec<Lobby> = Vec::new();

	while let Some(update) = updates.recv().await
	{
		handle_update(
			update,
			&mut clients,
			&mut ghostbusters,
			&mut lobbies,
			&current_challenge,
		);

		let removed = clients
			.e_drain_where(|client| client.handle.is_disconnected())
			.collect();
		handle_removed(removed, &mut clients, &mut ghostbusters);
	}

	info!("General chat has disbanded.");
	let _discarded = canary;
}

fn handle_update(
	update: Update,
	clients: &mut Vec<Client>,
	ghostbusters: &mut HashMap<Keycode, Ghostbuster>,
	lobbies: &mut Vec<Lobby>,
	current_challenge: &Challenge,
)
{
	match update
	{
		Update::Join {
			client_id,
			username,
			unlocks,
			handle,
		} => handle_join(
			client_id,
			username,
			unlocks,
			handle,
			clients,
			ghostbusters,
			lobbies,
			current_challenge,
		),
		Update::Init { handle } =>
		{
			handle_init(handle, clients, lobbies, current_challenge)
		}
		Update::StillAlive { client_id } =>
		{
			handle_still_alive(client_id, clients, ghostbusters)
		}
		Update::Leave { client_id } =>
		{
			handle_leave(client_id, clients, ghostbusters)
		}

		Update::ListLobby {
			lobby_id,
			description_message,
			sendbuffer,
		} =>
		{
			let lobby = Lobby {
				id: lobby_id,
				description_message,
				sendbuffer,
			};
			handle_list_lobby(lobby, clients, lobbies)
		}
		Update::DescribeLobby {
			lobby_id,
			description_message,
		} => handle_describe_lobby(
			lobby_id,
			description_message,
			clients,
			lobbies,
		),
		Update::DisbandLobby { lobby_id } =>
		{
			handle_disband_lobby(lobby_id, clients, lobbies)
		}

		Update::FindLobby {
			lobby_id,
			handle,
			general_chat,
		} =>
		{
			verify_lobby(lobby_id, clients, lobbies);
			handle_find_lobby(lobbies, lobby_id, handle, general_chat);
		}

		Update::InGame {
			lobby_id,
			client_id,
			role,
		} =>
		{
			handle_in_game(clients, lobby_id, client_id, role);
		}

		Update::Msg(message) =>
		{
			for client in clients.iter_mut()
			{
				client.handle.send(message.clone());
			}
		}
	}
}

struct Client
{
	id: Keycode,
	username: String,
	join_metadata: Option<JoinMetadata>,
	handle: client::Handle,
	hidden: bool,
}

struct Ghostbuster
{
	id: Keycode,
	username: String,
	handle: client::Handle,
	ghost_id: Keycode,
}

impl Ghostbuster
{
	fn deny(mut self)
	{
		debug!(
			"Client {} did not ghostbust client {}.",
			self.id, self.ghost_id
		);
		let message = Message::JoinServer {
			status: Some(ResponseStatus::UsernameTaken),
			content: None,
			sender: None,
			metadata: None,
		};
		self.handle.send(message);
	}

	fn resolve(mut self)
	{
		debug!(
			"Client {} successfully ghostbusted client {}.",
			self.id, self.ghost_id
		);
		// FUTURE is this the most sensible message type for this? (#962)
		let message = Message::LeaveServer {
			content: Some(self.username),
		};
		self.handle.send(message);
	}
}

#[derive(Debug, Clone)]
struct Lobby
{
	id: Keycode,
	description_message: Message,
	sendbuffer: mpsc::Sender<lobby::Update>,
}

fn handle_join(
	id: Keycode,
	username: String,
	unlocks: EnumSet<Unlock>,
	handle: client::Handle,
	clients: &mut Vec<Client>,
	ghostbusters: &mut HashMap<Keycode, Ghostbuster>,
	lobbies: &Vec<Lobby>,
	current_challenge: &Challenge,
)
{
	// Prevent a user being online with multiple connections simultaneously.
	match clients.iter_mut().find(|x| x.username == username)
	{
		Some(otherclient) =>
		{
			debug!(
				"Client {} is ghostbusting client {}, both named {}.",
				id, otherclient.id, username
			);

			// Make sure that that client is not a ghost by reducing their ping
			// tolerance and ensuring a ping is sent.
			otherclient.handle.notify(client::Update::BeingGhostbusted);

			// Make the newcomer wait for the result of ghostbusting.
			let newcomer = Ghostbuster {
				id,
				username,
				handle,
				ghost_id: otherclient.id,
			};
			let previous = ghostbusters.insert(otherclient.id, newcomer);
			if let Some(buster) = previous
			{
				buster.deny();
			}
			return;
		}
		None =>
		{}
	}

	let join_metadata = generate_join_metadata(&unlocks);
	let hidden = username.starts_with("#");

	let mut newcomer = Client {
		id: id,
		username,
		join_metadata,
		handle,
		hidden: hidden,
	};

	// Confirm to the newcomer that they have joined.
	let message = Message::JoinServer {
		status: None,
		content: Some(newcomer.username.clone()),
		sender: None,
		metadata: newcomer.join_metadata,
	};
	newcomer.handle.send(message.clone());

	// Tell everyone who the newcomer is.
	if !newcomer.hidden
	{
		for other in clients.iter_mut()
		{
			other.handle.send(message.clone());
		}

		// Tell everyone the rating and stars of the newcomer.
		// TODO rating and stars
	}

	// Let the client know which lobbies there are.
	for lobby in lobbies.iter()
	{
		newcomer.handle.send(lobby.description_message.clone());
	}

	// Let the client know who else is online.
	for other in clients.iter()
	{
		if !other.hidden
		{
			newcomer.handle.send(Message::JoinServer {
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
	// FUTURE this is weird (#1411)
	newcomer.handle.send(message);

	newcomer.handle.send(Message::ListChallenge {
		key: current_challenge.key.clone(),
		metadata: current_challenge.metadata.clone(),
	});

	// Let the client know we are done initializing.
	newcomer.handle.send(Message::Init);

	// Show them a welcome message, if any.
	welcome_client(&mut newcomer);

	// Let the clienthandler know we have successfully joined.
	newcomer.handle.notify(client::Update::JoinedServer);

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
	mut handle: client::Handle,
	clients: &Vec<Client>,
	lobbies: &Vec<Lobby>,
	current_challenge: &Challenge,
)
{
	// Let the client know which lobbies there are.
	for lobby in lobbies.iter()
	{
		handle.send(lobby.description_message.clone());
	}

	// Let the client know who else is online.
	for client in clients
	{
		if !client.hidden
		{
			handle.send(Message::JoinServer {
				status: None,
				content: Some(client.username.clone()),
				sender: None,
				metadata: client.join_metadata,
			});

			// TODO rating
			// TODO stars
			// TODO join_lobby
			// TODO in_game
		}
	}

	handle.send(Message::ListChallenge {
		key: current_challenge.key.clone(),
		metadata: current_challenge.metadata.clone(),
	});

	// Let the client know we are done initializing.
	handle.send(Message::Init)
}

fn handle_leave(
	client_id: Keycode,
	clients: &mut Vec<Client>,
	ghostbusters: &mut HashMap<Keycode, Ghostbuster>,
)
{
	let removed: Vec<Client> = clients
		.e_drain_where(|client| client.id == client_id)
		.collect();
	handle_removed(removed, clients, ghostbusters);
}

fn handle_removed(
	removed: Vec<Client>,
	clients: &mut Vec<Client>,
	ghostbusters: &mut HashMap<Keycode, Ghostbuster>,
)
{
	for removed_client in removed
	{
		let Client {
			id,
			username,
			join_metadata: _,
			mut handle,
			hidden,
		} = removed_client;

		let message = Message::LeaveServer {
			content: Some(username),
		};

		if !hidden
		{
			for client in clients.iter_mut()
			{
				client.handle.send(message.clone());
			}
		}

		handle.send(message);

		let ghostbuster = ghostbusters.remove(&id);
		if let Some(ghostbuster) = ghostbuster
		{
			ghostbuster.resolve();
		}
	}
}

fn handle_still_alive(
	client_id: Keycode,
	clients: &mut Vec<Client>,
	ghostbusters: &mut HashMap<Keycode, Ghostbuster>,
)
{
	match clients.iter().find(|x| x.id == client_id)
	{
		Some(_client) =>
		{
			let ghostbuster = ghostbusters.remove(&client_id);
			if let Some(ghostbuster) = ghostbuster
			{
				ghostbuster.deny();
			}
		}
		None =>
		{
			warn!("Missing client {} is still alive.", client_id);

			let ghostbuster = ghostbusters.remove(&client_id);
			if let Some(ghostbuster) = ghostbuster
			{
				ghostbuster.resolve();
			}
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
		client.handle.send(newlobby.description_message.clone());
	}

	lobbies.push(newlobby);
}

fn handle_describe_lobby(
	lobby_id: Keycode,
	description_message: Message,
	clients: &mut Vec<Client>,
	lobbies: &mut Vec<Lobby>,
)
{
	let lobby = match lobbies.into_iter().find(|x| x.id == lobby_id)
	{
		Some(lobby) => lobby,
		None =>
		{
			warn!("Cannot describe missing lobby {:?}.", lobby_id);
			return;
		}
	};

	for client in clients.iter_mut()
	{
		client.handle.send(description_message.clone());
	}

	lobby.description_message = description_message;
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
		client.handle.send(message.clone())
	}
}

fn verify_lobby(
	lobby_id: Keycode,
	clients: &mut Vec<Client>,
	lobbies: &mut Vec<Lobby>,
)
{
	if let Some(lobby) = lobbies.iter_mut().find(|x| x.id == lobby_id)
	{
		if lobby.sendbuffer.try_send(lobby::Update::Pulse).is_ok()
		{
			return;
		}
		else
		{
			// Continue below.
		}
	}
	else
	{
		return;
	}

	// The lobby crashed, so we disband it now.
	handle_disband_lobby(lobby_id, clients, lobbies);
}

fn handle_find_lobby(
	lobbies: &mut Vec<Lobby>,
	lobby_id: Keycode,
	mut handle: client::Handle,
	general_chat: mpsc::Sender<Update>,
)
{
	let update = match lobbies.iter_mut().find(|x| x.id == lobby_id)
	{
		Some(lobby) => client::Update::LobbyFound {
			lobby_id,
			lobby_sendbuffer: lobby.sendbuffer.clone(),
			general_chat,
		},
		None => client::Update::LobbyNotFound { lobby_id },
	};
	handle.notify(update);
}

fn handle_in_game(
	clients: &mut Vec<Client>,
	lobby_id: Keycode,
	client_id: Keycode,
	role: Role,
)
{
	let found = clients
		.iter()
		.find(|client| client.id == client_id)
		.map(|client| client.username.clone());
	let username = match found
	{
		Some(username) => username,
		None => return,
	};
	let message = Message::InGame {
		lobby_id: lobby_id.to_string(),
		username,
		role,
	};
	for client in clients.iter_mut()
	{
		client.handle.send(message.clone());
	}
}
