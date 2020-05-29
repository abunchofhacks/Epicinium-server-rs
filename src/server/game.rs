/* ServerGame */

use crate::common::keycode::*;
use crate::logic::ai;
use crate::logic::automaton::Automaton;
use crate::logic::challenge::ChallengeId;
use crate::logic::player::PlayerColor;
use crate::server::botslot::Botslot;
use crate::server::lobby;
use crate::server::message::*;

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

pub async fn run(
	lobby_id: Keycode,
	mut end_update: mpsc::Sender<lobby::Update>,
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
)
{
	// TODO challenge
	// TODO tutorial
	// TODO rated

	// TODO use planning time

	let mut playercolors = Vec::new();
	for player in players
	{
		playercolors.push(player.color);
	}
	for bot in bots
	{
		playercolors.push(bot.color);
	}

	let _automaton = Automaton::create(playercolors, &ruleset_name);

	match end_update.send(lobby::Update::GameEnded).await
	{
		Ok(()) =>
		{}
		Err(error) =>
		{
			eprintln!(
				"Game ended after lobby {} crashed: {:?}",
				lobby_id, error
			);
		}
	}
}

#[derive(Debug)]
pub enum Update {
	// TODO
}
