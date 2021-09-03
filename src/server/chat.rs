/*
 * Part of epicinium_server
 * developed by A Bunch of Hacks.
 *
 * Copyright (c) 2018-2021 A Bunch of Hacks
 *
 * This library is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This library is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 * [authors:]
 * Sander in 't Veld (sander@abunchofhacks.coop)
 */

use crate::common::keycode::*;
use crate::logic::challenge;
use crate::server::client;
use crate::server::lobby;
use crate::server::login::Unlock;
use crate::server::message::*;
use crate::server::rating;

use std::collections::HashMap;

use log::*;

use tokio::sync::mpsc;
use tokio::sync::watch;

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
		rating_data: rating::Data,
		rating_and_stars: watch::Receiver<rating::RatingAndStars>,
		handle: client::Handle,
	},
	RatingAndStars
	{
		client_id: Keycode,
	},
	StillAlive
	{
		client_id: Keycode,
	},
	Leave
	{
		client_id: Keycode,
	},

	ListBot
	{
		bot: lobby::ConnectedAi,
	},
	UnlistBot
	{
		client_id: Keycode,
	},

	ListLobby
	{
		lobby_id: Keycode,
		name: watch::Receiver<String>,
		metadata: watch::Receiver<LobbyMetadata>,
		sendbuffer: mpsc::Sender<lobby::Update>,
	},
	DescribeLobby
	{
		lobby_id: Keycode,
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
		invite: Option<lobby::Invite>,
	},

	JoinedLobby
	{
		lobby_id: Keycode,
		client_id: Keycode,
	},
	LeftLobby
	{
		lobby_id: Keycode,
		client_id: Keycode,
	},
	InGame
	{
		lobby_id: Keycode,
		client_id: Keycode,
		role: Role,
	},

	Msg(Message),
}

pub async fn run(
	mut updates: mpsc::Receiver<Update>,
	canary: mpsc::Sender<()>,
	challenge_pool: &[challenge::Challenge],
)
{
	let mut clients: Vec<Client> = Vec::new();
	let mut ghostbusters: HashMap<Keycode, Ghostbuster> = HashMap::new();
	let mut lobbies: Vec<Lobby> = Vec::new();
	let mut bots = Vec::new();

	while let Some(update) = updates.recv().await
	{
		handle_update(
			update,
			&mut clients,
			&mut ghostbusters,
			&mut lobbies,
			&mut bots,
			challenge_pool,
		);

		let removed = clients
			.e_drain_where(|client| client.handle.is_disconnected())
			.collect();
		handle_removed(removed, &mut clients, &mut ghostbusters, &mut bots);
	}

	info!("General chat has disbanded.");
	let _discarded = canary;
}

fn handle_update(
	update: Update,
	clients: &mut Vec<Client>,
	ghostbusters: &mut HashMap<Keycode, Ghostbuster>,
	lobbies: &mut Vec<Lobby>,
	listed_bots: &mut Vec<lobby::ConnectedAi>,
	challenge_pool: &[challenge::Challenge],
)
{
	match update
	{
		Update::Join {
			client_id,
			username,
			unlocks,
			rating_data,
			rating_and_stars,
			handle,
		} => handle_join(
			client_id,
			username,
			unlocks,
			rating_data,
			rating_and_stars,
			handle,
			clients,
			ghostbusters,
			lobbies,
			listed_bots,
			challenge_pool,
		),
		Update::RatingAndStars { client_id } =>
		{
			handle_rating_and_stars(client_id, clients)
		}
		Update::StillAlive { client_id } =>
		{
			handle_still_alive(client_id, clients, ghostbusters)
		}
		Update::Leave { client_id } =>
		{
			handle_leave(client_id, clients, ghostbusters, listed_bots)
		}

		Update::ListBot { bot } =>
		{
			let lobby_ids: Vec<Keycode> =
				lobbies.iter().map(|x| x.id).collect();
			for lobby_id in lobby_ids
			{
				let update = lobby::Update::ForSetup(
					lobby::Sub::ListConnectedAi(bot.clone()),
				);
				notify_lobby_or_disband(lobby_id, clients, lobbies, update);
			}
			listed_bots.push(bot);
		}
		Update::UnlistBot { client_id } =>
		{
			listed_bots.retain(|x| x.client_id != client_id);
		}

		Update::ListLobby {
			lobby_id,
			name,
			metadata,
			sendbuffer,
		} =>
		{
			let lobby = Lobby {
				id: lobby_id,
				name,
				metadata,
				sendbuffer,
			};
			handle_list_lobby(lobby, clients, lobbies, listed_bots)
		}
		Update::DescribeLobby { lobby_id } =>
		{
			handle_describe_lobby(lobby_id, clients, lobbies)
		}
		Update::DisbandLobby { lobby_id } =>
		{
			handle_disband_lobby(lobby_id, clients, lobbies)
		}

		Update::FindLobby {
			lobby_id,
			handle,
			general_chat,
			invite,
		} =>
		{
			verify_lobby_or_disband(lobby_id, clients, lobbies);
			handle_find_lobby(lobbies, lobby_id, handle, general_chat, invite);
		}

		Update::JoinedLobby {
			lobby_id,
			client_id,
		} =>
		{
			handle_joined_lobby(clients, lobby_id, client_id);
		}
		Update::LeftLobby {
			lobby_id,
			client_id,
		} =>
		{
			handle_left_lobby(clients, lobby_id, client_id);
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
	join_metadata: JoinMetadataOrTagMetadata,
	handle: client::Handle,
	rating_and_stars: watch::Receiver<rating::RatingAndStars>,
	availability_status: AvailabilityStatus,
	hidden: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum AvailabilityStatus
{
	Available,
	InLobby
	{
		lobby_id: Keycode,
	},
	InGame
	{
		lobby_id: Keycode,
		role: Role,
	},
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
			metadata: Default::default(),
		};
		self.handle.send(message);
	}

	fn resolve(mut self)
	{
		debug!(
			"Client {} successfully ghostbusted client {}.",
			self.id, self.ghost_id
		);
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
	name: watch::Receiver<String>,
	metadata: watch::Receiver<LobbyMetadata>,
	sendbuffer: mpsc::Sender<lobby::Update>,
}

fn handle_join(
	id: Keycode,
	username: String,
	unlocks: EnumSet<Unlock>,
	rating_data: rating::Data,
	rating_and_stars: watch::Receiver<rating::RatingAndStars>,
	handle: client::Handle,
	clients: &mut Vec<Client>,
	ghostbusters: &mut HashMap<Keycode, Ghostbuster>,
	lobbies: &Vec<Lobby>,
	listed_bots: &mut Vec<lobby::ConnectedAi>,
	challenge_pool: &[challenge::Challenge],
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
	let hidden = username.starts_with('#');

	let mut newcomer = Client {
		id,
		username,
		join_metadata,
		handle,
		rating_and_stars,
		availability_status: AvailabilityStatus::Available,
		hidden,
	};

	// Confirm to the newcomer that they have joined.
	let message = Message::JoinServer {
		status: None,
		content: Some(newcomer.username.clone()),
		sender: None,
		metadata: newcomer.join_metadata.clone(),
	};
	newcomer.handle.send(message.clone());

	// Tell everyone who the newcomer is.
	if !newcomer.hidden
	{
		for other in clients.iter_mut()
		{
			other.handle.send(message.clone());
		}
	}

	// Tell the newcomer that they are online.
	newcomer.handle.send(message);

	do_init(
		&mut newcomer.handle,
		clients,
		lobbies,
		listed_bots,
		challenge_pool,
	);

	// Tell everyone the rating and stars of the newcomer.
	if !newcomer.hidden
	{
		let message = Message::RatingAndStars {
			username: newcomer.username.clone(),
			rating: rating_data.rating,
			stars: rating_data.stars,
		};
		for other in clients.iter_mut()
		{
			other.handle.send(message.clone());
		}
		newcomer.handle.send(message);
	}

	// Let the newcomer know how many stars they have for the current challenge.
	for (challenge_key, stars) in rating_data.stars_per_challenge
	{
		let message = Message::RecentStars {
			challenge_key,
			stars,
		};
		newcomer.handle.send(message);
	}

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

fn generate_join_metadata(
	unlocks: &EnumSet<Unlock>,
) -> JoinMetadataOrTagMetadata
{
	let mut metadata: TagMetadata = Default::default();
	if unlocks.contains(Unlock::Dev)
	{
		metadata.dev = true;
	}
	if unlocks.contains(Unlock::Guest)
	{
		metadata.guest = true;
	}
	if unlocks.contains(Unlock::Bot)
	{
		metadata.bot = true;
	}
	if unlocks.contains(Unlock::Supporter)
	{
		metadata.supporter = true;
	}

	JoinMetadataOrTagMetadata::TagMetadata(metadata)
}

fn do_init(
	handle: &mut client::Handle,
	clients: &Vec<Client>,
	lobbies: &Vec<Lobby>,
	_listed_bots: &mut Vec<lobby::ConnectedAi>,
	challenge_pool: &[challenge::Challenge],
)
{
	// Let the client know which lobbies there are.
	for lobby in lobbies.iter()
	{
		let message = Message::ListLobby {
			lobby_id: lobby.id,
			lobby_name: lobby.name.borrow().clone(),
			metadata: lobby.metadata.borrow().clone(),
		};
		handle.send(message);
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
				metadata: client.join_metadata.clone(),
			});

			let rating_data: rating::RatingAndStars =
				*client.rating_and_stars.borrow();
			let message = Message::RatingAndStars {
				username: client.username.clone(),
				rating: rating_data.rating,
				stars: rating_data.stars,
			};
			handle.send(message);

			match client.availability_status
			{
				AvailabilityStatus::Available =>
				{}
				AvailabilityStatus::InLobby { lobby_id: _ } =>
				{
					// Send JoinLobby without lobby_id because the lobby itself
					// also sends JoinLobby messages that have more importance.
					handle.send(Message::JoinLobby {
						username: Some(client.username.clone()),
						lobby_id: None,
						invite: None,
					});
				}
				AvailabilityStatus::InGame { lobby_id, role } =>
				{
					// Send JoinLobby without lobby_id because the lobby itself
					// also sends JoinLobby messages that have more importance.
					handle.send(Message::JoinLobby {
						username: Some(client.username.clone()),
						lobby_id: None,
						invite: None,
					});
					handle.send(Message::InGame {
						username: client.username.clone(),
						lobby_id,
						role,
					});
				}
			}
		}
	}

	// Let the client know what the current challenge is called.
	for challenge in challenge_pool
	{
		handle.send(Message::ListChallenge {
			key: challenge.key.clone(),
			metadata: challenge.metadata.clone(),
		});
	}

	// Let the client know we are done initializing.
	handle.send(Message::Init)
}

fn handle_rating_and_stars(client_id: Keycode, clients: &mut Vec<Client>)
{
	let client = match clients.iter_mut().find(|x| x.id == client_id)
	{
		Some(client) => client,
		None =>
		{
			warn!("Missing client {}.", client_id);
			return;
		}
	};
	let rating_data: rating::RatingAndStars = *client.rating_and_stars.borrow();
	let message = Message::RatingAndStars {
		username: client.username.clone(),
		rating: rating_data.rating,
		stars: rating_data.stars,
	};
	for client in clients.iter_mut()
	{
		client.handle.send(message.clone());
	}
}

fn handle_leave(
	client_id: Keycode,
	clients: &mut Vec<Client>,
	ghostbusters: &mut HashMap<Keycode, Ghostbuster>,
	listed_bots: &mut Vec<lobby::ConnectedAi>,
)
{
	let removed: Vec<Client> = clients
		.e_drain_where(|client| client.id == client_id)
		.collect();
	handle_removed(removed, clients, ghostbusters, listed_bots);
}

fn handle_removed(
	removed: Vec<Client>,
	clients: &mut Vec<Client>,
	ghostbusters: &mut HashMap<Keycode, Ghostbuster>,
	listed_bots: &mut Vec<lobby::ConnectedAi>,
)
{
	for removed_client in removed
	{
		let Client {
			id,
			username,
			join_metadata: _,
			mut handle,
			rating_and_stars: _,
			availability_status: _,
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

		listed_bots.retain(|x| x.client_id != id);
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
	mut newlobby: Lobby,
	clients: &mut Vec<Client>,
	lobbies: &mut Vec<Lobby>,
	listed_bots: &mut Vec<lobby::ConnectedAi>,
)
{
	lobbies.retain(|lobby| lobby.id != newlobby.id);

	for bot in listed_bots
	{
		let update =
			lobby::Update::ForSetup(lobby::Sub::ListConnectedAi(bot.clone()));
		match newlobby.sendbuffer.try_send(update)
		{
			Ok(()) => (),
			Err(_error) => return,
		}
	}

	describe_lobby(&newlobby, clients);

	lobbies.push(newlobby);
}

fn handle_describe_lobby(
	lobby_id: Keycode,
	clients: &mut Vec<Client>,
	lobbies: &mut Vec<Lobby>,
)
{
	let lobby = match lobbies.iter().find(|x| x.id == lobby_id)
	{
		Some(lobby) => lobby,
		None =>
		{
			warn!("Cannot describe missing lobby {:?}.", lobby_id);
			return;
		}
	};

	describe_lobby(lobby, clients);
}

fn describe_lobby(lobby: &Lobby, clients: &mut Vec<Client>)
{
	let message = Message::ListLobby {
		lobby_id: lobby.id,
		lobby_name: lobby.name.borrow().clone(),
		metadata: lobby.metadata.borrow().clone(),
	};
	for client in clients.iter_mut()
	{
		client.handle.send(message.clone());
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
		client.handle.send(message.clone())
	}
}

fn verify_lobby_or_disband(
	lobby_id: Keycode,
	clients: &mut Vec<Client>,
	lobbies: &mut Vec<Lobby>,
)
{
	notify_lobby_or_disband(lobby_id, clients, lobbies, lobby::Update::Pulse)
}

fn notify_lobby_or_disband(
	lobby_id: Keycode,
	clients: &mut Vec<Client>,
	lobbies: &mut Vec<Lobby>,
	update: lobby::Update,
)
{
	if let Some(lobby) = lobbies.iter_mut().find(|x| x.id == lobby_id)
	{
		if lobby.sendbuffer.try_send(update).is_ok()
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
	invite: Option<lobby::Invite>,
)
{
	let update = match lobbies.iter_mut().find(|x| x.id == lobby_id)
	{
		Some(lobby) => client::Update::LobbyFound {
			lobby_id,
			lobby_sendbuffer: lobby.sendbuffer.clone(),
			general_chat,
			invite,
		},
		None => client::Update::LobbyNotFound { lobby_id },
	};
	handle.notify(update);
}

fn handle_joined_lobby(
	clients: &mut Vec<Client>,
	lobby_id: Keycode,
	client_id: Keycode,
)
{
	let client = match clients.iter_mut().find(|x| x.id == client_id)
	{
		Some(client) => client,
		None => return,
	};

	client.availability_status = AvailabilityStatus::InLobby { lobby_id };

	// Send JoinLobby without lobby_id because the lobby itself
	// also sends JoinLobby messages that have more importance.
	let message = Message::JoinLobby {
		lobby_id: None,
		username: Some(client.username.clone()),
		invite: None,
	};
	for client in clients.iter_mut()
	{
		client.handle.send(message.clone());
	}
}

fn handle_left_lobby(
	clients: &mut Vec<Client>,
	_lobby_id: Keycode,
	client_id: Keycode,
)
{
	let client = match clients.iter_mut().find(|x| x.id == client_id)
	{
		Some(client) => client,
		None => return,
	};

	client.availability_status = AvailabilityStatus::Available;

	// Send LeaveLobby without lobby_id because the lobby itself
	// also sends LeaveLobby messages that have more importance.
	let message = Message::LeaveLobby {
		lobby_id: None,
		username: Some(client.username.clone()),
	};
	for client in clients.iter_mut()
	{
		client.handle.send(message.clone());
	}
}

fn handle_in_game(
	clients: &mut Vec<Client>,
	lobby_id: Keycode,
	client_id: Keycode,
	role: Role,
)
{
	let client = match clients.iter_mut().find(|x| x.id == client_id)
	{
		Some(client) => client,
		None => return,
	};

	client.availability_status = AvailabilityStatus::InGame { lobby_id, role };

	let message = Message::InGame {
		lobby_id,
		username: client.username.clone(),
		role,
	};
	for client in clients.iter_mut()
	{
		client.handle.send(message.clone());
	}
}
