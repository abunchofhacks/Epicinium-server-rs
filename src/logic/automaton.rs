/* Automaton */

pub use crate::logic::challenge::ChallengeId;
pub use crate::logic::epicinium::InterfaceError;

use crate::logic::ai;
use crate::logic::change::ChangeSet;
use crate::logic::epicinium;
use crate::logic::epicinium::AllocatedAutomaton;
use crate::logic::map;
use crate::logic::order::Order;
use crate::logic::player::PlayerColor;

use serde_derive::{Deserialize, Serialize};

#[derive(Debug)]
pub struct Automaton(AllocatedAutomaton);

impl Automaton
{
	pub fn create(
		players: Vec<PlayerColor>,
		ruleset_name: &str,
	) -> Result<Automaton, InterfaceError>
	{
		let allocated = epicinium::allocate_automaton(players, ruleset_name)?;
		Ok(Automaton(allocated))
	}

	pub fn grant_global_vision(&mut self, player: PlayerColor)
	{
		epicinium::automaton_grant_global_vision(&mut self.0, player);
	}

	pub fn load(
		&mut self,
		map_name: String,
		shuffleplayers: bool,
	) -> Result<(), InterfaceError>
	{
		epicinium::automaton_load_map(&mut self.0, map_name, shuffleplayers)
	}

	pub fn restore(
		&mut self,
		recording_name: String,
	) -> Result<(), InterfaceError>
	{
		epicinium::automaton_restore(&mut self.0, recording_name)
	}

	pub fn load_replay(
		&mut self,
		recording_name: String,
	) -> Result<(), InterfaceError>
	{
		epicinium::automaton_load_replay(&mut self.0, recording_name)
	}

	pub fn start_recording(
		&mut self,
		metadata: Metadata,
		recording_name: String,
	) -> Result<(), InterfaceError>
	{
		epicinium::automaton_start_recording(
			&mut self.0,
			metadata,
			recording_name,
		)
	}

	pub fn set_challenge(
		&mut self,
		challenge_id: ChallengeId,
	) -> Result<(), InterfaceError>
	{
		epicinium::automaton_set_challenge(&mut self.0, challenge_id)
	}

	pub fn is_active(&mut self) -> bool
	{
		epicinium::automaton_is_active(&mut self.0)
	}

	pub fn is_replay_active(&mut self) -> bool
	{
		epicinium::automaton_is_replay_active(&mut self.0)
	}

	pub fn act(&mut self) -> Result<ChangeSet, InterfaceError>
	{
		epicinium::automaton_act(&mut self.0)
	}

	pub fn is_gameover(&mut self) -> bool
	{
		epicinium::automaton_is_gameover(&mut self.0)
	}

	pub fn is_defeated(&mut self, player: PlayerColor) -> bool
	{
		epicinium::automaton_is_defeated(&mut self.0, player)
	}

	pub fn current_round(&mut self) -> u32
	{
		epicinium::automaton_current_round(&mut self.0)
	}

	pub fn global_score(&mut self) -> i32
	{
		epicinium::automaton_global_score(&mut self.0)
	}

	pub fn score(&mut self, player: PlayerColor) -> i32
	{
		epicinium::automaton_score(&mut self.0, player)
	}

	pub fn award(&mut self, player: PlayerColor) -> i32
	{
		epicinium::automaton_award(&mut self.0, player)
	}

	pub fn hibernate(&mut self) -> Result<ChangeSet, InterfaceError>
	{
		epicinium::automaton_hibernate(&mut self.0)
	}

	pub fn awake(&mut self) -> Result<ChangeSet, InterfaceError>
	{
		epicinium::automaton_awake(&mut self.0)
	}

	pub fn receive(
		&mut self,
		player: PlayerColor,
		orders: Vec<Order>,
	) -> Result<(), InterfaceError>
	{
		epicinium::automaton_receive(&mut self.0, player, orders)
	}

	pub fn prepare(&mut self) -> Result<ChangeSet, InterfaceError>
	{
		epicinium::automaton_prepare(&mut self.0)
	}

	pub fn resign(&mut self, player: PlayerColor)
	{
		epicinium::automaton_resign(&mut self.0, player)
	}

	pub fn rejoin(
		&mut self,
		player: PlayerColor,
	) -> Result<ChangeSet, InterfaceError>
	{
		epicinium::automaton_rejoin(&mut self.0, player)
	}
}

// These are only the fields that we need to supply to libepicinium.
#[derive(Debug, Clone, Serialize)]
pub struct Metadata
{
	#[serde(rename = "map")]
	pub map_name: String,

	#[serde(rename = "online")]
	pub is_online: bool,

	#[serde(rename = "planningtime")]
	pub planning_time_in_seconds_or_zero: u32,

	pub players: Vec<Player>,
	pub watchers: Vec<Watcher>,
	pub bots: Vec<Bot>,

	#[serde(flatten)]
	pub map_metadata: map::Metadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player
{
	#[serde(rename = "player")]
	pub color: PlayerColor,

	pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Watcher
{
	pub username: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bot
{
	#[serde(rename = "player")]
	pub color: PlayerColor,

	#[serde(flatten)]
	pub ai_metadata: ai::Metadata,
}
