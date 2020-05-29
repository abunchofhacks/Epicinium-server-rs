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
	pub sendbuffer: mpsc::Sender<Message>,

	pub color: PlayerColor,
	pub vision: VisionType,
	// TODO flags
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
	pub sendbuffer: mpsc::Sender<Message>,

	pub role: Role,
	// TODO flags
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
	players: Vec<PlayerClient>,
	bots: Vec<Bot>,
	_watchers: Vec<WatcherClient>,
	_map_name: String,
	ruleset_name: String,
	_planning_time_in_seconds: Option<u32>,
	_challenge: Option<ChallengeId>,
	_is_tutorial: bool,
	_is_rated: bool,
) -> Result<(), Error>
{
	// TODO challenge
	// TODO tutorial
	// TODO rated

	// TODO use planning time

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
