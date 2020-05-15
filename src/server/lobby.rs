/* Server::Lobby */

use crate::common::keycode::*;
use crate::server::chat;
use crate::server::client;
use crate::server::message::*;

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

	Msg(Message),
}

pub fn create(ticker: &mut sync::Arc<atomic::AtomicU64>)
	-> mpsc::Sender<Update>
{
	let key = rand::random();
	let data = ticker.fetch_add(1, atomic::Ordering::Relaxed);
	let lobby_id = keycode(key, data);

	let (updates_in, updates_out) = mpsc::channel::<Update>(1000);

	let lobby = Lobby {
		id: lobby_id,
		name: initial_name(),
		num_players: 0,
		max_players: 2,
		public: true,
	};

	let task = run(lobby, updates_out);
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
}

async fn run(mut lobby: Lobby, mut updates: mpsc::Receiver<Update>)
{
	let lobby_id = lobby.id;
	let mut clients: Vec<Client> = Vec::new();

	while let Some(update) = updates.recv().await
	{
		match handle_update(update, &mut lobby, &mut clients).await
		{
			Ok(()) => continue,
			Err(error) => eprintln!("Lobby {} crashed: {:?}", lobby_id, error),
		}
	}

	println!("Lobby {} has disbanded.", lobby_id);
}

async fn handle_update(
	update: Update,
	lobby: &mut Lobby,
	clients: &mut Vec<Client>,
) -> Result<(), mpsc::error::SendError<chat::Update>>
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
			lobby.public = false;
			describe_lobby(lobby, &mut general_chat).await
		}
		Update::Unlock { mut general_chat } =>
		{
			lobby.public = true;
			describe_lobby(lobby, &mut general_chat).await
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
) -> Result<(), mpsc::error::SendError<chat::Update>>
{
	let update = chat::Update::ListLobby {
		lobby_id: lobby.id,
		description_messages: make_listing_messages(&lobby),
		sendbuffer: lobby_sendbuffer,
	};
	general_chat.send(update).await
}

async fn describe_lobby(
	lobby: &Lobby,
	general_chat: &mut mpsc::Sender<chat::Update>,
) -> Result<(), mpsc::error::SendError<chat::Update>>
{
	let update = chat::Update::DescribeLobby {
		lobby_id: lobby.id,
		description_messages: make_listing_messages(&lobby),
	};
	general_chat.send(update).await
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

async fn handle_join(
	lobby: &mut Lobby,
	client_id: Keycode,
	client_username: String,
	client_sendbuffer: mpsc::Sender<Message>,
	client_callback: mpsc::Sender<client::Update>,
	lobby_sendbuffer: mpsc::Sender<Update>,
	general_chat: &mut mpsc::Sender<chat::Update>,
	clients: &mut Vec<Client>,
) -> Result<(), mpsc::error::SendError<chat::Update>>
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
	general_chat.send(update).await
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
) -> Result<(), mpsc::error::SendError<chat::Update>>
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
