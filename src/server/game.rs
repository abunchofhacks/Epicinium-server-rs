/* ServerGame */

use crate::common::keycode::*;
use crate::logic::ai;
use crate::logic::automaton;
use crate::logic::automaton::Automaton;
use crate::logic::challenge::ChallengeId;
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
	// TODO flags
}

impl PlayerClient
{
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
	pub ai: ai::AllocatedAi,

	pub color: PlayerColor,
	pub vision: VisionType,
	// TODO flags
}

#[derive(Debug)]
pub struct WatcherClient
{
	pub id: Keycode,
	pub username: String,
	pub sendbuffer: Option<mpsc::Sender<Message>>,

	pub role: Role,
	// TODO flags
}

impl WatcherClient
{
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
	_updates: mpsc::Receiver<Update>,
	mut players: Vec<PlayerClient>,
	mut bots: Vec<Bot>,
	mut watchers: Vec<WatcherClient>,
	_map_name: String,
	ruleset_name: String,
	planning_time_in_seconds: Option<u32>,
	_challenge: Option<ChallengeId>,
	is_tutorial: bool,
	is_rated: bool,
) -> Result<(), Error>
{
	// TODO metadata

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

	// Load the map or replay.
	// TODO

	if is_rated
	{
		// TODO rating
	}

	Ok(())
}

#[derive(Debug)]
pub enum Update {
	// TODO
}

#[derive(Debug)]
enum Error
{
	AllocationError(automaton::AllocationError),
}

impl From<automaton::AllocationError> for Error
{
	fn from(error: automaton::AllocationError) -> Self
	{
		Error::AllocationError(error)
	}
}

impl fmt::Display for Error
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
	{
		match self
		{
			Error::AllocationError(error) =>
			{
				write!(f, "Error while allocating: {}", error)
			}
		}
	}
}

impl std::error::Error for Error {}
