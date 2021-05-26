/* Server::Lobby */

mod name;
mod secrets;

pub use secrets::Invite;
pub use secrets::Salts;
pub use secrets::Secrets;

use crate::common::keycode::*;
use crate::logic::ai;
use crate::logic::challenge;
use crate::logic::difficulty::*;
use crate::logic::map;
use crate::logic::player;
use crate::logic::player::PlayerColor;
use crate::logic::player::PLAYER_MAX;
use crate::logic::ruleset;
use crate::server::botslot;
use crate::server::botslot::Botslot;
use crate::server::chat;
use crate::server::client;
use crate::server::discord_api;
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

use serde_derive::{Deserialize, Serialize};
use serde_json::json;

use tokio::sync::mpsc;
use tokio::sync::watch;

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
		desired_metadata: Option<LobbyMetadata>,
		invite: Option<Invite>,
	},
	Leave
	{
		client_id: Keycode,
		general_chat: mpsc::Sender<chat::Update>,
	},

	ForSetup(Sub),

	ForGame(game::Sub),
	FromHost(game::FromHost),

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

	ListConnectedAi(ConnectedAi),

	ClaimHost
	{
		general_chat: mpsc::Sender<chat::Update>,
		username: String,
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
		username_or_slot: UsernameOrSlot,
		ai_name: String,
	},
	ClaimDifficulty
	{
		username_or_slot: UsernameOrSlot,
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
	EnableCustomMaps
	{
		general_chat: mpsc::Sender<chat::Update>,
	},
	PickMap
	{
		general_chat: mpsc::Sender<chat::Update>,
		map_name: String,
	},
	PickTimer
	{
		seconds: u32,
	},
	PickRuleset
	{
		ruleset_name: String,
	},
	ConfirmRuleset
	{
		client_id: Keycode,
		ruleset_name: String,
		general_chat: mpsc::Sender<chat::Update>,
		lobby_sendbuffer: mpsc::Sender<Update>,
	},

	Start
	{
		general_chat: mpsc::Sender<chat::Update>,
		lobby_sendbuffer: mpsc::Sender<Update>,
	},
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LobbyType
{
	Generic,
	OneVsOne,
	Custom,
	Tutorial,
	Challenge,
	Replay,
}

impl Default for LobbyType
{
	fn default() -> LobbyType
	{
		LobbyType::Generic
	}
}

#[derive(Debug, Clone)]
pub struct ConnectedAi
{
	pub client_id: Keycode,
	pub client_user_id: UserId,
	pub client_username: String,
	pub handle: client::Handle,
	pub ai_name: String,
	pub authors: String,
}

pub fn create(
	ticker: &mut sync::Arc<atomic::AtomicU64>,
	ratings: mpsc::Sender<rating::Update>,
	discord_api: mpsc::Sender<discord_api::Post>,
	canary: mpsc::Sender<()>,
) -> mpsc::Sender<Update>
{
	let key = rand::random();
	let data = ticker.fetch_add(1, atomic::Ordering::Relaxed);
	let lobby_id = keycode(key, data);

	let (updates_in, updates_out) = mpsc::channel::<Update>(1000);

	let task = run(lobby_id, ratings, discord_api, canary, updates_out);
	tokio::spawn(task);

	updates_in
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Stage
{
	Setup,
	WaitingForConfirmation,
}

#[derive(Debug)]
struct Listing
{
	name: watch::Sender<String>,
	metadata: watch::Sender<LobbyMetadata>,
	last_sent_metadata: LobbyMetadata,
}

#[derive(Debug)]
struct Host
{
	id: Keycode,
	username: String,
}

#[derive(Debug)]
struct Lobby
{
	id: Keycode,
	name: String,
	num_players: i32,
	max_players: i32,
	lobby_type: LobbyType,
	is_public: bool,

	listing: Option<Listing>,

	bots: Vec<Bot>,
	open_botslots: Vec<Botslot>,
	roles: HashMap<Keycode, Role>,
	available_colors: Vec<PlayerColor>,
	player_colors: HashMap<Keycode, PlayerColor>,
	bot_colors: HashMap<Botslot, PlayerColor>,
	player_visiontypes: HashMap<Keycode, VisionType>,
	bot_visiontypes: HashMap<Botslot, VisionType>,

	host: Option<Host>,
	ai_pool: Vec<(String, Option<BotAuthorsMetadata>)>,
	ai_name_blockers: Vec<String>,
	connected_ais: Vec<ConnectedAi>,
	staged_connected_ais: Vec<ConnectedAi>,
	map_pool: Vec<(String, map::Metadata)>,
	map_name: String,
	ruleset_name: String,
	ruleset_confirmations: HashSet<Keycode>,
	timer_in_seconds: u32,
	challenge_id: Option<challenge::ChallengeId>,

	stage: Stage,
	rating_database_for_games: mpsc::Sender<rating::Update>,
}

async fn run(
	lobby_id: Keycode,
	ratings: mpsc::Sender<rating::Update>,
	discord_api: mpsc::Sender<discord_api::Post>,
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

		match game::run(game, discord_api, updates).await
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
	let ai_pool = ai::load_pool().into_iter().map(|x| (x, None)).collect();

	let map_pool = map::load_pool_with_metadata().await?;

	let defaultmap = map_pool.get(0).ok_or(Error::EmptyMapPool)?;
	let (name, _) = defaultmap;
	let map_name = name.to_string();

	Ok(Lobby {
		id: lobby_id,
		name: name::generate(),
		num_players: 0,
		max_players: 2,
		lobby_type: LobbyType::Generic,
		is_public: true,
		listing: None,
		bots: Vec::new(),
		open_botslots: botslot::pool(),
		roles: HashMap::new(),
		available_colors: player::color_pool(),
		player_colors: HashMap::new(),
		bot_colors: HashMap::new(),
		player_visiontypes: HashMap::new(),
		bot_visiontypes: HashMap::new(),
		host: None,
		ai_pool,
		ai_name_blockers: Vec::new(),
		connected_ais: Vec::new(),
		staged_connected_ais: Vec::new(),
		map_pool,
		map_name,
		ruleset_name: ruleset::current(),
		ruleset_confirmations: HashSet::new(),
		timer_in_seconds: 60,
		challenge_id: None,
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
			desired_metadata,
			invite,
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
				desired_metadata,
				invite,
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
		Update::FromHost(_) => Ok(None),

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
			rename_lobby(lobby, lobby_name, &mut general_chat).await?;
			Ok(None)
		}

		Sub::ListConnectedAi(ai) =>
		{
			add_ai_to_list(lobby, clients, ai).await;
			Ok(None)
		}

		Sub::ClaimHost {
			mut general_chat,
			username,
		} =>
		{
			handle_claim_host(lobby, clients, username)?;
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
		Sub::ClaimAi {
			username_or_slot,
			ai_name,
		} =>
		{
			change_ai(lobby, clients, username_or_slot, ai_name);
			Ok(None)
		}
		Sub::ClaimDifficulty {
			username_or_slot,
			difficulty,
		} =>
		{
			change_difficulty(lobby, clients, username_or_slot, difficulty);
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

		Sub::EnableCustomMaps { mut general_chat } =>
		{
			become_custom_lobby(lobby, clients).await?;
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
			lobby_sendbuffer,
		} =>
		{
			handle_ruleset_confirmation(
				lobby,
				clients,
				client_id,
				ruleset_name,
				&mut general_chat,
				lobby_sendbuffer,
			)
			.await
		}

		Sub::Start {
			mut general_chat,
			lobby_sendbuffer,
		} =>
		{
			try_start(lobby, clients, &mut general_chat, lobby_sendbuffer).await
		}
	}
}

async fn list_lobby(
	lobby: &mut Lobby,
	lobby_sendbuffer: mpsc::Sender<Update>,
	general_chat: &mut mpsc::Sender<chat::Update>,
) -> Result<(), Error>
{
	if lobby.listing.is_some()
	{
		trace!("Refusing to re-list lobby {}.", lobby.id);
		return Ok(());
	}

	let metadata = make_description_metadata(lobby);
	let (dm_in, dm_out) = watch::channel(metadata);
	let (name_in, name_out) = watch::channel(lobby.name.clone());
	let listing = Listing {
		name: name_in,
		metadata: dm_in,
		last_sent_metadata: metadata,
	};
	lobby.listing = Some(listing);

	let update = chat::Update::ListLobby {
		lobby_id: lobby.id,
		name: name_out,
		metadata: dm_out,
		sendbuffer: lobby_sendbuffer,
	};
	general_chat.send(update).await?;
	Ok(())
}

async fn rename_lobby(
	lobby: &mut Lobby,
	new_name: String,
	general_chat: &mut mpsc::Sender<chat::Update>,
) -> Result<(), Error>
{
	lobby.name = new_name;

	if let Some(Listing { name, .. }) = &mut lobby.listing
	{
		name.broadcast(lobby.name.clone())?;
		let update = chat::Update::DescribeLobby { lobby_id: lobby.id };
		general_chat.send(update).await?;
	}
	Ok(())
}

async fn describe_lobby(
	lobby: &mut Lobby,
	general_chat: &mut mpsc::Sender<chat::Update>,
) -> Result<(), Error>
{
	let metadata = make_description_metadata(lobby);
	if let Some(listing) = &mut lobby.listing
	{
		if metadata != listing.last_sent_metadata
		{
			listing.metadata.broadcast(metadata)?;
			listing.last_sent_metadata = metadata;
			let update = chat::Update::DescribeLobby { lobby_id: lobby.id };
			general_chat.send(update).await?;
		}
	}
	Ok(())
}

fn make_description_metadata(lobby: &Lobby) -> LobbyMetadata
{
	debug_assert!(lobby.bots.len() <= 0xFF);
	let num_bot_players = lobby.bots.len() as i32;

	match lobby.lobby_type
	{
		LobbyType::Generic | LobbyType::Custom =>
		{
			debug_assert!(lobby.max_players > 0);
		}
		LobbyType::OneVsOne =>
		{
			debug_assert!(lobby.max_players == 2);
		}
		LobbyType::Challenge | LobbyType::Tutorial =>
		{
			debug_assert!(lobby.max_players == 2);
			debug_assert!(num_bot_players == 1);
		}
		LobbyType::Replay =>
		{
			debug_assert!(lobby.max_players == 0);
		}
	}
	debug_assert!(lobby.num_players <= lobby.max_players);
	debug_assert!(num_bot_players <= lobby.num_players);

	LobbyMetadata {
		max_players: lobby.max_players,
		num_players: lobby.num_players,
		num_bot_players,
		lobby_type: lobby.lobby_type,
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
	desired_metadata: Option<LobbyMetadata>,
	invite: Option<Invite>,
	clients: &mut Vec<Client>,
) -> Result<(), Error>
{
	let forced_role = match &invite
	{
		Some(Invite::JoinSecret(_)) => None,
		Some(Invite::SpectateSecret(_)) => Some(Role::Observer),
		None => None,
	};
	let mut handle_for_listing = client_handle.clone();

	let is_invited = if let Some(invite) = invite
	{
		if invite.secret().lobby_id != lobby.id
		{
			warn!("Client {} has invite for different lobby", client_id);
			return Ok(());
		}
		// Make sure the person that sent the invitation is still present.
		clients.iter().any(|x| x.handle.verify_invite(&invite))
	}
	else
	{
		false
	};

	if !lobby.is_public && !is_invited
	{
		return Ok(());
	}

	do_join(
		lobby,
		client_id,
		client_user_id,
		client_username.clone(),
		client_handle,
		lobby_sendbuffer,
		clients,
	);

	let update = chat::Update::JoinedLobby {
		client_id,
		lobby_id: lobby.id,
	};
	general_chat.send(update).await?;

	// If the newcomer was invited, their role might be forced to be observer.
	change_role(lobby, clients, client_id, forced_role)?;

	if let Some(metadata) = desired_metadata
	{
		if lobby.listing.is_some()
		{
			debug!("Ignoring desired metadata; lobby {} is listed", lobby.id);
		}
		else
		{
			become_desired_lobby(lobby, clients, metadata).await?;
		}
	}

	// Describe the lobby to the client so that Discord presence is updated.
	if lobby.listing.is_some()
	{
		let message = Message::ListLobby {
			lobby_id: lobby.id,
			lobby_name: lobby.name.clone(),
			metadata: make_description_metadata(lobby),
		};
		handle_for_listing.send(message);
	}

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
)
{
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
			invite: None,
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

	if let Some(host) = &lobby.host
	{
		// Tell the newcomer who is host.
		newcomer.handle.send(Message::ClaimHost {
			username: Some(host.username.clone()),
		});
	}

	if lobby.lobby_type != LobbyType::Replay
	{
		// Tell the newcomer the AI pool.
		for (name, metadata) in &lobby.ai_pool
		{
			newcomer.handle.send(Message::ListAi {
				ai_name: name.clone(),
				metadata: metadata.clone(),
			});
		}
	}

	for bot in &lobby.bots
	{
		newcomer.handle.send(Message::AddBot {
			slot: Some(bot.slot),
		});
		newcomer.handle.send(Message::ClaimAi {
			username_or_slot: UsernameOrSlot::Slot(bot.slot),
			ai_name: bot.ai_name.clone(),
		});
		newcomer.handle.send(Message::ClaimDifficulty {
			username_or_slot: UsernameOrSlot::Slot(bot.slot),
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

	if lobby.lobby_type != LobbyType::Replay
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
			metadata: Some(ListRulesetMetadata { lobby_id: lobby.id }),
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
		invite: None,
	};
	for client in clients.iter_mut()
	{
		client.handle.send(message.clone());
	}
	newcomer.handle.send(message);

	let update = client::Update::JoinedLobby {
		lobby_id: lobby.id,
		lobby: lobby_sendbuffer,
	};
	newcomer.handle.notify(update);

	clients.push(newcomer);
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
	let host_left = lobby
		.host
		.as_ref()
		.map(|x| x.id == client_id)
		.unwrap_or(false);
	let removed: Vec<Client> = if host_left
	{
		// When the host leaves, the lobby is disbanded.
		clients.drain(0..).collect()
	}
	else
	{
		clients
			.e_drain_where(|client| client.id == client_id)
			.collect()
	};

	handle_removed(lobby, clients, removed).await?;

	let update = chat::Update::LeftLobby {
		lobby_id: lobby.id,
		client_id,
	};
	general_chat.send(update).await?;

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

		// This is a stupid hack that is necessary because clients <1.0.8
		// do not handle LIST_AI messages sent after the lobby is created.
		for (name, metadata) in &lobby.ai_pool
		{
			handle.send(Message::ListAi {
				ai_name: name.clone(),
				metadata: metadata.clone(),
			});
		}
		for ai in &lobby.connected_ais
		{
			handle.send(Message::ListAi {
				ai_name: ai.ai_name.clone(),
				metadata: Some(BotAuthorsMetadata {
					authors: ai.authors.clone(),
				}),
			});
		}
	}

	Ok(())
}

fn handle_claim_host(
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
	username: String,
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

	if let Some(host) = &lobby.host
	{
		// Claim failed. Remind the claimant of the actual host.
		client.handle.send(Message::ClaimHost {
			username: Some(host.username.clone()),
		});
		return Ok(());
	}

	// Claim successful.
	lobby.host = Some(Host {
		id: client.id,
		username: username.clone(),
	});

	// Announce the new host.
	let message = Message::ClaimHost {
		username: Some(username),
	};
	for client in clients.iter_mut()
	{
		client.handle.send(message.clone());
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
	mut username_or_slot: UsernameOrSlot,
	color: PlayerColor,
)
{
	if let UsernameOrSlot::Empty(_empty) = username_or_slot
	{
		match lobby.bots.last()
		{
			Some(bot) =>
			{
				username_or_slot = UsernameOrSlot::Slot(bot.slot);
			}
			None =>
			{}
		}
	}

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
				let oldcolor = lobby.player_colors.insert(client_id, color);
				if let Some(oldcolor) = oldcolor
				{
					lobby.available_colors.push(oldcolor);
				}
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
				let oldcolor = lobby.bot_colors.insert(slot, color);
				if let Some(oldcolor) = oldcolor
				{
					lobby.available_colors.push(oldcolor);
				}
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
		&UsernameOrSlot::Empty(_empty) =>
		{
			warn!("Failed to find latest bot: there are no bots.");
			// FUTURE let the sender know somehow?
			return;
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
	mut username_or_slot: UsernameOrSlot,
	mut visiontype: VisionType,
)
{
	if lobby.lobby_type == LobbyType::OneVsOne
		&& visiontype != VisionType::Normal
	{
		warn!("Cannot change visiontype in onevsone lobby {}.", lobby.id);
		visiontype = VisionType::Normal;
	}

	if let UsernameOrSlot::Empty(_empty) = username_or_slot
	{
		match lobby.bots.last()
		{
			Some(bot) =>
			{
				username_or_slot = UsernameOrSlot::Slot(bot.slot);
			}
			None =>
			{}
		}
	}

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
		&UsernameOrSlot::Empty(_empty) =>
		{
			warn!("Failed to find latest bot: there are no bots.");
			// FUTURE let the sender know somehow?
			return;
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
	username_or_slot: UsernameOrSlot,
	ai_name: String,
)
{
	let slot: Option<Botslot> = match username_or_slot
	{
		UsernameOrSlot::Username(username) =>
		{
			warn!("Failed to find bot with username '{:?}'.", username);
			// FUTURE let the sender know somehow?
			return;
		}
		UsernameOrSlot::Slot(slot) => Some(slot),
		UsernameOrSlot::Empty(_empty) => match lobby.bots.last()
		{
			Some(bot) => Some(bot.slot),
			None => None,
		},
	};

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

	let connected = lobby.connected_ais.iter().find(|x| x.ai_name == ai_name);

	if connected.is_none() && !ai::exists(&ai_name)
	{
		warn!("Cannot set AI to non-existing '{}'.", ai_name);
		for client in clients.into_iter()
		{
			client.handle.send(Message::ClaimAi {
				username_or_slot: UsernameOrSlot::Slot(bot.slot),
				ai_name: bot.ai_name.clone(),
			});
		}
		return;
	}
	else if lobby
		.ai_name_blockers
		.iter()
		.any(|blocker| ai_name.to_lowercase().contains(blocker))
	{
		warn!("Cannot set AI to blocked '{}'.", ai_name);
		for client in clients.into_iter()
		{
			client.handle.send(Message::ClaimAi {
				username_or_slot: UsernameOrSlot::Slot(bot.slot),
				ai_name: bot.ai_name.clone(),
			});
		}
		return;
	}

	if lobby.ai_pool.iter().find(|&(x, _)| x == &ai_name).is_none()
	{
		let metadata = connected.map(|ai| BotAuthorsMetadata {
			authors: ai.authors.clone(),
		});

		lobby.ai_pool.push((ai_name.clone(), metadata.clone()));

		for client in clients.into_iter()
		{
			client.handle.send(Message::ListAi {
				ai_name: ai_name.clone(),
				metadata: metadata.clone(),
			});
		}
	}

	bot.ai_name = ai_name;

	for client in clients.into_iter()
	{
		client.handle.send(Message::ClaimAi {
			username_or_slot: UsernameOrSlot::Slot(bot.slot),
			ai_name: bot.ai_name.clone(),
		});
	}
}

fn change_difficulty(
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
	username_or_slot: UsernameOrSlot,
	difficulty: Difficulty,
)
{
	let slot: Option<Botslot> = match username_or_slot
	{
		UsernameOrSlot::Username(username) =>
		{
			warn!("Failed to find bot with username '{:?}'.", username);
			// FUTURE let the sender know somehow?
			return;
		}
		UsernameOrSlot::Slot(slot) => Some(slot),
		UsernameOrSlot::Empty(_empty) => match lobby.bots.last()
		{
			Some(bot) => Some(bot.slot),
			None => None,
		},
	};

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
		for client in clients.into_iter()
		{
			client.handle.send(Message::ClaimDifficulty {
				username_or_slot: UsernameOrSlot::Slot(bot.slot),
				difficulty,
			});
		}
		return;
	}

	bot.difficulty = difficulty;

	for client in clients.into_iter()
	{
		client.handle.send(Message::ClaimDifficulty {
			username_or_slot: UsernameOrSlot::Slot(bot.slot),
			difficulty,
		});
	}
}

fn add_bot(lobby: &mut Lobby, clients: &mut Vec<Client>)
{
	if lobby.lobby_type == LobbyType::OneVsOne
	{
		warn!("Cannot add bot in onevsone lobby {}.", lobby.id);
		// FUTURE let the sender know somehow?
		return;
	}

	if lobby.num_players >= lobby.max_players
	{
		warn!("Cannot add bot to lobby {}: lobby full", lobby.id);
		// FUTURE let the sender know somehow?
		return;
	}

	let slot = {
		if lobby.open_botslots.is_empty()
		{
			warn!("Cannot add bot to lobby {}: all slots taken", lobby.id);
			// FUTURE let the sender know somehow?
			return;
		}
		let mut rng = rand::thread_rng();
		let i = rng.gen_range(0, lobby.open_botslots.len());
		lobby.open_botslots.swap_remove(i)
	};

	let ai_name = match lobby.ai_pool.first()
	{
		Some((name, _metadata)) => name.clone(),
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
			username_or_slot: UsernameOrSlot::Slot(bot.slot),
			ai_name: bot.ai_name.clone(),
		});
		client.handle.send(Message::ClaimDifficulty {
			username_or_slot: UsernameOrSlot::Slot(bot.slot),
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
	if lobby.lobby_type == LobbyType::Replay
	{
		warn!("Cannot pick map for replay lobby {}.", lobby.id);
		return Ok(());
	}

	let found = lobby.map_pool.iter().find(|&(x, _)| *x == map_name);

	// The two player maps in the official map pool are compatible with
	// NeuralNewt (see below); note that unofficial maps can be added to
	// map_pool in the code below, but then a block occurred when that happened.
	let is_neural_newt_compatible = match found
	{
		Some((_, metadata)) =>
		{
			metadata.playercount == 2
				&& metadata.cols <= 20
				&& metadata.rows <= 13
		}
		None => map_name == "1v1",
	};

	let found = if found.is_some()
	{
		found
	}
	else if lobby.lobby_type == LobbyType::OneVsOne
	{
		warn!("Cannot pick unlisted map in onevsone lobby {}.", lobby.id);
		None
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

	// If this is a custom lobby, change rulesets based on the map.
	let custom_ruleset = if lobby.lobby_type == LobbyType::Custom
	{
		if metadata.pool_type == map::PoolType::Custom
		{
			metadata.ruleset_name.clone()
		}
		else
		{
			None
		}
	}
	else
	{
		None
	};

	// We might have a new playercount.
	let playercount = if metadata.playercount < 2
	{
		warn!("Map playercount cannot be less than 2.");
		2
	}
	else if metadata.playercount as usize > PLAYER_MAX
	{
		warn!("Map playercount cannot be more than PLAYER_MAX.");
		PLAYER_MAX as i32
	}
	else
	{
		metadata.playercount
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

	// The current version of NeuralNewt can only run on maps that are at most
	// 20 cols by 13 rows; other maps will probably cause the NN to crash.
	if !is_neural_newt_compatible
	{
		let blocker = "neuralnewt".to_string();
		block_ai(lobby, clients, blocker);
	}

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

	// If this is a custom lobby, change rulesets based on the map.
	if let Some(ruleset_name) = custom_ruleset
	{
		pick_ruleset(lobby, clients, ruleset_name).await?;
	}

	Ok(())
}

async fn become_desired_lobby(
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
	desired_metadata: LobbyMetadata,
) -> Result<(), Error>
{
	if lobby.lobby_type != LobbyType::Generic
	{
		warn!(
			"Cannot turn non-generic lobby {} into desired lobby.",
			lobby.id
		);
		return Ok(());
	}

	// Prevent lobbies from being drastically changed if there are
	// multiple human players present.
	if clients.len() > 1
	{
		warn!(
			"Cannot turn lobby {} with {} clients into desired lobby.",
			lobby.id,
			clients.len()
		);
		return Ok(());
	}

	match desired_metadata
	{
		LobbyMetadata {
			lobby_type: LobbyType::Generic,
			max_players,
			num_bot_players,
			..
		} if num_bot_players >= 0 && num_bot_players <= max_players =>
		{
			let max_players = max_players as usize;
			let num_bot_players = num_bot_players as usize;
			set_max_players_as_desired(lobby, clients, max_players).await?;
			add_or_remove_bots_as_desired(lobby, clients, num_bot_players)?;
		}
		LobbyMetadata {
			lobby_type: LobbyType::OneVsOne,
			max_players: 2,
			num_bot_players: 0,
			..
		} =>
		{
			lobby.lobby_type = LobbyType::OneVsOne;
			restrict_map_pool_for_one_vs_one(lobby).await?;
		}
		LobbyMetadata {
			lobby_type: LobbyType::Custom,
			max_players,
			num_bot_players,
			..
		} if num_bot_players >= 0 && num_bot_players <= max_players =>
		{
			let max_players = max_players as usize;
			let num_bot_players = num_bot_players as usize;
			become_custom_lobby(lobby, clients).await?;
			set_max_players_as_desired(lobby, clients, max_players).await?;
			add_or_remove_bots_as_desired(lobby, clients, num_bot_players)?;
		}
		LobbyMetadata {
			lobby_type: LobbyType::Tutorial,
			..
		} =>
		{
			become_tutorial_lobby(lobby, clients).await?;
		}
		LobbyMetadata {
			lobby_type: LobbyType::Challenge,
			..
		} =>
		{
			become_challenge_lobby(lobby, clients).await?;
		}
		// TODO LobbyType::Replay
		_ =>
		{
			warn!("Cannot turn lobby {} into desired lobby.", lobby.id);
			return Ok(());
		}
	}

	lobby.is_public = desired_metadata.is_public;

	Ok(())
}

async fn restrict_map_pool_for_one_vs_one(
	lobby: &mut Lobby,
) -> Result<(), Error>
{
	let filtered: Vec<(String, map::Metadata)> = lobby
		.map_pool
		.iter()
		.filter(|(name, _metadata)| name.contains("1v1"))
		.cloned()
		.collect();
	if filtered.is_empty()
	{
		error!(
			"Cannot turn lobby {} into a 1v1 lobby without 1v1 maps.",
			lobby.id
		);
		return Err(Error::EmptyMapPool);
	}
	lobby.map_pool = filtered;
	Ok(())
}

async fn set_max_players_as_desired(
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
	desired_max_players: usize,
) -> Result<(), Error>
{
	let suffix = match desired_max_players
	{
		2 => "1v1",
		3 => "3ffa",
		4 => "4ffa",
		5 => "5ffa",
		6 => "6ffa",
		7 => "7ffa",
		8 => "8ffa",
		_ =>
		{
			warn!(
				"Cannot turn lobby {} into a lobby for {} players.",
				lobby.id, desired_max_players
			);
			return Ok(());
		}
	};
	pick_map(lobby, clients, suffix.to_string()).await
}

fn add_or_remove_bots_as_desired(
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
	desired_num_bot_players: usize,
) -> Result<(), Error>
{
	let from = lobby.bots.len();
	for _ in from..desired_num_bot_players
	{
		add_bot(lobby, clients);
	}

	let to_be_removed: Vec<Botslot> = lobby.bots[desired_num_bot_players..]
		.iter()
		.map(|x| x.slot)
		.rev()
		.collect();
	for slot in to_be_removed
	{
		remove_bot(lobby, clients, slot);
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
	if lobby.lobby_type == LobbyType::Generic
	{
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

		lobby.lobby_type = LobbyType::Tutorial;
	}
	else if lobby.lobby_type == LobbyType::Tutorial
	{
		debug!("Cannot turn tutorial lobby {} into tutorial.", lobby.id);
		return Ok(());
	}
	else
	{
		warn!("Cannot turn non-generic lobby {} into tutorial.", lobby.id);
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
		let latest = UsernameOrSlot::Empty(botslot::EmptyBotslot);
		change_ai(lobby, clients, latest, ai_name.to_string());
		let latest = UsernameOrSlot::Empty(botslot::EmptyBotslot);
		change_difficulty(lobby, clients, latest, difficulty);
	}

	Ok(())
}

async fn become_challenge_lobby(
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
) -> Result<(), Error>
{
	// FUTURE check if client is host

	// Is this a game lobby?
	if lobby.lobby_type == LobbyType::Generic
	{
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

		lobby.lobby_type = LobbyType::Challenge;
	}
	else if lobby.lobby_type == LobbyType::Challenge
	{
		return Ok(());
	}
	else
	{
		warn!("Cannot turn non-generic lobby {} into challenge.", lobby.id);
		return Ok(());
	}

	let id: challenge::ChallengeId = {
		// TODO #1086 get from client request
		let pool = challenge::load_pool().unwrap();
		let challenge = pool.first().unwrap();
		challenge.id
	};
	let challenge_key = challenge::key(id);

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
		let latest = UsernameOrSlot::Empty(botslot::EmptyBotslot);
		change_ai(lobby, clients, latest, ai_name.clone());
		let latest = UsernameOrSlot::Empty(botslot::EmptyBotslot);
		change_difficulty(lobby, clients, latest, difficulty);
	}

	Ok(())
}

async fn become_custom_lobby(
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
) -> Result<(), Error>
{
	// FUTURE check if client is host

	// Is this a game lobby?
	if lobby.lobby_type == LobbyType::Generic
	{
		lobby.lobby_type = LobbyType::Custom;
	}
	else if lobby.lobby_type == LobbyType::Custom
	{
		debug!("Cannot turn custom lobby {} into custom.", lobby.id);
		return Ok(());
	}
	else
	{
		warn!("Cannot turn non-generic lobby {} into custom.", lobby.id);
		return Ok(());
	}

	let loaded_pool = map::load_custom_and_user_pool_with_metadata().await?;
	for (name, metadata) in loaded_pool
	{
		let message = Message::ListMap {
			map_name: name.clone(),
			metadata: metadata.clone(),
		};
		for client in clients.iter_mut()
		{
			client.handle.send(message.clone());
		}

		lobby.map_pool.push((name, metadata));
	}

	Ok(())
}

async fn pick_timer(
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
	mut timer_in_seconds: u32,
) -> Result<(), Error>
{
	// FUTURE check if client is host

	// Is this a game lobby?
	if lobby.lobby_type == LobbyType::Replay
	{
		warn!("Cannot pick map for replay lobby {}.", lobby.id);
		// FUTURE let the sender know somehow?
		return Ok(());
	}
	else if lobby.lobby_type == LobbyType::OneVsOne
	{
		warn!("Ignoring a change in lobby {} because OneVsOne.", lobby.id);
		timer_in_seconds = lobby.timer_in_seconds;
	}

	// Not much need for extreme validation. Current hard cap 5 minutes.
	if timer_in_seconds > 300
	{
		warn!("Capping excessive timer value in lobby {}.", lobby.id);
		timer_in_seconds = 300;
	}

	lobby.timer_in_seconds = timer_in_seconds;

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
	if lobby.lobby_type == LobbyType::Replay
	{
		warn!("Cannot pick ruleset in replay lobby {}.", lobby.id);
		return Ok(());
	}
	else if lobby.lobby_type == LobbyType::OneVsOne
	{
		warn!("Cannot pick ruleset in onevsone lobby {}.", lobby.id);
		let message = Message::PickRuleset {
			ruleset_name: lobby.ruleset_name.clone(),
		};
		for client in clients.iter_mut()
		{
			client.handle.send(message.clone());
		}
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
		metadata: Some(ListRulesetMetadata { lobby_id: lobby.id }),
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

fn block_ai(lobby: &mut Lobby, clients: &mut Vec<Client>, blocker: String)
{
	if !lobby.ai_name_blockers.contains(&blocker)
	{
		lobby.ai_pool.retain(|(ainame, _metadata)| {
			!ainame.to_lowercase().contains(&blocker)
		});

		lobby
			.connected_ais
			.retain(|x| !x.ai_name.to_lowercase().contains(&blocker));

		let to_be_changed: Vec<Botslot> = lobby
			.bots
			.iter()
			.filter(|bot| bot.ai_name.to_lowercase().contains(&blocker))
			.map(|bot| bot.slot)
			.collect();
		for slot in to_be_changed
		{
			let replacement = "RampantRhino".to_string();
			let username_or_slot = UsernameOrSlot::Slot(slot);
			change_ai(lobby, clients, username_or_slot, replacement);
		}

		lobby.ai_name_blockers.push(blocker);
	}
}

async fn add_ai_to_list(
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
	ai: ConnectedAi,
)
{
	if lobby.ai_name_blockers.contains(&ai.ai_name)
	{
		return;
	}
	else if lobby
		.ai_pool
		.iter()
		.any(|(ainame, _metadata)| ainame == &ai.ai_name)
	{
		return;
	}

	for client in clients.into_iter()
	{
		client.handle.send(Message::ListAi {
			ai_name: ai.ai_name.clone(),
			metadata: Some(BotAuthorsMetadata {
				authors: ai.authors.clone(),
			}),
		});
	}

	lobby.connected_ais.push(ai);
}

async fn handle_ruleset_confirmation(
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
	client_id: Keycode,
	ruleset_name: String,
	general_chat: &mut mpsc::Sender<chat::Update>,
	lobby_sendbuffer: mpsc::Sender<Update>,
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
		try_start(lobby, clients, general_chat, lobby_sendbuffer).await
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
	for ai in &lobby.staged_connected_ais
	{
		if !lobby.ruleset_confirmations.contains(&ai.client_id)
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
	lobby_sendbuffer: mpsc::Sender<Update>,
) -> Result<Option<game::Setup>, Error>
{
	// FUTURE check if host

	if let Some(host) = &lobby.host
	{
		// Make sure the host is present when the game starts.
		if !clients.iter().any(|x| x.id == host.id)
		{
			debug!("Cannot start lobby {}: host missing.", lobby.id);
			return Ok(None);
		}
	}

	// Add connected bots if necessary.
	let to_be_added: Vec<ConnectedAi> = lobby
		.connected_ais
		.iter()
		.filter(|ai| {
			lobby.bots.iter().any(|bot| bot.ai_name == ai.ai_name)
				&& !lobby
					.staged_connected_ais
					.iter()
					.any(|x| x.client_id == ai.client_id)
		})
		.map(|ai| ai.clone())
		.collect();
	for mut connected_ai in to_be_added
	{
		let update = client::Update::JoinedLobby {
			lobby_id: lobby.id,
			lobby: lobby_sendbuffer.clone(),
		};
		connected_ai.handle.notify(update);
		lobby.staged_connected_ais.push(connected_ai);
	}

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

	// Check that all clients have access to the ruleset that we will use.
	if !is_ruleset_confirmed(lobby, clients)
	{
		debug!("Delaying start in lobby {}: ruleset unconfirmed.", lobby.id);

		// List the new ruleset to trigger additional confirmations.
		for client in clients.iter_mut()
		{
			if !lobby.ruleset_confirmations.contains(&client.id)
			{
				let message = Message::ListRuleset {
					ruleset_name: lobby.ruleset_name.clone(),
					metadata: None,
				};
				client.handle.send(message);
			}
		}

		for ai in lobby.staged_connected_ais.iter_mut()
		{
			if !lobby.ruleset_confirmations.contains(&ai.client_id)
			{
				let message = Message::ListRuleset {
					ruleset_name: lobby.ruleset_name.clone(),
					metadata: Some(ListRulesetMetadata { lobby_id: lobby.id }),
				};
				ai.handle.send(message);
			}
		}

		// Cannot continue until all player have confirmed the ruleset.
		lobby.stage = Stage::WaitingForConfirmation;
		return Ok(None);
	}

	// We are truly starting.
	let game = start(lobby, clients, general_chat).await?;
	Ok(Some(game))
}

async fn start(
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
	general_chat: &mut mpsc::Sender<chat::Update>,
) -> Result<game::Setup, Error>
{
	// If this is a client-hosted game, prepare the host client.
	let mut host_client = if let Some(host) = &lobby.host
	{
		if lobby.lobby_type != LobbyType::Custom
		{
			None
		}
		else if let Some(client) = clients.iter().find(|x| x.id == host.id)
		{
			Some(game::HostClient {
				id: client.id,
				user_id: client.user_id,
				username: client.username.clone(),
				handle: client.handle.clone(),

				is_gameover: false,
			})
		}
		else
		{
			return Err(Error::StartGameHostMissing);
		}
	}
	else
	{
		None
	};

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

				// If there is a host, tell them who is playing as which color.
				// We also send this information to players, but then using descriptive
				// names instead of botslots, so we need to send it here as well.
				if let Some(host) = &mut host_client
				{
					host.handle.send(Message::ClaimColor {
						color,
						username_or_slot: UsernameOrSlot::Username(
							client.username.clone(),
						),
					});
				}

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
	let mut connected_bots = Vec::new();
	let mut local_bots = Vec::new();
	let mut hosted_bots = Vec::new();
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

		// If there is a host, tell them who is playing as which color.
		// We also send this information to players, but then using descriptive
		// names instead of botslots, so we need to send it here as well.
		if let Some(host) = &mut host_client
		{
			host.handle.send(Message::ClaimColor {
				username_or_slot: UsernameOrSlot::Slot(bot.slot),
				color,
			});
		}

		let vision = match lobby.bot_visiontypes.get(&bot.slot)
		{
			Some(&vision) => vision,
			None => VisionType::Normal,
		};

		let character = bot.slot.get_character();

		if host_client.is_some()
		{
			let difficulty_str = match bot.difficulty
			{
				Difficulty::None => "Easy",
				Difficulty::Easy => "Easy",
				Difficulty::Medium => "Medium",
				Difficulty::Hard => "Hard",
			};
			let display_name = bot.slot.get_display_name();
			let descriptive_name = format!(
				"{} ({} {})",
				display_name, difficulty_str, bot.ai_name
			);
			hosted_bots.push(game::HostedBot {
				descriptive_name,
				color,
			});
		}
		else if let Some(connected_ai) = lobby
			.connected_ais
			.iter()
			.find(|ai| ai.ai_name == bot.ai_name)
		{
			let difficulty_str = match bot.difficulty
			{
				Difficulty::None => "Easy",
				Difficulty::Easy => "Easy",
				Difficulty::Medium => "Medium",
				Difficulty::Hard => "Hard",
			};
			let display_name = bot.slot.get_display_name();
			let descriptive_name = format!(
				"{} ({} {})",
				display_name, difficulty_str, bot.ai_name
			);
			let ai_metadata_json = json!({
				"difficulty": bot.difficulty,
				"character": character.to_string(),
				"displayname": display_name,
				"ainame": bot.ai_name,
				"authors": connected_ai.authors,
				"connected_user_id": connected_ai.client_user_id,
				"connected_username": connected_ai.client_username,
			});
			let ai_metadata: ai::Metadata =
				match serde_json::from_value(ai_metadata_json)
				{
					Ok(metadata) => metadata,
					Err(error) => return Err(Error::AiMetadataParsing(error)),
				};
			let forwarding_metadata = ForwardingMetadata::ConnectedBot {
				lobby_id: lobby.id,
				slot: bot.slot,
			};

			connected_bots.push(game::BotClient {
				slot: bot.slot,
				difficulty: bot.difficulty,
				descriptive_name,
				ai_metadata,
				forwarding_metadata,

				id: connected_ai.client_id,
				user_id: connected_ai.client_user_id,
				handle: connected_ai.handle.clone(),

				color,
				vision,

				is_defeated: false,
				submitted_orders: None,
			});
		}
		else
		{
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

			local_bots.push(game::LocalBot {
				slot: bot.slot,
				ai,

				color,
				vision,

				is_defeated: false,
			});
		}
	}

	// TODO if (_replay && _replayname.empty()) return error;

	let map_name = lobby.map_name.clone();
	let map_metadata = {
		match lobby.map_pool.iter().find(|(name, _)| name == &map_name)
		{
			Some((_, metadata)) => metadata.clone(),
			None => map::load_metadata(&map_name).await?,
		}
	};

	let planning_timer = Some(lobby.timer_in_seconds).filter(|&x| x > 0);

	let game = game::Setup {
		lobby_id: lobby.id,
		lobby_name: lobby.name.clone(),
		lobby_description_metadata: make_description_metadata(lobby),
		host: host_client,
		players: player_clients,
		connected_bots,
		local_bots,
		hosted_bots,
		watchers: watcher_clients,
		map_name,
		map_metadata,
		ruleset_name: lobby.ruleset_name.clone(),
		planning_time_in_seconds: planning_timer,
		lobby_type: lobby.lobby_type,
		challenge: lobby.challenge_id,
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
				warn!("Missing role for InGame message");
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

	Ok(game)
}

#[derive(Debug)]
enum Error
{
	EmptyMapPool,
	ClientMissing,
	StartGameHostMissing,
	StartGameNotEnoughColors,
	Io
	{
		error: io::Error,
	},
	GeneralChat
	{
		error: mpsc::error::SendError<chat::Update>,
	},
	NameSend
	{
		error: watch::error::SendError<String>,
	},
	MetadataSend
	{
		error: watch::error::SendError<LobbyMetadata>,
	},
	AiAllocationError
	{
		error: ai::InterfaceError,
	},
	AiMetadataParsing(serde_json::error::Error),
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

impl From<watch::error::SendError<String>> for Error
{
	fn from(error: watch::error::SendError<String>) -> Self
	{
		Error::NameSend { error }
	}
}

impl From<watch::error::SendError<LobbyMetadata>> for Error
{
	fn from(error: watch::error::SendError<LobbyMetadata>) -> Self
	{
		Error::MetadataSend { error }
	}
}

impl fmt::Display for Error
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
	{
		match self
		{
			Error::EmptyMapPool => write!(f, "{:#?}", self),
			Error::ClientMissing => write!(f, "{:#?}", self),
			Error::StartGameHostMissing => write!(f, "{:#?}", self),
			Error::StartGameNotEnoughColors => write!(f, "{:#?}", self),
			Error::Io { error } => error.fmt(f),
			Error::GeneralChat { error } => error.fmt(f),
			Error::NameSend { error } => error.fmt(f),
			Error::MetadataSend { error } => error.fmt(f),
			Error::AiAllocationError { error } =>
			{
				write!(f, "Error while allocating AI: {}", error)
			}
			Error::AiMetadataParsing(error) => error.fmt(f),
		}
	}
}

impl std::error::Error for Error {}
