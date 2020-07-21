/* Server::Lobby */

mod name;
mod secrets;

pub use secrets::Salts;
pub use secrets::Secrets;

use crate::common::keycode::*;
use crate::logic::ai;
use crate::logic::challenge;
use crate::logic::difficulty::*;
use crate::logic::map;
use crate::logic::player;
use crate::logic::player::PlayerColor;
use crate::logic::ruleset;
use crate::server::botslot;
use crate::server::botslot::Botslot;
use crate::server::chat;
use crate::server::client;
use crate::server::game;
use crate::server::login::UserId;
use crate::server::message::*;
use crate::server::rating;

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io;
use std::sync;
use std::sync::atomic;

use log::*;

use rand::Rng;

use tokio::sync::mpsc;

use vec_drain_where::VecDrainWhereExt;

#[derive(Debug)]
pub enum Update
{
	Pulse,

	Join
	{
		client_id: Keycode,
		client_user_id: UserId,
		client_username: String,
		client_handle: client::Handle,
		lobby_sendbuffer: mpsc::Sender<Update>,
		general_chat: mpsc::Sender<chat::Update>,
	},
	Leave
	{
		client_id: Keycode,
		general_chat: mpsc::Sender<chat::Update>,
	},

	ForSetup(Sub),

	ForGame(game::Sub),

	Msg(Message),
}

#[derive(Debug)]
pub enum Sub
{
	Save
	{
		lobby_sendbuffer: mpsc::Sender<Update>,
		general_chat: mpsc::Sender<chat::Update>,
	},

	Lock
	{
		general_chat: mpsc::Sender<chat::Update>,
	},
	Unlock
	{
		general_chat: mpsc::Sender<chat::Update>,
	},

	Rename
	{
		lobby_name: String,
		general_chat: mpsc::Sender<chat::Update>,
	},

	ClaimRole
	{
		general_chat: mpsc::Sender<chat::Update>,
		username: String,
		role: Role,
	},
	ClaimColor
	{
		username_or_slot: UsernameOrSlot,
		color: PlayerColor,
	},
	ClaimVisionType
	{
		username_or_slot: UsernameOrSlot,
		visiontype: VisionType,
	},
	ClaimAi
	{
		slot: Option<Botslot>,
		ai_name: String,
	},
	ClaimDifficulty
	{
		slot: Option<Botslot>,
		difficulty: Difficulty,
	},
	AddBot
	{
		general_chat: mpsc::Sender<chat::Update>,
	},
	RemoveBot
	{
		general_chat: mpsc::Sender<chat::Update>,
		slot: Botslot,
	},
	PickMap
	{
		general_chat: mpsc::Sender<chat::Update>,
		map_name: String,
	},
	PickTutorial
	{
		general_chat: mpsc::Sender<chat::Update>,
	},
	PickChallenge
	{
		general_chat: mpsc::Sender<chat::Update>,
	},
	PickTimer
	{
		seconds: u32
	},
	PickRuleset
	{
		ruleset_name: String
	},
	ConfirmRuleset
	{
		client_id: Keycode,
		ruleset_name: String,
		general_chat: mpsc::Sender<chat::Update>,
	},

	Start
	{
		general_chat: mpsc::Sender<chat::Update>,
	},
}

pub fn create(
	ticker: &mut sync::Arc<atomic::AtomicU64>,
	ratings: mpsc::Sender<rating::Update>,
	canary: mpsc::Sender<()>,
) -> mpsc::Sender<Update>
{
	let key = rand::random();
	let data = ticker.fetch_add(1, atomic::Ordering::Relaxed);
	let lobby_id = keycode(key, data);

	let (updates_in, updates_out) = mpsc::channel::<Update>(1000);

	let task = run(lobby_id, ratings, canary, updates_out);
	tokio::spawn(task);

	updates_in
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Stage
{
	Setup,
	WaitingForConfirmation,
}

#[derive(Debug, Clone)]
struct Lobby
{
	id: Keycode,
	name: String,
	num_players: i32,
	max_players: i32,
	is_public: bool,
	is_replay: bool,

	has_been_listed: bool,
	last_description_metadata: Option<LobbyMetadata>,

	bots: Vec<Bot>,
	open_botslots: Vec<Botslot>,
	roles: HashMap<Keycode, Role>,
	available_colors: Vec<PlayerColor>,
	player_colors: HashMap<Keycode, PlayerColor>,
	bot_colors: HashMap<Botslot, PlayerColor>,
	player_visiontypes: HashMap<Keycode, VisionType>,
	bot_visiontypes: HashMap<Botslot, VisionType>,

	ai_pool: Vec<String>,
	map_pool: Vec<(String, map::Metadata)>,
	map_name: String,
	ruleset_name: String,
	ruleset_confirmations: HashSet<Keycode>,
	timer_in_seconds: u32,
	challenge_id: Option<challenge::ChallengeId>,
	is_tutorial: bool,
	is_rated: bool,

	stage: Stage,
	rating_database_for_games: mpsc::Sender<rating::Update>,
}

async fn run(
	lobby_id: Keycode,
	ratings: mpsc::Sender<rating::Update>,
	canary: mpsc::Sender<()>,
	mut updates: mpsc::Receiver<Update>,
)
{
	let lobby = match initialize(lobby_id, ratings).await
	{
		Ok(lobby) => lobby,
		Err(error) =>
		{
			error!("Failed to create lobby: {:?}", error);
			return;
		}
	};

	let game = match run_setup(lobby, &mut updates).await
	{
		Ok(game) => game,
		Err(error) =>
		{
			error!("Lobby {} crashed: {:?}", lobby_id, error);
			return;
		}
	};

	if let Some(game) = game
	{
		debug!("Game started in lobby {}.", lobby_id);

		match game::run(game, updates).await
		{
			Ok(()) =>
			{}
			Err(error) =>
			{
				error!("Game crashed in lobby {}: {:?}", lobby_id, error);
				return;
			}
		}
	}

	debug!("Lobby {} has disbanded.", lobby_id);
	let _discarded = canary;
}

async fn initialize(
	lobby_id: Keycode,
	rating_database_for_games: mpsc::Sender<rating::Update>,
) -> Result<Lobby, Error>
{
	let map_pool = map::load_pool_with_metadata().await?;

	let defaultmap = map_pool.get(0).ok_or(Error::EmptyMapPool)?;
	let (name, _) = defaultmap;
	let map_name = name.to_string();

	Ok(Lobby {
		id: lobby_id,
		name: name::generate(),
		num_players: 0,
		max_players: 2,
		is_public: true,
		is_replay: false,
		has_been_listed: false,
		last_description_metadata: None,
		bots: Vec::new(),
		open_botslots: botslot::pool(),
		roles: HashMap::new(),
		available_colors: player::color_pool(),
		player_colors: HashMap::new(),
		bot_colors: HashMap::new(),
		player_visiontypes: HashMap::new(),
		bot_visiontypes: HashMap::new(),
		ai_pool: ai::load_pool(),
		map_pool,
		map_name,
		ruleset_name: ruleset::current(),
		ruleset_confirmations: HashSet::new(),
		timer_in_seconds: 60,
		challenge_id: None,
		is_tutorial: false,
		is_rated: true,
		stage: Stage::Setup,
		rating_database_for_games,
	})
}

async fn run_setup(
	mut lobby: Lobby,
	updates: &mut mpsc::Receiver<Update>,
) -> Result<Option<game::Setup>, Error>
{
	let mut clients: Vec<Client> = Vec::new();

	while let Some(update) = updates.recv().await
	{
		match handle_update(update, &mut lobby, &mut clients).await?
		{
			Some(game) => return Ok(Some(game)),
			None => continue,
		}
	}
	Ok(None)
}

async fn handle_update(
	update: Update,
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
) -> Result<Option<game::Setup>, Error>
{
	match update
	{
		Update::Join {
			client_id,
			client_user_id,
			client_username,
			client_handle,
			lobby_sendbuffer,
			mut general_chat,
		} =>
		{
			handle_join(
				lobby,
				client_id,
				client_user_id,
				client_username,
				client_handle,
				lobby_sendbuffer,
				&mut general_chat,
				clients,
			)
			.await?;
			Ok(None)
		}
		Update::Leave {
			client_id,
			mut general_chat,
		} =>
		{
			handle_leave(lobby, client_id, clients, &mut general_chat).await?;
			Ok(None)
		}

		Update::ForSetup(sub) => handle_sub(sub, lobby, clients).await,

		Update::ForGame(_) => Ok(None),

		Update::Msg(message) =>
		{
			for client in clients.iter_mut()
			{
				client.handle.send(message.clone());
			}
			Ok(None)
		}

		// The chat is making sure that this lobby still exists.
		Update::Pulse => Ok(None),
	}
}

async fn handle_sub(
	sub: Sub,
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
) -> Result<Option<game::Setup>, Error>
{
	match sub
	{
		Sub::Save {
			lobby_sendbuffer,
			mut general_chat,
		} =>
		{
			list_lobby(lobby, lobby_sendbuffer, &mut general_chat).await?;
			Ok(None)
		}

		Sub::Lock { mut general_chat } =>
		{
			lobby.is_public = false;
			describe_lobby(lobby, &mut general_chat).await?;
			Ok(None)
		}
		Sub::Unlock { mut general_chat } =>
		{
			lobby.is_public = true;
			describe_lobby(lobby, &mut general_chat).await?;
			Ok(None)
		}

		Sub::Rename {
			lobby_name,
			mut general_chat,
		} =>
		{
			lobby.name = lobby_name;
			// Unset the description metadata to force a lobby description.
			lobby.last_description_metadata = None;
			describe_lobby(lobby, &mut general_chat).await?;
			Ok(None)
		}

		Sub::ClaimRole {
			mut general_chat,
			username,
			role,
		} =>
		{
			handle_claim_role(lobby, clients, username, role)?;
			describe_lobby(lobby, &mut general_chat).await?;
			Ok(None)
		}
		Sub::ClaimColor {
			username_or_slot,
			color,
		} =>
		{
			change_color(lobby, clients, username_or_slot, color);
			Ok(None)
		}
		Sub::ClaimVisionType {
			username_or_slot,
			visiontype,
		} =>
		{
			change_visiontype(lobby, clients, username_or_slot, visiontype);
			Ok(None)
		}
		Sub::ClaimAi { slot, ai_name } =>
		{
			change_ai(lobby, clients, slot, ai_name);
			Ok(None)
		}
		Sub::ClaimDifficulty { slot, difficulty } =>
		{
			change_difficulty(lobby, clients, slot, difficulty);
			Ok(None)
		}

		Sub::AddBot { mut general_chat } =>
		{
			add_bot(lobby, clients);
			describe_lobby(lobby, &mut general_chat).await?;
			Ok(None)
		}
		Sub::RemoveBot {
			mut general_chat,
			slot,
		} =>
		{
			remove_bot(lobby, clients, slot);
			describe_lobby(lobby, &mut general_chat).await?;
			Ok(None)
		}

		Sub::PickMap {
			mut general_chat,
			map_name,
		} =>
		{
			pick_map(lobby, clients, map_name).await?;
			describe_lobby(lobby, &mut general_chat).await?;
			Ok(None)
		}
		Sub::PickTutorial { mut general_chat } =>
		{
			become_tutorial_lobby(lobby, clients).await?;
			describe_lobby(lobby, &mut general_chat).await?;
			Ok(None)
		}
		Sub::PickChallenge { mut general_chat } =>
		{
			become_challenge_lobby(lobby, clients).await?;
			describe_lobby(lobby, &mut general_chat).await?;
			Ok(None)
		}
		Sub::PickTimer { seconds } =>
		{
			pick_timer(lobby, clients, seconds).await?;
			Ok(None)
		}
		Sub::PickRuleset { ruleset_name } =>
		{
			pick_ruleset(lobby, clients, ruleset_name).await?;
			Ok(None)
		}

		Sub::ConfirmRuleset {
			client_id,
			ruleset_name,
			mut general_chat,
		} =>
		{
			handle_ruleset_confirmation(
				lobby,
				clients,
				client_id,
				ruleset_name,
				&mut general_chat,
			)
			.await
		}

		Sub::Start { mut general_chat } =>
		{
			try_start(lobby, clients, &mut general_chat).await
		}
	}
}

async fn list_lobby(
	lobby: &mut Lobby,
	lobby_sendbuffer: mpsc::Sender<Update>,
	general_chat: &mut mpsc::Sender<chat::Update>,
) -> Result<(), Error>
{
	if lobby.has_been_listed
	{
		trace!("Refusing to re-list lobby {}.", lobby.id);
		return Ok(());
	}

	let metadata = make_description_metadata(lobby);
	lobby.has_been_listed = true;
	lobby.last_description_metadata = Some(metadata);

	let description_message = Message::ListLobby {
		lobby_id: lobby.id,
		lobby_name: lobby.name.clone(),
		metadata,
	};
	let update = chat::Update::ListLobby {
		lobby_id: lobby.id,
		description_message,
		sendbuffer: lobby_sendbuffer,
	};
	general_chat.send(update).await?;
	Ok(())
}

async fn describe_lobby(
	lobby: &mut Lobby,
	general_chat: &mut mpsc::Sender<chat::Update>,
) -> Result<(), Error>
{
	if !lobby.has_been_listed
	{
		return Ok(());
	}

	let metadata = make_description_metadata(lobby);
	if lobby.last_description_metadata == Some(metadata)
	{
		return Ok(());
	}
	else
	{
		lobby.last_description_metadata = Some(metadata);
	}

	let description_message = Message::ListLobby {
		lobby_id: lobby.id,
		lobby_name: lobby.name.clone(),
		metadata,
	};
	let update = chat::Update::DescribeLobby {
		lobby_id: lobby.id,
		description_message,
	};
	general_chat.send(update).await?;
	Ok(())
}

fn make_description_metadata(lobby: &Lobby) -> LobbyMetadata
{
	debug_assert!(lobby.bots.len() <= 0xFF);
	let num_bot_players = lobby.bots.len() as i32;

	debug_assert!(lobby.is_replay == (lobby.max_players == 0));
	debug_assert!(lobby.num_players <= lobby.max_players);
	debug_assert!(num_bot_players <= lobby.num_players);

	LobbyMetadata {
		max_players: lobby.max_players,
		num_players: lobby.num_players,
		num_bot_players,
		is_public: lobby.is_public,
	}
}

#[derive(Debug, Clone)]
struct Bot
{
	slot: Botslot,
	ai_name: String,
	difficulty: Difficulty,
}

#[derive(Debug, Clone)]
struct Client
{
	id: Keycode,
	user_id: UserId,
	username: String,
	handle: client::Handle,
}

async fn handle_join(
	lobby: &mut Lobby,
	client_id: Keycode,
	client_user_id: UserId,
	client_username: String,
	client_handle: client::Handle,
	lobby_sendbuffer: mpsc::Sender<Update>,
	general_chat: &mut mpsc::Sender<chat::Update>,
	clients: &mut Vec<Client>,
) -> Result<(), Error>
{
	let mut handle_for_listing = client_handle.clone();

	match do_join(
		lobby,
		client_id,
		client_user_id,
		client_username.clone(),
		client_handle,
		lobby_sendbuffer,
		clients,
	)
	{
		Ok(()) => (),
		Err(()) => return Ok(()),
	}

	let message = Message::JoinLobby {
		lobby_id: Some(lobby.id),
		username: Some(client_username.clone()),
		metadata: None,
	};
	let update = chat::Update::Msg(message);
	general_chat.send(update).await?;

	// If the newcomer was invited, their role might be forced to be observer.
	// TODO forced role if joining through spectate secret
	let forced_role = None;
	change_role(lobby, clients, client_id, forced_role)?;

	// Describe the lobby to the client so that Discord presence is updated.
	let message = Message::ListLobby {
		lobby_id: lobby.id,
		lobby_name: lobby.name.clone(),
		metadata: make_description_metadata(lobby),
	};
	handle_for_listing.send(message);

	// Describe the lobby to the chat if the number of players changed.
	if Some(&Role::Player) == lobby.roles.get(&client_id)
	{
		describe_lobby(lobby, general_chat).await?;
	}

	// Send secrets for Discord invites and Ask-to-Join.
	send_secrets(lobby.id, client_id, clients)?;

	Ok(())
}

fn do_join(
	lobby: &mut Lobby,
	client_id: Keycode,
	client_user_id: UserId,
	client_username: String,
	client_handle: client::Handle,
	lobby_sendbuffer: mpsc::Sender<Update>,
	clients: &mut Vec<Client>,
) -> Result<(), ()>
{
	// TODO check invitation
	let is_invited = false;

	if !lobby.is_public && !is_invited
	{
		return Err(());
	}

	let mut newcomer = Client {
		id: client_id,
		user_id: client_user_id,
		username: client_username,
		handle: client_handle,
	};

	// Tell the newcomer which users are already in the lobby.
	for other in clients.into_iter()
	{
		newcomer.handle.send(Message::JoinLobby {
			lobby_id: Some(lobby.id),
			username: Some(other.username.clone()),
			metadata: None,
		});

		if let Some(&role) = lobby.roles.get(&other.id)
		{
			newcomer.handle.send(Message::ClaimRole {
				username: other.username.clone(),
				role,
			});
		}
		if let Some(&color) = lobby.player_colors.get(&other.id)
		{
			newcomer.handle.send(Message::ClaimColor {
				username_or_slot: UsernameOrSlot::Username(
					other.username.clone(),
				),
				color,
			});
		}
		if let Some(&visiontype) = lobby.player_visiontypes.get(&other.id)
		{
			newcomer.handle.send(Message::ClaimVisionType {
				username_or_slot: UsernameOrSlot::Username(
					other.username.clone(),
				),
				visiontype,
			});
		}
	}

	if !lobby.is_replay
	{
		// Tell the newcomer the AI pool.
		for name in &lobby.ai_pool
		{
			newcomer.handle.send(Message::ListAi {
				ai_name: name.clone(),
			});
		}
	}

	for bot in &lobby.bots
	{
		newcomer.handle.send(Message::AddBot {
			slot: Some(bot.slot),
		});
		newcomer.handle.send(Message::ClaimAi {
			slot: Some(bot.slot),
			ai_name: bot.ai_name.clone(),
		});
		newcomer.handle.send(Message::ClaimDifficulty {
			slot: Some(bot.slot),
			difficulty: bot.difficulty,
		});
		if let Some(&color) = lobby.bot_colors.get(&bot.slot)
		{
			newcomer.handle.send(Message::ClaimColor {
				username_or_slot: UsernameOrSlot::Slot(bot.slot),
				color,
			});
		}
		if let Some(&visiontype) = lobby.bot_visiontypes.get(&bot.slot)
		{
			newcomer.handle.send(Message::ClaimVisionType {
				username_or_slot: UsernameOrSlot::Slot(bot.slot),
				visiontype,
			});
		}
	}

	if !lobby.is_replay
	{
		for (mapname, metadata) in &lobby.map_pool
		{
			newcomer.handle.send(Message::ListMap {
				map_name: mapname.clone(),
				metadata: metadata.clone(),
			});
		}

		newcomer.handle.send(Message::PickMap {
			map_name: lobby.map_name.clone(),
		});
		newcomer.handle.send(Message::PickTimer {
			seconds: lobby.timer_in_seconds,
		});

		newcomer.handle.send(Message::ListRuleset {
			ruleset_name: lobby.ruleset_name.clone(),
		});
		newcomer.handle.send(Message::PickRuleset {
			ruleset_name: lobby.ruleset_name.clone(),
		});
	}
	else
	{
		// TODO list all recordings if this is a replay lobby
		// TODO other replay settings
	}

	let message = Message::JoinLobby {
		lobby_id: Some(lobby.id),
		username: Some(newcomer.username.clone()),
		metadata: None,
	};
	for client in clients.iter_mut()
	{
		client.handle.send(message.clone());
	}
	newcomer.handle.send(message);

	let update = client::Update::JoinedLobby {
		lobby: lobby_sendbuffer,
	};
	newcomer.handle.notify(update);

	clients.push(newcomer);

	Ok(())
}

fn send_secrets(
	lobby_id: Keycode,
	client_id: Keycode,
	clients: &mut Vec<Client>,
) -> Result<(), Error>
{
	let client = clients
		.iter_mut()
		.find(|x| x.id == client_id)
		.ok_or(Error::ClientMissing)?;
	client.handle.generate_and_send_secrets(lobby_id);
	Ok(())
}

async fn handle_leave(
	lobby: &mut Lobby,
	client_id: Keycode,
	clients: &mut Vec<Client>,
	general_chat: &mut mpsc::Sender<chat::Update>,
) -> Result<(), Error>
{
	let removed: Vec<Client> = clients
		.e_drain_where(|client| client.id == client_id)
		.collect();

	handle_removed(lobby, clients, removed).await?;

	if clients.is_empty()
	{
		let update = chat::Update::DisbandLobby { lobby_id: lobby.id };
		general_chat.send(update).await?;
	}
	else
	{
		describe_lobby(lobby, general_chat).await?;
	}

	Ok(())
}

async fn handle_removed(
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
	removed: Vec<Client>,
) -> Result<(), Error>
{
	for removed_client in removed
	{
		let Client {
			id,
			user_id: _,
			username,
			mut handle,
		} = removed_client;

		let message = Message::LeaveLobby {
			lobby_id: Some(lobby.id),
			username: Some(username),
		};

		for client in clients.iter_mut()
		{
			client.handle.send(message.clone());
		}

		handle.send(message);

		let removed_color = lobby.player_colors.remove(&id);
		if let Some(color) = removed_color
		{
			lobby.available_colors.push(color);
		}

		lobby.player_visiontypes.remove(&id);

		let removed_role = lobby.roles.remove(&id);
		if removed_role == Some(Role::Player)
		{
			lobby.num_players -= 1;
		}
	}

	Ok(())
}

fn handle_claim_role(
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
	username: String,
	role: Role,
) -> Result<(), Error>
{
	// Find any client based on the username supplied by the sender.
	let client = match clients.into_iter().find(|x| x.username == username)
	{
		Some(client) => client,
		None =>
		{
			// Client not found.
			// FUTURE let the sender know somehow?
			return Ok(());
		}
	};
	let subject_client_id = client.id;

	change_role(lobby, clients, subject_client_id, Some(role))
}

fn change_role(
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
	client_id: Keycode,
	preferred_role: Option<Role>,
) -> Result<(), Error>
{
	let client = clients
		.into_iter()
		.find(|x| x.id == client_id)
		.ok_or(Error::ClientMissing)?;
	let client_username = client.username.clone();

	let assigned_role = match preferred_role
	{
		Some(Role::Player) =>
		{
			if lobby.num_players < lobby.max_players
			{
				Role::Player
			}
			else
			{
				// Claim failed. The user keeps their original role.
				if let Some(&oldrole) = lobby.roles.get(&client_id)
				{
					let message = Message::ClaimRole {
						username: client_username,
						role: oldrole,
					};
					for client in clients.iter_mut()
					{
						client.handle.send(message.clone());
					}
				}
				return Ok(());
			}
		}
		Some(Role::Observer) => Role::Observer,
		None =>
		{
			if lobby.num_players < lobby.max_players
			{
				Role::Player
			}
			else
			{
				Role::Observer
			}
		}
	};

	let previous_role = lobby.roles.insert(client_id, assigned_role);
	match previous_role
	{
		Some(Role::Player) => lobby.num_players -= 1,
		Some(Role::Observer) => (),
		None => (),
	}

	let message = Message::ClaimRole {
		username: client_username,
		role: assigned_role,
	};
	for client in clients.iter_mut()
	{
		client.handle.send(message.clone());
	}

	if assigned_role == Role::Player
	{
		lobby.num_players += 1;
	}
	else
	{
		// The player loses their player color when they stop being a player.
		let removed_color = lobby.player_colors.remove(&client_id);
		if let Some(color) = removed_color
		{
			lobby.available_colors.push(color);
		}

		// If the lobby used to be an AI lobby, we keep it that way.
		// A lobby is an AI lobby in this sense if there is at most 1 human
		// and all other slots are filled by AI players.
		if lobby.bots.len() == lobby.num_players as usize
			&& lobby.num_players + 1 == lobby.max_players
		{
			add_bot(lobby, clients);
		}
	}

	Ok(())
}

fn change_color(
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
	username_or_slot: UsernameOrSlot,
	color: PlayerColor,
)
{
	let resulting_color = match &username_or_slot
	{
		UsernameOrSlot::Username(ref username) =>
		{
			let client = clients.iter().find(|x| &x.username == username);
			let client_id = match client
			{
				Some(client) => client.id,
				None =>
				{
					warn!("Failed to find client named {}.", username);
					// FUTURE let the sender know somehow?
					return;
				}
			};

			match lobby.roles.get(&client_id)
			{
				Some(Role::Player) =>
				{}
				Some(Role::Observer) | None =>
				{
					warn!("Cannot assign to non-player {}.", client_id);
					// FUTURE let the sender know somehow?
					return;
				}
			}

			if color == PlayerColor::None
			{
				// The player reliquishes their old claim.
				let removed_color = lobby.player_colors.remove(&client_id);
				if let Some(oldcolor) = removed_color
				{
					lobby.available_colors.push(oldcolor);
				}
				color
			}
			else if lobby.player_colors.get(&client_id) == Some(&color)
			{
				// The player already has this color.
				color
			}
			else if lobby.available_colors.contains(&color)
			{
				// Claim successful.
				lobby.available_colors.retain(|&x| x != color);
				lobby.player_colors.insert(client_id, color);
				color
			}
			else
			{
				// Claim failed. The player keeps their original color.
				match lobby.player_colors.get(&client_id)
				{
					Some(&oldcolor) => oldcolor,
					None => PlayerColor::None,
				}
			}
		}
		&UsernameOrSlot::Slot(slot) =>
		{
			if lobby.bots.iter_mut().find(|x| x.slot == slot).is_none()
			{
				warn!("Failed to find bot '{:?}'.", slot);
				// FUTURE let the sender know somehow?
				return;
			}

			if color == PlayerColor::None
			{
				// The bot reliquishes its old claim.
				let removed_color = lobby.bot_colors.remove(&slot);
				if let Some(oldcolor) = removed_color
				{
					lobby.available_colors.push(oldcolor);
				}
				color
			}
			else if lobby.bot_colors.get(&slot) == Some(&color)
			{
				// The bot already has this color.
				color
			}
			else if lobby.available_colors.contains(&color)
			{
				// Claim successful.
				lobby.available_colors.retain(|&x| x != color);
				lobby.bot_colors.insert(slot, color);
				color
			}
			else
			{
				// Claim failed. The bot keeps its original color.
				match lobby.bot_colors.get(&slot)
				{
					Some(&oldcolor) => oldcolor,
					None => PlayerColor::None,
				}
			}
		}
	};

	// Broadcast whatever the result of the claim was.
	for client in clients.into_iter()
	{
		client.handle.send(Message::ClaimColor {
			username_or_slot: username_or_slot.clone(),
			color: resulting_color,
		});
	}
}

fn change_visiontype(
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
	username_or_slot: UsernameOrSlot,
	visiontype: VisionType,
)
{
	match &username_or_slot
	{
		UsernameOrSlot::Username(ref username) =>
		{
			let client = clients.iter().find(|x| &x.username == username);
			let client_id = match client
			{
				Some(client) => client.id,
				None =>
				{
					warn!("Failed to find client named {}.", username);
					// FUTURE let the sender know somehow?
					return;
				}
			};

			match lobby.roles.get(&client_id)
			{
				Some(Role::Player) =>
				{}
				Some(Role::Observer) | None =>
				{
					warn!("Cannot assign to non-player {}.", client_id);
					// FUTURE let the sender know somehow?
					return;
				}
			}

			lobby.player_visiontypes.insert(client_id, visiontype);
		}
		&UsernameOrSlot::Slot(slot) =>
		{
			if lobby.bots.iter_mut().find(|x| x.slot == slot).is_none()
			{
				warn!("Failed to find bot '{:?}'.", slot);
				// FUTURE let the sender know somehow?
				return;
			}

			lobby.bot_visiontypes.insert(slot, visiontype);
		}
	};

	// Broadcast the claim.
	for client in clients.into_iter()
	{
		client.handle.send(Message::ClaimVisionType {
			username_or_slot: username_or_slot.clone(),
			visiontype,
		});
	}
}

fn change_ai(
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
	slot: Option<Botslot>,
	ai_name: String,
)
{
	let mut bot = {
		let found = match slot
		{
			Some(slot) => lobby.bots.iter_mut().find(|x| x.slot == slot),
			None => lobby.bots.last_mut(),
		};
		match found
		{
			Some(bot) => bot,
			None =>
			{
				warn!("Failed to find bot '{:?}'.", slot);
				// FUTURE let the sender know somehow?
				return;
			}
		}
	};

	if !ai::exists(&ai_name)
	{
		warn!("Cannot set AI to non-existing '{}'.", ai_name);
		// FUTURE let the sender know somehow?
		return;
	}

	if lobby.ai_pool.iter().find(|&x| x == &ai_name).is_none()
	{
		lobby.ai_pool.push(ai_name.clone());

		for client in clients.into_iter()
		{
			client.handle.send(Message::ListAi {
				ai_name: ai_name.clone(),
			});
		}
	}

	bot.ai_name = ai_name;

	for client in clients.into_iter()
	{
		client.handle.send(Message::ClaimAi {
			slot: Some(bot.slot),
			ai_name: bot.ai_name.clone(),
		});
	}
}

fn change_difficulty(
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
	slot: Option<Botslot>,
	difficulty: Difficulty,
)
{
	let mut bot = {
		let found = match slot
		{
			Some(slot) => lobby.bots.iter_mut().find(|x| x.slot == slot),
			None => lobby.bots.last_mut(),
		};
		match found
		{
			Some(bot) => bot,
			None =>
			{
				warn!("Failed to find bot '{:?}'.", slot);
				// FUTURE let the sender know somehow?
				return;
			}
		}
	};

	if difficulty == Difficulty::None && bot.ai_name != "Dummy"
	{
		warn!("Cannot send difficulty of AI '{}' to none.", bot.ai_name);
		// FUTURE let the sender know somehow?
		return;
	}

	bot.difficulty = difficulty;

	for client in clients.into_iter()
	{
		client.handle.send(Message::ClaimDifficulty {
			slot: Some(bot.slot),
			difficulty,
		});
	}
}

fn add_bot(lobby: &mut Lobby, clients: &mut Vec<Client>)
{
	if lobby.num_players >= lobby.max_players
	{
		warn!("Cannot add bot to lobby {}: lobby full", lobby.id);
		return;
	}

	let slot = {
		if lobby.open_botslots.is_empty()
		{
			warn!("Cannot add bot to lobby {}: all slots taken", lobby.id);
			return;
		}
		let mut rng = rand::thread_rng();
		let i = rng.gen_range(0, lobby.open_botslots.len());
		lobby.open_botslots.swap_remove(i)
	};

	let ai_name = match lobby.ai_pool.first()
	{
		Some(name) => name.clone(),
		None => "Dummy".to_string(),
	};
	let bot = Bot {
		slot,
		ai_name,
		difficulty: Difficulty::Medium,
	};

	for client in clients.into_iter()
	{
		client.handle.send(Message::AddBot {
			slot: Some(bot.slot),
		});
		client.handle.send(Message::ClaimAi {
			slot: Some(bot.slot),
			ai_name: bot.ai_name.clone(),
		});
		client.handle.send(Message::ClaimDifficulty {
			slot: Some(bot.slot),
			difficulty: bot.difficulty,
		});
	}

	lobby.bots.push(bot);
	lobby.num_players += 1;
}

fn remove_bot(lobby: &mut Lobby, clients: &mut Vec<Client>, slot: Botslot)
{
	if lobby.bots.iter().find(|x| x.slot == slot).is_some()
	{
		lobby.bots.retain(|x| x.slot != slot);
		lobby.num_players -= 1;
		lobby.open_botslots.push(slot);

		let removed_color = lobby.bot_colors.remove(&slot);
		if let Some(color) = removed_color
		{
			lobby.available_colors.push(color);
		}

		lobby.bot_visiontypes.remove(&slot);

		for client in clients.into_iter()
		{
			client.handle.send(Message::RemoveBot { slot });
		}
	}
}

async fn pick_map(
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
	map_name: String,
) -> Result<(), Error>
{
	// FUTURE check if client is host

	// Is this a game lobby?
	if lobby.is_replay
	{
		warn!("Cannot pick map for replay lobby {}.", lobby.id);
		return Ok(());
	}

	let found = lobby.map_pool.iter().find(|&(x, _)| *x == map_name);

	let found = if found.is_some()
	{
		found
	}
	// FUTURE check if map in hidden pool or client is developer
	else if map::exists(&map_name)
	{
		let metadata = map::load_metadata(&map_name).await?;

		let message = Message::ListMap {
			map_name: map_name.clone(),
			metadata: metadata.clone(),
		};
		for client in clients.iter_mut()
		{
			client.handle.send(message.clone());
		}

		lobby.map_pool.push((map_name.clone(), metadata));
		lobby.map_pool.last()
	}
	else if map_name.len() >= 3
	{
		// Partial search for '1v1' or 'ffa'.
		let pat = &map_name;
		lobby.map_pool.iter().find(|&(x, _)| x.find(pat).is_some())
	}
	else
	{
		None
	};

	let (name, metadata) = match found
	{
		Some(x) => x,
		None =>
		{
			// Pick failed, send the current map.
			warn!(
				"Cannot pick non-existing map '{}' in lobby {}.",
				map_name, lobby.id
			);
			let message = Message::PickMap {
				map_name: lobby.map_name.clone(),
			};
			for client in clients.iter_mut()
			{
				client.handle.send(message.clone());
			}
			return Ok(());
		}
	};

	lobby.map_name = name.to_string();

	let message = Message::PickMap {
		map_name: lobby.map_name.clone(),
	};
	for client in clients.iter_mut()
	{
		client.handle.send(message.clone());
	}

	let playercount = match metadata.playercount()
	{
		Some(count) => count as i32,
		None => return Err(Error::NoPlayerCount),
	};

	// We may need to demote players to observers.
	let oldcount = lobby.max_players;
	let mut newcount = 0;
	let mut humancount = 0;
	let mut botcount = 0;
	let mut demotions = Vec::new();
	for client in clients.into_iter()
	{
		if lobby.roles.get(&client.id) == Some(&Role::Player)
		{
			if newcount < playercount
			{
				newcount += 1;
				humancount += 1;
			}
			else
			{
				demotions.push(client.id);
			}
		}
	}
	for id in demotions
	{
		change_role(lobby, clients, id, Some(Role::Observer))?;
	}

	// We may need to remove bots.
	let mut removals = Vec::new();
	for bot in &lobby.bots
	{
		if newcount < playercount
		{
			newcount += 1;
			botcount += 1;
		}
		else
		{
			removals.push(bot.slot);
		}
	}
	for slot in removals
	{
		remove_bot(lobby, clients, slot);
	}

	// We have a new playercount.
	lobby.max_players = playercount;

	// If the lobby used to be an AI lobby, we keep it that way.
	// A lobby is an AI lobby in this sense if there is at most 1 human
	// and all other slots are filled by AI players.
	if humancount <= 1 && humancount + botcount == oldcount
	{
		for _ in oldcount..playercount
		{
			add_bot(lobby, clients);
		}
	}

	Ok(())
}

async fn become_tutorial_lobby(
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
) -> Result<(), Error>
{
	// FUTURE check if client is host

	// Is this a game lobby?
	if lobby.is_replay
	{
		warn!("Cannot turn replay lobby {} into tutorial.", lobby.id);
		return Ok(());
	}

	// Prevent lobbies from being turned to tutorial lobbies if there are
	// multiple human players present.
	if clients.len() > 1
	{
		warn!(
			"Cannot turn lobby {} with {} clients into tutorial.",
			lobby.id,
			clients.len()
		);
		return Ok(());
	}

	let client_id = match clients.first()
	{
		Some(client) => client.id,
		None =>
		{
			warn!(
				"Cannot turn lobby {} without clients into tutorial.",
				lobby.id,
			);
			return Ok(());
		}
	};

	pick_map(lobby, clients, "tutorial".to_string()).await?;
	pick_timer(lobby, clients, 0).await?;

	lobby
		.player_visiontypes
		.insert(client_id, VisionType::Global);

	let ai_name = "TutorialTurtle";
	let difficulty = Difficulty::Easy;
	let num = 1;
	for _ in 0..num
	{
		add_bot(lobby, clients);
		change_ai(lobby, clients, None, ai_name.to_string());
		change_difficulty(lobby, clients, None, difficulty);
	}

	lobby.is_tutorial = true;

	Ok(())
}

async fn become_challenge_lobby(
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
) -> Result<(), Error>
{
	// FUTURE check if client is host

	// Is this a game lobby?
	if lobby.is_replay
	{
		warn!("Cannot turn replay lobby {} into challenge.", lobby.id);
		return Ok(());
	}

	// Prevent lobbies from being turned to challenge lobbies if there are
	// multiple human players present.
	if clients.len() > 1
	{
		warn!(
			"Cannot turn lobby {} with {} clients into challenge.",
			lobby.id,
			clients.len()
		);
		return Ok(());
	}

	// Challenges are unrated because you cannot get 100 points.
	lobby.is_rated = false;

	let id = challenge::current_id();
	let challenge_key = challenge::get_current_key();

	lobby.challenge_id = Some(id);

	for client in clients.iter_mut()
	{
		client.handle.send(Message::PickChallenge {
			challenge_key: challenge_key.clone(),
		});
	}

	pick_map(lobby, clients, challenge::map_name(id)).await?;
	match challenge::ruleset_name(id)
	{
		Some(ruleset_name) =>
		{
			pick_ruleset(lobby, clients, ruleset_name).await?;
		}
		None =>
		{}
	}
	pick_timer(lobby, clients, 0).await?;

	let ai_name = challenge::bot_name(id);
	let difficulty = challenge::bot_difficulty(id);
	let num = challenge::num_bots(id);
	for _ in 0..num
	{
		add_bot(lobby, clients);
		change_ai(lobby, clients, None, ai_name.clone());
		change_difficulty(lobby, clients, None, difficulty);
	}

	Ok(())
}

async fn pick_timer(
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
	timer_in_seconds: u32,
) -> Result<(), Error>
{
	// FUTURE check if client is host

	// Is this a game lobby?
	if lobby.is_replay
	{
		return Ok(());
	}

	// Not much need for extreme validation. Current hard cap 5 minutes.
	if timer_in_seconds > 300
	{
		// Do not change lobby timer.
	}
	else
	{
		lobby.timer_in_seconds = timer_in_seconds;
	}

	let message = Message::PickTimer {
		seconds: timer_in_seconds,
	};
	for client in clients.iter_mut()
	{
		client.handle.send(message.clone());
	}

	Ok(())
}

async fn pick_ruleset(
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
	ruleset_name: String,
) -> Result<(), Error>
{
	// FUTURE check if client is host
	// FUTURE check if ruleset in pool or client is developer or replay

	// Is this a game lobby?
	if lobby.is_replay
	{
		warn!("Cannot pick ruleset in replay lobby {}.", lobby.id);
		return Ok(());
	}

	// Maybe give an error here? For now, this is used because AIChallenge
	// might want to use the default ruleset.
	if ruleset_name.is_empty()
	{
		lobby.ruleset_name = ruleset::current();
	}
	else if ruleset::exists(&ruleset_name)
	{
		lobby.ruleset_name = ruleset_name;
	}
	else
	{
		// Pick failed, send the current ruleset.
		warn!(
			"Cannot pick non-existing ruleset '{}' in lobby {}.",
			ruleset_name, lobby.id
		);
		let message = Message::PickRuleset {
			ruleset_name: lobby.ruleset_name.clone(),
		};
		for client in clients.iter_mut()
		{
			client.handle.send(message.clone());
		}
		return Ok(());
	}

	// All players have to confirm that they have this ruleset.
	lobby.ruleset_confirmations.clear();

	// Before picking, list the new ruleset to trigger the confirmations.
	// This is a bit redundant, I guess, but in the future we might want to
	// have an actual ruleset dropdown, in which case we do not want to have to
	// reconfirm the ruleset every time it is picked, just once it is listed.
	let listmessage = Message::ListRuleset {
		ruleset_name: lobby.ruleset_name.clone(),
	};
	let pickmessage = Message::PickRuleset {
		ruleset_name: lobby.ruleset_name.clone(),
	};
	for client in clients.iter_mut()
	{
		client.handle.send(listmessage.clone());
		client.handle.send(pickmessage.clone());
	}

	Ok(())
}

async fn handle_ruleset_confirmation(
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
	client_id: Keycode,
	ruleset_name: String,
	general_chat: &mut mpsc::Sender<chat::Update>,
) -> Result<Option<game::Setup>, Error>
{
	if ruleset_name != lobby.ruleset_name
	{
		debug!(
			"Ignoring confirmation for ruleset '{}' \
			 when current ruleset is '{}'.",
			ruleset_name, lobby.ruleset_name
		);
		return Ok(None);
	}

	if lobby.ruleset_confirmations.contains(&client_id)
	{
		return Ok(None);
	}

	lobby.ruleset_confirmations.insert(client_id);

	if lobby.stage == Stage::WaitingForConfirmation
		&& is_ruleset_confirmed(lobby, clients)
	{
		// Start the game once everyone has confirmed.
		try_start(lobby, clients, general_chat).await
	}
	else
	{
		Ok(None)
	}
}

fn is_ruleset_confirmed(lobby: &Lobby, clients: &Vec<Client>) -> bool
{
	for client in clients
	{
		if !lobby.ruleset_confirmations.contains(&client.id)
		{
			return false;
		}
	}
	return true;
}

async fn try_start(
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
	general_chat: &mut mpsc::Sender<chat::Update>,
) -> Result<Option<game::Setup>, Error>
{
	// Make sure all the clients are still valid.
	let client_count = clients.len();
	let removed = clients
		.e_drain_where(|client| client.handle.is_disconnected())
		.collect();
	handle_removed(lobby, clients, removed).await?;

	if clients.len() < 1
	{
		let update = chat::Update::DisbandLobby { lobby_id: lobby.id };
		general_chat.send(update).await?;

		debug!("Disbanding lobby {}: no clients at game start.", lobby.id);
		return Ok(None);
	}
	else if clients.len() < client_count
	{
		describe_lobby(lobby, general_chat).await?;
	}

	if lobby.num_players < lobby.max_players
	{
		debug!("Cannot start lobby {}: not enough players.", lobby.id);
		return Ok(None);
	}

	// Create a stack of available colors, with Red on top, then Blue, etcetera.
	let mut colorstack = lobby.available_colors.clone();
	colorstack.sort();
	colorstack.reverse();

	// Assign colors and roles to clients.
	let mut player_clients = Vec::new();
	let mut watcher_clients = Vec::new();
	for client in clients.iter()
	{
		let role = match lobby.roles.get(&client.id)
		{
			Some(&role) => role,
			None =>
			{
				lobby.roles.insert(client.id, Role::Observer);
				Role::Observer
			}
		};

		match role
		{
			Role::Player =>
			{
				let color = match lobby.player_colors.get(&client.id)
				{
					Some(&color) => color,
					None => match colorstack.pop()
					{
						Some(color) => color,
						None =>
						{
							return Err(Error::StartGameNotEnoughColors);
						}
					},
				};

				let vision = match lobby.player_visiontypes.get(&client.id)
				{
					Some(&vision) => vision,
					None => VisionType::Normal,
				};

				let rating_callback = lobby.rating_database_for_games.clone();

				player_clients.push(game::PlayerClient {
					id: client.id,
					user_id: client.user_id,
					username: client.username.clone(),
					handle: client.handle.clone(),
					rating_callback: Some(rating_callback),

					color,
					vision,

					is_defeated: false,
					has_synced: false,
					submitted_orders: None,
				});
			}
			Role::Observer =>
			{
				watcher_clients.push(game::WatcherClient {
					id: client.id,
					user_id: client.user_id,
					username: client.username.clone(),
					handle: client.handle.clone(),

					role,
					vision_level: role.vision_level(),

					has_synced: false,
				});
			}
		}
	}

	// Assign colors and roles to bots.
	let mut bots = Vec::new();
	for bot in lobby.bots.iter()
	{
		let color = match lobby.bot_colors.get(&bot.slot)
		{
			Some(&color) => color,
			None => match colorstack.pop()
			{
				Some(color) => color,
				None =>
				{
					return Err(Error::StartGameNotEnoughColors);
				}
			},
		};

		let vision = match lobby.bot_visiontypes.get(&bot.slot)
		{
			Some(&vision) => vision,
			None => VisionType::Normal,
		};

		let character = bot.slot.get_character();

		let allocated_ai = ai::Commander::create(
			&bot.ai_name,
			color,
			bot.difficulty,
			&lobby.ruleset_name,
			character,
		);
		let ai = match allocated_ai
		{
			Ok(ai) => ai,
			Err(error) => return Err(Error::AiAllocationError { error }),
		};

		bots.push(game::Bot {
			slot: bot.slot,
			ai,

			color,
			vision,

			is_defeated: false,
		});
	}

	// TODO if (_replay && _replayname.empty()) return false;

	// Check that all clients have access to the ruleset that we will use.
	if !is_ruleset_confirmed(lobby, clients)
	{
		debug!("Delaying start in lobby {}: ruleset unconfirmed.", lobby.id);

		// List the new ruleset to trigger additional confirmations.
		let message = Message::ListRuleset {
			ruleset_name: lobby.ruleset_name.clone(),
		};
		for client in clients.iter_mut()
		{
			client.handle.send(message.clone());
		}

		// Cannot continue until all player have confirmed the ruleset.
		lobby.stage = Stage::WaitingForConfirmation;
		return Ok(None);
	}

	let map_name = lobby.map_name.clone();
	let map_metadata = {
		match lobby.map_pool.iter().find(|(name, _)| name == &map_name)
		{
			Some((_, metadata)) => metadata.clone(),
			None => map::load_metadata(&map_name).await?,
		}
	};

	let planning_timer = Some(lobby.timer_in_seconds).filter(|&x| x > 0);

	// We are truly starting.
	let game = game::Setup {
		lobby_id: lobby.id,
		lobby_name: lobby.name.clone(),
		lobby_description_metadata: make_description_metadata(lobby),
		players: player_clients,
		bots,
		watchers: watcher_clients,
		map_name,
		map_metadata,
		ruleset_name: lobby.ruleset_name.clone(),
		planning_time_in_seconds: planning_timer,
		challenge: lobby.challenge_id,
		is_tutorial: lobby.is_tutorial,
		is_rated: lobby.is_rated,
		is_public: lobby.is_public,
	};

	for client in clients.iter()
	{
		let role = match lobby.roles.get(&client.id)
		{
			Some(&role) => role,
			None =>
			{
				debug_assert!(false, "role should be assigned above");
				continue;
			}
		};

		let update = chat::Update::InGame {
			lobby_id: lobby.id,
			client_id: client.id,
			role,
		};
		general_chat.send(update).await?;
	}

	Ok(Some(game))
}

#[derive(Debug)]
enum Error
{
	EmptyMapPool,
	NoPlayerCount,
	ClientMissing,
	StartGameNotEnoughColors,
	Io
	{
		error: io::Error,
	},
	GeneralChat
	{
		error: mpsc::error::SendError<chat::Update>,
	},
	AiAllocationError
	{
		error: ai::InterfaceError,
	},
}

impl From<io::Error> for Error
{
	fn from(error: io::Error) -> Self
	{
		Error::Io { error }
	}
}

impl From<mpsc::error::SendError<chat::Update>> for Error
{
	fn from(error: mpsc::error::SendError<chat::Update>) -> Self
	{
		Error::GeneralChat { error }
	}
}

impl fmt::Display for Error
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
	{
		match self
		{
			Error::EmptyMapPool => write!(f, "{:#?}", self),
			Error::NoPlayerCount => write!(f, "{:#?}", self),
			Error::ClientMissing => write!(f, "{:#?}", self),
			Error::StartGameNotEnoughColors => write!(f, "{:#?}", self),
			Error::Io { error } => error.fmt(f),
			Error::GeneralChat { error } => error.fmt(f),
			Error::AiAllocationError { error } =>
			{
				write!(f, "Error while allocating AI: {}", error)
			}
		}
	}
}

impl std::error::Error for Error {}
