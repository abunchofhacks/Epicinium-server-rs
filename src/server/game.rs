/* ServerGame */

use crate::common::keycode::*;
use crate::logic::ai;
use crate::logic::automaton;
use crate::logic::automaton::Automaton;
use crate::logic::challenge::ChallengeId;
use crate::logic::change::ChangeSet;
use crate::logic::order::Order;
use crate::logic::player::PlayerColor;
use crate::server::botslot::Botslot;
use crate::server::lobby;
use crate::server::message::*;

use std::fmt;

use tokio::sync::mpsc;

#[derive(Debug)]
pub struct PlayerClient
{
	pub id: Keycode,
	pub username: String,
	pub sendbuffer: Option<mpsc::Sender<Message>>,

	pub color: PlayerColor,
	pub vision: VisionType,

	pub is_defeated: bool,
	pub is_retired: bool,
	pub has_synced: bool,
	pub submitted_orders: Option<Vec<Order>>,
}

impl PlayerClient
{
	fn is_disconnected(&self) -> bool
	{
		self.sendbuffer.is_none()
	}

	fn send(&mut self, message: Message)
	{
		let result = match &mut self.sendbuffer
		{
			Some(sendbuffer) => sendbuffer.try_send(message),
			None => Ok(()),
		};

		match result
		{
			Ok(()) => (),
			Err(_error) => self.sendbuffer = None,
		}
	}
}

#[derive(Debug)]
pub struct Bot
{
	pub slot: Botslot,
	pub ai: ai::Commander,

	pub color: PlayerColor,
	pub vision: VisionType,

	pub is_defeated: bool,
}

#[derive(Debug)]
pub struct WatcherClient
{
	pub id: Keycode,
	pub username: String,
	pub sendbuffer: Option<mpsc::Sender<Message>>,

	pub role: Role,
	pub vision_level: PlayerColor,

	pub has_synced: bool,
}

impl WatcherClient
{
	fn is_disconnected(&self) -> bool
	{
		self.sendbuffer.is_some()
	}

	fn send(&mut self, message: Message)
	{
		let result = match &mut self.sendbuffer
		{
			Some(sendbuffer) => sendbuffer.try_send(message),
			None => Ok(()),
		};

		match result
		{
			Ok(()) => (),
			Err(_error) => self.sendbuffer = None,
		}
	}
}

pub async fn start(
	lobby_id: Keycode,
	mut end_update: mpsc::Sender<lobby::Update>,
	updates: mpsc::Receiver<Update>,
	players: Vec<PlayerClient>,
	bots: Vec<Bot>,
	watchers: Vec<WatcherClient>,
	map_name: String,
	ruleset_name: String,
	planning_time_in_seconds: Option<u32>,
	challenge: Option<ChallengeId>,
	is_tutorial: bool,
	is_rated: bool,
)
{
	let result = run(
		updates,
		players,
		bots,
		watchers,
		map_name,
		ruleset_name,
		planning_time_in_seconds,
		challenge,
		is_tutorial,
		is_rated,
	)
	.await;

	match result
	{
		Ok(()) =>
		{}
		Err(error) =>
		{
			eprintln!("Game in lobby {} crashed: {:#?}", lobby_id, error);
		}
	}

	match end_update.send(lobby::Update::GameEnded).await
	{
		Ok(()) =>
		{}
		Err(error) =>
		{
			eprintln!("Game ended after its lobby {}: {:#?}", lobby_id, error);
		}
	}
}

async fn run(
	mut updates: mpsc::Receiver<Update>,
	mut players: Vec<PlayerClient>,
	mut bots: Vec<Bot>,
	mut watchers: Vec<WatcherClient>,
	map_name: String,
	ruleset_name: String,
	planning_time_in_seconds: Option<u32>,
	challenge: Option<ChallengeId>,
	is_tutorial: bool,
	is_rated: bool,
) -> Result<(), Error>
{
	// TODO metadata
	let metadata = automaton::Metadata {};

	let mut playercolors = Vec::new();
	for player in &players
	{
		playercolors.push(player.color);
	}
	for bot in &bots
	{
		playercolors.push(bot.color);
	}

	let mut automaton = Automaton::create(playercolors, &ruleset_name)?;

	// Certain players might have global vision.
	for player in &players
	{
		match player.vision
		{
			VisionType::Normal => (),
			VisionType::Global => automaton.grant_global_vision(player.color),
		}
	}
	for bot in &bots
	{
		match bot.vision
		{
			VisionType::Normal => (),
			VisionType::Global => automaton.grant_global_vision(bot.color),
		}
	}

	for client in &mut players
	{
		if is_tutorial
		{
			client.send(Message::Tutorial {
				role: Some(Role::Player),
				player: Some(client.color),
				ruleset_name: Some(ruleset_name.clone()),
				timer_in_seconds: planning_time_in_seconds,
			});
		}
		else
		{
			client.send(Message::Game {
				role: Some(Role::Player),
				player: Some(client.color),
				ruleset_name: Some(ruleset_name.clone()),
				timer_in_seconds: planning_time_in_seconds,
			});
		}
	}

	for client in &mut watchers
	{
		client.send(Message::Game {
			role: Some(Role::Observer),
			player: None,
			ruleset_name: Some(ruleset_name.clone()),
			timer_in_seconds: planning_time_in_seconds,
		});
	}

	// Tell everyone who is playing as which color.
	// TODO colors

	// Tell everyone which skins are being used.
	// TODO skins

	// A challenge might be set.
	// TODO set in automaton
	// TODO tell clients mission briefing

	// Load the map.
	{
		let shuffleplayers = challenge.is_none()
			&& !map_name.contains("demo")
			&& !map_name.contains("tutorial");
		automaton.load(map_name, shuffleplayers, metadata)?;
	}

	if is_rated
	{
		// TODO rating
	}

	loop
	{
		let state = iterate(
			&mut automaton,
			&mut players,
			&mut bots,
			&mut watchers,
			&mut updates,
			planning_time_in_seconds,
		)
		.await?;

		match state
		{
			State::InProgress => (),
			State::Finished => break,
		}
	}

	// TODO handle rejoins of observers that joined at the last moment

	// Is this a competitive 1v1 match with two humans?
	// TODO rating

	// If there are non-bot non-observer participants, adjust their ratings.
	// TODO retire

	Ok(())
}

#[derive(Debug)]
pub enum Update
{
	Orders
	{
		client_id: Keycode,
		orders: Vec<Order>,
	},
	Sync
	{
		client_id: Keycode
	},
	Resign
	{
		client_id: Keycode
	},
}

#[derive(Debug, PartialEq, Eq)]
enum State
{
	InProgress,
	Finished,
}

async fn iterate(
	automaton: &mut Automaton,
	players: &mut Vec<PlayerClient>,
	bots: &mut Vec<Bot>,
	watchers: &mut Vec<WatcherClient>,
	updates: &mut mpsc::Receiver<Update>,
	planning_time_in_seconds: Option<u32>,
) -> Result<State, Error>
{
	while automaton.is_active()
	{
		let cset = automaton.act()?;
		broadcast(players, bots, watchers, cset)?;
	}

	// If players are defeated, we no longer wait for them in the
	// planning phase.
	for player in players.into_iter()
	{
		if automaton.is_defeated(player.color)
		{
			player.is_defeated = true;
		}
	}
	// If bots are defeated, we no longer ask them for orders in the
	// action phase.
	for bot in bots.into_iter()
	{
		if automaton.is_defeated(bot.color)
		{
			bot.is_defeated = true;
		}
	}

	rest(players, watchers, updates).await?;

	// If the game has ended, we are done.
	// We waited with this check until all players have finished animating,
	// to avoid spoiling the outcome by changing ratings or posting to Discord.
	if automaton.is_gameover()
	{
		return Ok(State::Finished);
	}

	// If all live players are disconnected during the resting phase,
	// the game cannot continue until at least one player reconnects
	// and has finished rejoining.
	ensure_at_least_one_live_player(players, watchers, updates).await?;

	let message = Message::Sync {
		planning_time_in_seconds,
	};
	for client in players.into_iter()
	{
		client.has_synced = false;
		client.send(message.clone());
	}
	for client in watchers.into_iter()
	{
		client.has_synced = false;
		client.send(message.clone());
	}

	let cset = automaton.hibernate()?;
	broadcast(players, bots, watchers, cset)?;

	// Allow the bots to calculate their next move.
	for bot in bots.into_iter()
	{
		bot.ai.prepare_orders();
	}

	let num_bots = bots.len();
	sleep(players, num_bots, watchers, updates).await?;

	let cset = automaton.awake()?;
	broadcast(players, bots, watchers, cset)?;

	stage(players, watchers, updates).await?;

	for player in players.into_iter()
	{
		if let Some(orders) = player.submitted_orders.take()
		{
			automaton.receive(player.color, orders)?;
		}
	}

	for bot in bots.into_iter()
	{
		if !bot.is_defeated
		{
			let orders = bot.ai.retrieve_orders()?;
			automaton.receive(bot.color, orders)?;
		}
	}

	let cset = automaton.prepare()?;
	broadcast(players, bots, watchers, cset)?;

	Ok(State::InProgress)
}

fn broadcast(
	players: &mut Vec<PlayerClient>,
	bots: &mut Vec<Bot>,
	watchers: &mut Vec<WatcherClient>,
	cset: ChangeSet,
) -> Result<(), Error>
{
	for client in players
	{
		let changes = cset.get(client.color);
		let message = Message::Changes { changes };
		client.send(message);
	}

	for bot in bots
	{
		let changes = cset.get(bot.color);
		bot.ai.receive(changes)?;
	}

	for client in watchers
	{
		let changes = cset.get(client.vision_level);
		let message = Message::Changes { changes };
		client.send(message);
	}

	Ok(())
}

async fn rest(
	players: &mut Vec<PlayerClient>,
	watchers: &mut Vec<WatcherClient>,
	updates: &mut mpsc::Receiver<Update>,
) -> Result<(), Error>
{
	// Start the planning phase when all players (or all watchers if
	// if this is a replay lobby) are ready. Players and watchers can
	// reconnect in the resting phase while others are animating.
	// Players will wait until other players have fully rejoined,
	// and watchers wait until other watchers have rejoined.
	while !all_players_or_watchers_have_synced(players, watchers)
	{
		let update = match updates.recv().await
		{
			Some(update) => update,
			None => return Err(Error::LobbyGone),
		};

		// TODO handle rejoins
		match update
		{
			Update::Orders { client_id, .. } =>
			{
				eprintln!("Ignoring orders from {} while resting", client_id);
			}
			Update::Sync { client_id } =>
			{
				for client in players.iter_mut()
				{
					if client.id == client_id
					{
						client.has_synced = true;
					}
				}
				for client in watchers.iter_mut()
				{
					if client.id == client_id
					{
						client.has_synced = true;
					}
				}
			}
			Update::Resign { client_id: _ } =>
			{
				// TODO handle resign
			}
		}
	}

	Ok(())
}

fn all_players_or_watchers_have_synced(
	players: &mut Vec<PlayerClient>,
	watchers: &mut Vec<WatcherClient>,
) -> bool
{
	if players.iter().find(|x| !x.is_disconnected()).is_some()
	{
		players
			.iter()
			.find(|x| !x.is_disconnected() && !x.has_synced)
			.is_none()
	}
	else
	{
		watchers
			.iter()
			.find(|x| !x.is_disconnected() && !x.has_synced)
			.is_none()
	}
}

async fn ensure_at_least_one_live_player(
	players: &mut Vec<PlayerClient>,
	_watchers: &mut Vec<WatcherClient>,
	updates: &mut mpsc::Receiver<Update>,
) -> Result<(), Error>
{
	if players.is_empty()
	{
		return Ok(());
	}

	while !at_least_one_live_player(players)
	{
		let update = match updates.recv().await
		{
			Some(update) => update,
			None => return Err(Error::LobbyGone),
		};

		// TODO handle rejoins
		match update
		{
			Update::Orders { client_id, .. } =>
			{
				eprintln!("Ignoring orders from {} after resting", client_id);
			}
			Update::Sync { client_id } =>
			{
				eprintln!("Ignoring sync from {} after resting", client_id);
			}
			Update::Resign { client_id: _ } =>
			{
				// TODO handle resign
			}
		}
	}

	Ok(())
}

fn at_least_one_live_player(players: &mut Vec<PlayerClient>) -> bool
{
	players
		.iter()
		.find(|x| !x.is_defeated && !x.is_retired && !x.is_disconnected())
		.is_some()
}

async fn sleep(
	players: &mut Vec<PlayerClient>,
	num_bots: usize,
	_watchers: &mut Vec<WatcherClient>,
	updates: &mut mpsc::Receiver<Update>,
) -> Result<(), Error>
{
	if too_few_potential_winners(players, num_bots)
	{
		println!("Ending sleep early");
		return Ok(());
	}

	// Start staging when either the timer runs out or all non-defeated
	// players are ready, or when all players except one are undefeated
	// or have retired. Players and watchers can reconnect in the
	// planning phase if there is still time.
	// TODO timer
	while !all_players_have_submitted_orders(players)
	{
		let update = match updates.recv().await
		{
			Some(update) => update,
			None => return Err(Error::LobbyGone),
		};

		// TODO handle rejoins
		match update
		{
			Update::Orders { client_id, orders } =>
			{
				for client in players.iter_mut()
				{
					if client.id == client_id
					{
						client.submitted_orders = Some(orders);
						break;
					}
				}
			}
			Update::Sync { client_id } =>
			{
				eprintln!("Ignoring sync from {} while sleeping", client_id);
			}
			Update::Resign { client_id: _ } =>
			{
				// TODO handle resign
				if too_few_potential_winners(players, num_bots)
				{
					println!("Ending sleep early");
					return Ok(());
				}
			}
		}
	}

	Ok(())
}

fn too_few_potential_winners(
	players: &mut Vec<PlayerClient>,
	num_bots: usize,
) -> bool
{
	let potentialwinners = players
		.iter()
		.filter(|x| !x.is_defeated && !x.is_retired)
		.count();

	potentialwinners + num_bots < 2
}

fn all_players_have_submitted_orders(players: &mut Vec<PlayerClient>) -> bool
{
	players
		.iter()
		.filter(|x| !x.is_defeated && !x.is_retired && !x.is_disconnected())
		.all(|x| x.submitted_orders.is_some())
}

async fn stage(
	players: &mut Vec<PlayerClient>,
	_watchers: &mut Vec<WatcherClient>,
	updates: &mut mpsc::Receiver<Update>,
) -> Result<(), Error>
{
	// There is a 10 second grace period for anyone whose orders we
	// haven't received; they might have sent their orders before
	// receiving the staging announcement.
	// TODO timer
	// TODO defer rejoins until later (separate mpsc?)
	while !all_players_have_submitted_orders(players)
	{
		let update = match updates.recv().await
		{
			Some(update) => update,
			None => return Err(Error::LobbyGone),
		};

		// TODO handle rejoins
		match update
		{
			Update::Orders { client_id, orders } =>
			{
				for client in players.iter_mut()
				{
					if client.id == client_id
					{
						client.submitted_orders = Some(orders);
						break;
					}
				}
			}
			Update::Sync { client_id } =>
			{
				eprintln!("Ignoring sync from {} while staging", client_id);
			}
			Update::Resign { client_id: _ } =>
			{
				// TODO handle resign
			}
		}
	}

	Ok(())
}

#[derive(Debug)]
enum Error
{
	LobbyGone,
	Interface(automaton::InterfaceError),
}

impl From<automaton::InterfaceError> for Error
{
	fn from(error: automaton::InterfaceError) -> Self
	{
		Error::Interface(error)
	}
}

impl fmt::Display for Error
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
	{
		match self
		{
			Error::LobbyGone => write!(f, "{:#?}", &self),
			Error::Interface(error) => error.fmt(f),
		}
	}
}

impl std::error::Error for Error {}
