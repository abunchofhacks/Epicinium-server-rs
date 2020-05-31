/* Server::Lobby */

use crate::common::keycode::*;
use crate::logic::ai;
use crate::logic::challenge;
use crate::logic::difficulty::*;
use crate::logic::map;
use crate::logic::player::PlayerColor;
use crate::server::botslot;
use crate::server::botslot::Botslot;
use crate::server::chat;
use crate::server::client;
use crate::server::game;
use crate::server::message::*;

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io;
use std::sync;
use std::sync::atomic;

use rand::seq::SliceRandom;
use rand::Rng;

use tokio::sync::mpsc;

use vec_drain_where::VecDrainWhereExt;

#[derive(Debug)]
pub enum Update
{
	Save
	{
		lobby_sendbuffer: mpsc::Sender<Update>,
		general_chat: mpsc::Sender<chat::Update>,
	},

	Join
	{
		client_id: Keycode,
		client_username: String,
		client_sendbuffer: mpsc::Sender<Message>,
		client_callback: mpsc::Sender<client::Update>,
		lobby_sendbuffer: mpsc::Sender<Update>,
		general_chat: mpsc::Sender<chat::Update>,
	},
	Leave
	{
		client_id: Keycode,
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

	GameEnded,

	ForwardToGame(game::Update),

	Msg(Message),
}

pub fn create(ticker: &mut sync::Arc<atomic::AtomicU64>)
	-> mpsc::Sender<Update>
{
	let key = rand::random();
	let data = ticker.fetch_add(1, atomic::Ordering::Relaxed);
	let lobby_id = keycode(key, data);

	let (updates_in, updates_out) = mpsc::channel::<Update>(1000);

	let task = run(lobby_id, updates_out);
	tokio::spawn(task);

	updates_in
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Stage
{
	Setup,
	WaitingForConfirmation,
	GameStarted,
	GameEnded,
}

impl Stage
{
	pub fn has_game_started(&self) -> bool
	{
		match self
		{
			Stage::Setup => false,
			Stage::WaitingForConfirmation => false,
			Stage::GameStarted => true,
			Stage::GameEnded => true,
		}
	}
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
	game_in_progress: Option<mpsc::Sender<game::Update>>,
}

async fn run(lobby_id: Keycode, mut updates: mpsc::Receiver<Update>)
{
	let mut lobby = match initialize(lobby_id).await
	{
		Ok(lobby) => lobby,
		Err(error) =>
		{
			eprintln!("Failed to create lobby: {:?}", error);
			return;
		}
	};

	let mut clients: Vec<Client> = Vec::new();

	while let Some(update) = updates.recv().await
	{
		match handle_update(update, &mut lobby, &mut clients).await
		{
			Ok(()) => continue,
			Err(error) =>
			{
				eprintln!("Lobby {} crashed: {:?}", lobby_id, error);
				break;
			}
		}
	}

	println!("Lobby {} has disbanded.", lobby_id);
}

async fn initialize(lobby_id: Keycode) -> Result<Lobby, Error>
{
	let map_pool = map::load_pool_with_metadata().await?;

	let defaultmap = map_pool.get(0).ok_or(Error::EmptyMapPool)?;
	let (name, _) = defaultmap;
	let map_name = name.to_string();

	// TODO Library::nameCurrentBible()
	let ruleset_name = "v0.33.0".to_string();

	Ok(Lobby {
		id: lobby_id,
		name: initial_name(),
		num_players: 0,
		max_players: 2,
		is_public: true,
		is_replay: false,
		has_been_listed: false,
		last_description_metadata: None,
		bots: Vec::new(),
		open_botslots: botslot::pool(),
		roles: HashMap::new(),
		ai_pool: ai::load_pool(),
		map_pool,
		map_name,
		ruleset_name,
		ruleset_confirmations: HashSet::new(),
		timer_in_seconds: 60,
		challenge_id: None,
		is_tutorial: false,
		is_rated: true,
		stage: Stage::Setup,
		game_in_progress: None,
	})
}

async fn handle_update(
	update: Update,
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
) -> Result<(), Error>
{
	match update
	{
		Update::Save {
			lobby_sendbuffer,
			mut general_chat,
		} => list_lobby(lobby, lobby_sendbuffer, &mut general_chat).await,

		Update::Join {
			client_id,
			client_username,
			client_sendbuffer,
			client_callback,
			lobby_sendbuffer,
			mut general_chat,
		} =>
		{
			handle_join(
				lobby,
				client_id,
				client_username,
				client_sendbuffer,
				client_callback,
				lobby_sendbuffer,
				&mut general_chat,
				clients,
			)
			.await
		}
		Update::Leave {
			client_id,
			mut general_chat,
		} => handle_leave(lobby, client_id, clients, &mut general_chat).await,

		Update::Lock { mut general_chat } =>
		{
			lobby.is_public = false;
			describe_lobby(lobby, &mut general_chat).await
		}
		Update::Unlock { mut general_chat } =>
		{
			lobby.is_public = true;
			describe_lobby(lobby, &mut general_chat).await
		}

		Update::Rename {
			lobby_name,
			mut general_chat,
		} =>
		{
			lobby.name = lobby_name;
			// Unset the description metadata to force a lobby description.
			lobby.last_description_metadata = None;
			describe_lobby(lobby, &mut general_chat).await
		}

		Update::ClaimRole {
			mut general_chat,
			username,
			role,
		} =>
		{
			handle_claim_role(lobby, clients, username, role)?;
			describe_lobby(lobby, &mut general_chat).await
		}
		Update::ClaimAi { slot, ai_name } =>
		{
			change_ai(lobby, clients, slot, ai_name);
			Ok(())
		}
		Update::ClaimDifficulty { slot, difficulty } =>
		{
			change_difficulty(lobby, clients, slot, difficulty);
			Ok(())
		}

		Update::AddBot { mut general_chat } =>
		{
			add_bot(lobby, clients);
			describe_lobby(lobby, &mut general_chat).await
		}
		Update::RemoveBot {
			mut general_chat,
			slot,
		} =>
		{
			remove_bot(lobby, clients, slot);
			describe_lobby(lobby, &mut general_chat).await
		}

		Update::PickMap {
			mut general_chat,
			map_name,
		} =>
		{
			pick_map(lobby, clients, map_name).await?;
			describe_lobby(lobby, &mut general_chat).await
		}
		Update::PickTutorial { mut general_chat } =>
		{
			become_tutorial_lobby(lobby, clients).await?;
			describe_lobby(lobby, &mut general_chat).await
		}
		Update::PickChallenge { mut general_chat } =>
		{
			become_challenge_lobby(lobby, clients).await?;
			describe_lobby(lobby, &mut general_chat).await
		}
		Update::PickTimer { seconds } =>
		{
			pick_timer(lobby, clients, seconds).await
		}
		Update::PickRuleset { ruleset_name } =>
		{
			pick_ruleset(lobby, clients, ruleset_name).await
		}
		Update::ConfirmRuleset {
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

		Update::Start {
			mut general_chat,
			lobby_sendbuffer: sendbuffer,
		} =>
		{
			try_start(lobby, clients, &mut general_chat, sendbuffer).await?;
			describe_lobby(lobby, &mut general_chat).await
		}

		Update::GameEnded if lobby.stage == Stage::GameStarted =>
		{
			// TODO announce to general chat somehow
			lobby.stage = Stage::GameEnded;
			Ok(())
		}
		Update::GameEnded => Err(Error::GameEndedWithoutStarting),

		Update::ForwardToGame(update) =>
		{
			match &mut lobby.game_in_progress
			{
				Some(game) =>
				{
					game.send(update).await?;
					Ok(())
				}
				None =>
				{
					eprintln!("Discarding game update {:?} after game ended in lobby {}", update, lobby.id);
					Ok(())
				}
			}
		}

		Update::Msg(message) =>
		{
			for client in clients.iter_mut()
			{
				client.send(message.clone());
			}
			Ok(())
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
		eprintln!("Refusing to re-list lobby {}.", lobby.id);
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
	username: String,
	sendbuffer: mpsc::Sender<Message>,
	is_dead: bool,
}

impl Client
{
	fn send(&mut self, message: Message)
	{
		match self.sendbuffer.try_send(message)
		{
			Ok(()) => (),
			Err(_error) => self.is_dead = true,
		}
	}
}

async fn handle_join(
	lobby: &mut Lobby,
	client_id: Keycode,
	client_username: String,
	client_sendbuffer: mpsc::Sender<Message>,
	client_callback: mpsc::Sender<client::Update>,
	lobby_sendbuffer: mpsc::Sender<Update>,
	general_chat: &mut mpsc::Sender<chat::Update>,
	clients: &mut Vec<Client>,
) -> Result<(), Error>
{
	let mut sendbuffer_for_listing = client_sendbuffer.clone();

	match do_join(
		lobby,
		client_id,
		client_username.clone(),
		client_sendbuffer,
		client_callback,
		lobby_sendbuffer,
		clients,
	)
	{
		Ok(()) => (),
		Err(()) => return Ok(()),
	}

	let message = Message::JoinLobby {
		lobby_id: Some(lobby.id),
		username: Some(client_username),
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
	match sendbuffer_for_listing.try_send(message)
	{
		Ok(()) => (),
		Err(error) => eprintln!("Send error in join: {}", error),
	}

	if Some(&Role::Player) == lobby.roles.get(&client_id)
	{
		describe_lobby(lobby, general_chat).await?;
	}

	// If a game is already in progress, rejoin it.
	// TODO rejoin

	Ok(())
}

fn do_join(
	lobby: &mut Lobby,
	client_id: Keycode,
	client_username: String,
	client_sendbuffer: mpsc::Sender<Message>,
	mut client_callback: mpsc::Sender<client::Update>,
	lobby_sendbuffer: mpsc::Sender<Update>,
	clients: &mut Vec<Client>,
) -> Result<(), ()>
{
	// TODO joining might fail because it is full or locked etcetera

	let mut newcomer = Client {
		id: client_id,
		username: client_username,
		sendbuffer: client_sendbuffer,
		is_dead: false,
	};

	// Tell the newcomer which users are already in the lobby.
	for other in clients.into_iter()
	{
		newcomer.send(Message::JoinLobby {
			lobby_id: Some(lobby.id),
			username: Some(other.username.clone()),
			metadata: None,
		});

		if let Some(&role) = lobby.roles.get(&other.id)
		{
			newcomer.send(Message::ClaimRole {
				username: other.username.clone(),
				role,
			});
		}
		// TODO colors
		// TODO vision types
	}

	if !lobby.is_replay
	{
		// Tell the newcomer the AI pool.
		for name in &lobby.ai_pool
		{
			newcomer.send(Message::ListAi {
				ai_name: name.clone(),
			});
		}
	}

	for bot in &lobby.bots
	{
		newcomer.send(Message::AddBot {
			slot: Some(bot.slot),
		});
		newcomer.send(Message::ClaimAi {
			slot: Some(bot.slot),
			ai_name: bot.ai_name.clone(),
		});
		newcomer.send(Message::ClaimDifficulty {
			slot: Some(bot.slot),
			difficulty: bot.difficulty,
		});
		// TODO colors
		// TODO vision types
	}

	if !lobby.is_replay
	{
		for (mapname, metadata) in &lobby.map_pool
		{
			newcomer.send(Message::ListMap {
				map_name: mapname.clone(),
				metadata: metadata.clone(),
			});
		}

		newcomer.send(Message::PickMap {
			map_name: lobby.map_name.clone(),
		});
		newcomer.send(Message::PickTimer {
			seconds: lobby.timer_in_seconds,
		});

		newcomer.send(Message::ListRuleset {
			ruleset_name: lobby.ruleset_name.clone(),
		});
		newcomer.send(Message::PickRuleset {
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
		client.send(message.clone());
	}
	newcomer.send(message);

	client_callback
		.try_send(client::Update::JoinedLobby {
			lobby: lobby_sendbuffer,
		})
		.unwrap_or_else(|e| eprintln!("Callback error in join: {:?}", e));

	clients.push(newcomer);

	Ok(())
}

async fn handle_leave(
	lobby: &mut Lobby,
	client_id: Keycode,
	clients: &mut Vec<Client>,
	general_chat: &mut mpsc::Sender<chat::Update>,
) -> Result<(), Error>
{
	do_leave(lobby, client_id, clients);

	// TODO dont disband if rejoinable etcetera
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

fn do_leave(lobby: &mut Lobby, client_id: Keycode, clients: &mut Vec<Client>)
{
	let removed: Vec<Client> = clients
		.e_drain_where(|client| client.id == client_id)
		.collect();

	handle_removed(lobby, clients, removed)
}

fn handle_removed(
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
	removed: Vec<Client>,
)
{
	for removed_client in removed
	{
		let Client {
			id,
			username,
			mut sendbuffer,
			is_dead,
		} = removed_client;

		let message = Message::LeaveLobby {
			lobby_id: Some(lobby.id),
			username: Some(username),
		};

		for client in clients.iter_mut()
		{
			client.send(message.clone());
		}

		if !is_dead
		{
			match sendbuffer.try_send(message)
			{
				Ok(()) => (),
				Err(e) =>
				{
					eprintln!("Send error while processing leave: {:?}", e)
				}
			}
		}

		let removed_role = lobby.roles.remove(&id);
		// TODO colors
		// TODO visiontypes

		if removed_role == Some(Role::Player)
		{
			lobby.num_players -= 1;
		}
	}
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
			// TODO let the sender know somehow?
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

	// If a game is already in progress, the user might be a disconnected user.
	let inprogress = false;
	// TODO determine role if was disconnected

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
						client.send(message.clone());
					}
				}
				return Ok(());
			}
		}
		Some(Role::Observer) => Role::Observer,
		None =>
		{
			if !inprogress && lobby.num_players < lobby.max_players
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
		client.send(message.clone());
	}

	if assigned_role == Role::Player
	{
		lobby.num_players += 1;

		// If we remembered the player's vision type, they keep it.
		// TODO vision types
		{}
	}
	else
	{
		// The player loses their player color when they stop being a player.
		// TODO colors

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
				eprintln!("Failed to find bot '{:?}'.", slot);
				// TODO let the sender know somehow?
				return;
			}
		}
	};

	if !ai::exists(&ai_name)
	{
		eprintln!("Cannot set AI to non-existing '{}'.", ai_name);
		// TODO let the sender know somehow?
		return;
	}

	if lobby.ai_pool.iter().find(|&x| x == &ai_name).is_none()
	{
		lobby.ai_pool.push(ai_name.clone());

		for client in clients.into_iter()
		{
			client.send(Message::ListAi {
				ai_name: ai_name.clone(),
			});
		}
	}

	bot.ai_name = ai_name;

	for client in clients.into_iter()
	{
		client.send(Message::ClaimAi {
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
				eprintln!("Failed to find bot '{:?}'.", slot);
				// TODO let the sender know somehow?
				return;
			}
		}
	};

	if difficulty == Difficulty::None && bot.ai_name != "Dummy"
	{
		eprintln!("Cannot send difficulty of AI '{}' to none.", bot.ai_name);
		// TODO let the sender know somehow?
		return;
	}

	bot.difficulty = difficulty;

	for client in clients.into_iter()
	{
		client.send(Message::ClaimDifficulty {
			slot: Some(bot.slot),
			difficulty,
		});
	}
}

fn add_bot(lobby: &mut Lobby, clients: &mut Vec<Client>)
{
	if lobby.num_players >= lobby.max_players
	{
		eprintln!("Cannot add bot to lobby {}: lobby full", lobby.id);
		return;
	}

	let slot = {
		if lobby.open_botslots.is_empty()
		{
			eprintln!("Cannot add bot to lobby {}: all slots taken", lobby.id);
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
		client.send(Message::AddBot {
			slot: Some(bot.slot),
		});
		client.send(Message::ClaimAi {
			slot: Some(bot.slot),
			ai_name: bot.ai_name.clone(),
		});
		client.send(Message::ClaimDifficulty {
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
		// TODO colors
		lobby.open_botslots.push(slot);

		for client in clients.into_iter()
		{
			client.send(Message::RemoveBot { slot });
		}
	}
}

async fn pick_map(
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
	map_name: String,
) -> Result<(), Error>
{
	// TODO check if client is host

	// Is this a game lobby?
	if lobby.is_replay
	{
		eprintln!("Cannot pick map for replay lobby {}.", lobby.id);
		return Ok(());
	}

	let found = lobby.map_pool.iter().find(|&(x, _)| *x == map_name);

	let found = if found.is_some()
	{
		found
	}
	// TODO check if map in hidden pool or client is developer
	else if map::exists(&map_name)
	{
		let metadata = map::load_metadata(&map_name).await?;

		let message = Message::ListMap {
			map_name: map_name.clone(),
			metadata: metadata.clone(),
		};
		for client in clients.iter_mut()
		{
			client.send(message.clone());
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
			let message = Message::PickMap {
				map_name: lobby.map_name.clone(),
			};
			for client in clients.iter_mut()
			{
				client.send(message.clone());
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
		client.send(message.clone());
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
	// TODO check if client is host

	// Is this a game lobby?
	if lobby.is_replay
	{
		eprintln!("Cannot turn replay lobby {} into tutorial.", lobby.id);
		return Ok(());
	}

	// Prevent lobbies from being turned to tutorial lobbies if there are
	// multiple human players present.
	if clients.len() > 1
	{
		eprintln!(
			"Cannot turn lobby {} with {} clients into tutorial.",
			lobby.id,
			clients.len()
		);
		return Ok(());
	}

	pick_map(lobby, clients, "tutorial".to_string()).await?;
	pick_timer(lobby, clients, 0).await?;

	// TODO global vision

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
	// TODO check if client is host

	// Is this a game lobby?
	if lobby.is_replay
	{
		eprintln!("Cannot turn replay lobby {} into challenge.", lobby.id);
		return Ok(());
	}

	// Prevent lobbies from being turned to challenge lobbies if there are
	// multiple human players present.
	if clients.len() > 1
	{
		eprintln!(
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
		client.send(Message::PickChallenge {
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
	// TODO check if client is host

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
		client.send(message.clone());
	}

	Ok(())
}

async fn pick_ruleset(
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
	ruleset_name: String,
) -> Result<(), Error>
{
	// TODO check if client is host
	// TODO check if ruleset in pool or client is developer or replay

	// Is this a game lobby?
	if lobby.is_replay
	{
		eprintln!("Cannot pick ruleset in replay lobby {}.", lobby.id);
		return Ok(());
	}

	// Maybe give an error here? For now, this is used because AIChallenge
	// might want to use the default ruleset.
	if ruleset_name.is_empty()
	{
		// TODO Library::nameCurrentBible()
		lobby.ruleset_name = "v0.33.0".to_string();
	}
	// TODO Library::existsBible(ruleset_name)
	else
	{
		lobby.ruleset_name = ruleset_name;
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
		client.send(listmessage.clone());
		client.send(pickmessage.clone());
	}

	Ok(())
}

async fn handle_ruleset_confirmation(
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
	client_id: Keycode,
	ruleset_name: String,
	general_chat: &mut mpsc::Sender<chat::Update>,
	lobby_sendbuffer: mpsc::Sender<Update>,
) -> Result<(), Error>
{
	if ruleset_name != lobby.ruleset_name
	{
		println!(
			"Ignoring confirmation for ruleset '{}' \
			 when current ruleset is '{}'.",
			ruleset_name, lobby.ruleset_name
		);
		return Ok(());
	}

	if lobby.ruleset_confirmations.contains(&client_id)
	{
		return Ok(());
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
		Ok(())
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
	lobby_sendbuffer: mpsc::Sender<Update>,
) -> Result<(), Error>
{
	// We cannot start a game if it is already in progress.
	if lobby.stage.has_game_started()
	{
		return Ok(());
	}

	// Make sure all the clients are still valid.
	let removed = clients.e_drain_where(|client| client.is_dead).collect();
	handle_removed(lobby, clients, removed);

	if clients.len() < 1
	{
		eprintln!("Cannot start lobby {} without clients.", lobby.id);
		return Ok(());
	}

	if lobby.num_players < lobby.max_players
	{
		println!("Cannot start lobby {}: not enough players.", lobby.id);
		return Ok(());
	}

	// TODO replace with lobby.open_colors
	let mut open_colors = vec![
		PlayerColor::Red,
		PlayerColor::Blue,
		PlayerColor::Yellow,
		PlayerColor::Teal,
		PlayerColor::Black,
		PlayerColor::Pink,
		PlayerColor::Indigo,
		PlayerColor::Purple,
	];

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
				let color = {
					// TODO color claims
					let assigned = open_colors.pop();
					debug_assert!(assigned.is_some());
					match assigned
					{
						Some(color) => color,
						None =>
						{
							return Err(Error::StartGameNotEnoughColors);
						}
					}
				};

				// TODO visiontypes
				let vision = VisionType::Normal;

				player_clients.push(game::PlayerClient {
					id: client.id,
					username: client.username.clone(),
					sendbuffer: Some(client.sendbuffer.clone()),

					color,
					vision,

					is_defeated: false,
					is_retired: false,
					has_synced: false,
					received_orders: None,
				});
			}
			Role::Observer =>
			{
				watcher_clients.push(game::WatcherClient {
					id: client.id,
					username: client.username.clone(),
					sendbuffer: Some(client.sendbuffer.clone()),

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
		let color = {
			// TODO color claims
			let assigned = open_colors.pop();
			debug_assert!(assigned.is_some());
			match assigned
			{
				Some(color) => color,
				None =>
				{
					return Err(Error::StartGameNotEnoughColors);
				}
			}
		};

		// TODO visiontypes
		let vision = VisionType::Normal;

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
		println!("Delaying start in lobby {}: ruleset unconfirmed.", lobby.id);

		// List the new ruleset to trigger additional confirmations.
		let message = Message::ListRuleset {
			ruleset_name: lobby.ruleset_name.clone(),
		};
		for client in clients.iter_mut()
		{
			client.send(message.clone());
		}

		// Cannot continue until all player have confirmed the ruleset.
		lobby.stage = Stage::WaitingForConfirmation;
		return Ok(());
	}

	let planning_timer = Some(lobby.timer_in_seconds).filter(|&x| x > 0);

	// We are truly starting.
	let (updates_in, updates_out) = mpsc::channel::<game::Update>(1000);
	let task = game::start(
		lobby.id,
		lobby_sendbuffer,
		updates_out,
		player_clients,
		bots,
		watcher_clients,
		lobby.map_name.clone(),
		lobby.ruleset_name.clone(),
		planning_timer,
		lobby.challenge_id,
		lobby.is_tutorial,
		lobby.is_rated,
	);
	tokio::spawn(task);
	lobby.game_in_progress = Some(updates_in);

	println!("Game started in lobby {}.", lobby.id);
	lobby.stage = Stage::GameStarted;

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

	Ok(())
}

#[derive(Debug)]
enum Error
{
	EmptyMapPool,
	NoPlayerCount,
	ClientMissing,
	StartGameNotEnoughColors,
	GameEndedWithoutStarting,
	Io
	{
		error: io::Error,
	},
	GeneralChat
	{
		error: mpsc::error::SendError<chat::Update>,
	},
	Game
	{
		error: mpsc::error::SendError<game::Update>,
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

impl From<mpsc::error::SendError<game::Update>> for Error
{
	fn from(error: mpsc::error::SendError<game::Update>) -> Self
	{
		Error::Game { error }
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
			Error::GameEndedWithoutStarting => write!(f, "{:#?}", self),
			Error::Io { error } => error.fmt(f),
			Error::GeneralChat { error } => error.fmt(f),
			Error::Game { error } => error.fmt(f),
			Error::AiAllocationError { error } =>
			{
				write!(f, "Error while allocating AI: {}", error)
			}
		}
	}
}

impl std::error::Error for Error {}

fn initial_name() -> String
{
	const FIRST: [&str; 97] = [
		"Superfluous",
		"Amazing",
		"Exciting",
		"Wonderful",
		"Thirsty",
		"Hungry",
		"Woke",
		"Lit",
		"Dope",
		"Sleepy",
		"Underestimated",
		"Drunk",
		"Handsome",
		"Silly",
		"Clumsy",
		"Ancient",
		"Creepy",
		"Colossal",
		"Delightful",
		"Embarrassing",
		"Superb",
		"Mysterious",
		"Gentle",
		"Bewildered",
		"Important",
		"Fiery",
		"Whack",
		"Entertaining",
		"Dank",
		"Impressive",
		"Finicky",
		"Powerful",
		"Stupendous",
		"Chthonic",
		"Evil",
		"Demonic",
		"Lethargic",
		"Dreamy",
		"Angelic",
		"Badass",
		"Secret",
		"Clandestine",
		"Undercover",
		"Stealthy",
		"Unauthorized",
		"Fraudulent",
		"Covert",
		"Sneaky",
		"Influential",
		"Omnipotent",
		"Omnicient",
		"Persuasive",
		"Mighty",
		"Wicked",
		"Mischievous",
		"Wayward",
		"Dreadful",
		"Outrageous",
		"Dangerous",
		"Barbarous",
		"Exemplary",
		"Well-Behaved",
		"Courteous",
		"Scandalous",
		"Wanton",
		"Disgraceful",
		"Graceful",
		"Naughty",
		"Nefarious",
		"Fierce",
		"Dastardly",
		"Barbaric",
		"Heroic",
		"Brazen",
		"Flagrant",
		"Heinous",
		"Scurrilous",
		"Abominable",
		"Notorious",
		"Noble",
		"Ignoble",
		"Spicy",
		"Tyrannical",
		"Defiant",
		"Fantastic",
		"Haughty",
		"Villainous",
		"Diabolical",
		"Omnipresent",
		"Cacophonic",
		"Lightheaded",
		"Allegorical",
		"Wise",
		"Fresh",
		"Respectable",
		"Nihilistic",
		"Satisfactory",
	];
	const SECOND: [&str; 91] = [
		" Rifleman",
		" Tank",
		" Machinegunner",
		" Settler",
		" Sapper",
		" Militia",
		" Zeppelin",
		" Boys",
		" Girls",
		" Chimpanzee",
		" Caterpillar",
		" Aardvark",
		" Donkey",
		" Moose",
		" Snail",
		" Whale",
		" Platypus",
		" Zebra",
		" Buffalo",
		" Walrus",
		" Wildebeest",
		" Firefighter",
		" Drunks",
		" Hackers",
		" Cuttlefish",
		" Vigilante",
		" Dinosaur",
		" Anteater",
		" Musicians",
		" Superhero",
		" Wizard",
		" Overlord",
		" Astronaut",
		" Rockstar",
		" Ninja",
		" Magician",
		" Dreamers",
		" Hippo",
		" Dragon",
		" Hippopotamus",
		" Firefly",
		" Maniac",
		" Abomination",
		" Spies",
		" Sentinel",
		" Champions",
		" Tyrants",
		" Headbangers",
		" Despot",
		" Individuals",
		" Chicken",
		" Triumphator",
		" Goblin",
		" Hobgoblin",
		" Stamp Collectors",
		" Mathematicians",
		" Philosophers",
		" Scientists",
		" Crook",
		" Desperados",
		" Hoodlum",
		" Culprit",
		" Yardbird",
		" Racketeers",
		" Crook",
		" Guerilla",
		" Gorilla",
		" Alchemists",
		" Cryptozoologists",
		" Drummers",
		" Singers",
		" Guitarists",
		" Bards",
		" Runner-ups",
		" Goody-Two-Shoes",
		" Kung Fu Fighters",
		" Cosmopolitans",
		" Impostors",
		" Samurai",
		" Villains",
		" Wimps",
		" Humans",
		" Weaklings",
		" Dragonling",
		" Earthworm",
		" Winners",
		" Losers",
		" Indie",
		" Nerd",
		" Geek",
		" Daredevil",
	];
	const THIRD: [&str; 88] = [
		" Gathering",
		" Collective",
		" Syndicate",
		" Federation",
		" Conclave",
		" Conference",
		" Congregation",
		" Convocation",
		" Conspiracy",
		" Assembly",
		" Gang",
		" Party",
		" Convention",
		" Group",
		" Lobby",
		" Union",
		" Society",
		" Rally",
		" Meeting",
		" Assemblage",
		" Association",
		" Committee",
		" Crew",
		", Inc.",
		", Ltd.",
		" Cartel",
		" Partnership",
		" Session",
		" Band",
		" Get-Together",
		" Corporation",
		" Cooperative",
		" Guild",
		" Clan",
		" Pack",
		" Coalition",
		" Club",
		" League",
		" Clique",
		" Fraternity",
		" Sorority",
		" Mob",
		" Confederation",
		" Tribe",
		" Alliance",
		" Affiliation",
		" Fellowship",
		" Circle",
		" Company",
		" Commune",
		" Bunch",
		" Faction",
		" Hangout",
		" Lodge",
		" Order",
		" Outfit",
		" Show",
		" Council",
		" Delegation",
		" Meetup",
		" Congress",
		" Fiesta",
		" Apocalypse",
		" Outbreak",
		" Invasion",
		" Summoning",
		" Conflux",
		" Brawl",
		" Conglomeration",
		" Conventicle",
		" Summit",
		" Forum",
		" Collaboration",
		" Coven",
		" Organization",
		" Camp",
		" Sect",
		" Squad",
		" Bloc",
		" Division",
		" Battalion",
		" Crowd",
		" Horde",
		" Throng",
		" Force",
		" Cemetery",
		" Sanctuary",
		" Refuge",
	];

	let mut rng = rand::thread_rng();
	let a = FIRST.choose(&mut rng).unwrap();
	let b = SECOND.choose(&mut rng).unwrap();
	let c = THIRD.choose(&mut rng).unwrap();

	let mut name = String::with_capacity(a.len() + b.len() + c.len());
	name.push_str(a);
	name.push_str(b);
	name.push_str(c);

	name
}
