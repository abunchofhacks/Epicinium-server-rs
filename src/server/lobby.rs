/* Server::Lobby */

use crate::common::keycode::*;
use crate::logic::map;
use crate::server::chat;
use crate::server::client;
use crate::server::message::*;

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::io;
use std::sync;
use std::sync::atomic;

use rand::seq::SliceRandom;

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
		general_chat: mpsc::Sender<chat::Update>,
		ruleset_name: String,
	},

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

#[derive(Debug, Clone)]
struct Lobby
{
	id: Keycode,
	name: String,
	num_players: i32,
	max_players: i32,
	is_public: bool,
	is_replay: bool,

	roles: HashMap<Keycode, Role>,

	map_pool: Vec<(String, map::Metadata)>,
	map_name: String,
	ruleset_name: String,
	ruleset_confirmations: HashSet<Keycode>,
	timer_in_seconds: u32,

	is_waiting_for_confirmation: bool,
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
		roles: HashMap::new(),
		map_pool,
		map_name,
		ruleset_name,
		ruleset_confirmations: HashSet::new(),
		timer_in_seconds: 60,
		is_waiting_for_confirmation: false,
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

		Update::PickMap {
			mut general_chat,
			map_name,
		} =>
		{
			pick_map(lobby, clients, map_name).await?;
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
			mut general_chat,
			ruleset_name,
		} =>
		{
			handle_ruleset_confirmation(
				lobby,
				clients,
				client_id,
				&mut general_chat,
				ruleset_name,
			)
			.await
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
	lobby: &Lobby,
	lobby_sendbuffer: mpsc::Sender<Update>,
	general_chat: &mut mpsc::Sender<chat::Update>,
) -> Result<(), Error>
{
	let update = chat::Update::ListLobby {
		lobby_id: lobby.id,
		description_messages: make_listing_messages(&lobby),
		sendbuffer: lobby_sendbuffer,
	};
	general_chat.send(update).await?;
	Ok(())
}

async fn describe_lobby(
	lobby: &Lobby,
	general_chat: &mut mpsc::Sender<chat::Update>,
) -> Result<(), Error>
{
	let update = chat::Update::DescribeLobby {
		lobby_id: lobby.id,
		description_messages: make_listing_messages(&lobby),
	};
	general_chat.send(update).await?;
	Ok(())
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
			lobby_name: lobby.name.clone(),
		},
		Message::MaxPlayers {
			lobby_id: lobby.id,
			value: lobby.max_players,
		},
		Message::NumPlayers {
			lobby_id: lobby.id,
			value: lobby.num_players,
		},
		if lobby.is_public
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

	// TODO AI pool
	// TODO bots

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

	// The newcomer will be announced globally.

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

	Ok(())
}

fn do_leave(lobby: &mut Lobby, client_id: Keycode, clients: &mut Vec<Client>)
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

	lobby.roles.remove(&client_id);
	// TODO change num_players
	// TODO colors
	// TODO visiontypes
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

	lobby.roles.insert(client_id, assigned_role);

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
		// TODO bots
	}

	Ok(())
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
	// TODO remove bots
	for _ in 0..0
	{
		if newcount < playercount
		{
			newcount += 1;
			botcount += 1;
		}
		else
		{
			// TODO remove bots
		}
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
			// TODO add a bot
		}
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
	general_chat: &mut mpsc::Sender<chat::Update>,
	ruleset_name: String,
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

	if lobby.is_waiting_for_confirmation && is_ruleset_confirmed(lobby, clients)
	{
		// Start the game once everyone has confirmed.
		try_start(lobby, clients, general_chat).await
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
		if lobby.ruleset_confirmations.contains(&client.id)
		{
			return false;
		}
	}
	return true;
}

async fn try_start(
	_lobby: &mut Lobby,
	_clients: &mut Vec<Client>,
	_general_chat: &mut mpsc::Sender<chat::Update>,
) -> Result<(), Error>
{
	unimplemented!()
}

#[derive(Debug)]
enum Error
{
	EmptyMapPool,
	NoPlayerCount,
	ClientMissing,
	Io
	{
		error: io::Error,
	},
	GeneralChat
	{
		error: mpsc::error::SendError<chat::Update>,
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
			Error::EmptyMapPool => write!(f, "The map pool is empty"),
			Error::NoPlayerCount => write!(f, "No player count defined"),
			Error::ClientMissing => write!(f, "Client missing"),
			Error::Io { error } => error.fmt(f),
			Error::GeneralChat { error } => error.fmt(f),
		}
	}
}

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
