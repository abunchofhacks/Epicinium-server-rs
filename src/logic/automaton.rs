/* Automaton */

pub use crate::logic::challenge::ChallengeId;
pub use epicinium_lib::error::InterfaceError;
pub use epicinium_lib::logic::automaton::BotMetadata;
pub use epicinium_lib::logic::automaton::Metadata;
pub use epicinium_lib::logic::automaton::PlayerMetadata;
pub use epicinium_lib::logic::automaton::WatcherMetadata;

use crate::logic::change::ChangeSet;
use crate::logic::order::Order;
use crate::logic::player::PlayerColor;

use epicinium_lib::AllocatedAutomaton;

#[derive(Debug)]
pub struct Automaton(AllocatedAutomaton);

impl Automaton
{
	pub fn create(
		players: Vec<PlayerColor>,
		ruleset_name: &str,
	) -> Result<Automaton, InterfaceError>
	{
		let allocated =
			epicinium_lib::allocate_automaton(players, ruleset_name)?;
		Ok(Automaton(allocated))
	}

	pub fn grant_global_vision(&mut self, player: PlayerColor)
	{
		epicinium_lib::automaton_grant_global_vision(&mut self.0, player);
	}

	pub fn load(
		&mut self,
		map_name: String,
		shuffleplayers: bool,
	) -> Result<(), InterfaceError>
	{
		epicinium_lib::automaton_load_map(&mut self.0, map_name, shuffleplayers)
	}

	pub fn restore(
		&mut self,
		recording_name: String,
	) -> Result<(), InterfaceError>
	{
		epicinium_lib::automaton_restore(&mut self.0, recording_name)
	}

	pub fn load_replay(
		&mut self,
		recording_name: String,
	) -> Result<(), InterfaceError>
	{
		epicinium_lib::automaton_load_replay(&mut self.0, recording_name)
	}

	pub fn start_recording(
		&mut self,
		metadata: Metadata,
		recording_name: String,
	) -> Result<(), InterfaceError>
	{
		epicinium_lib::automaton_start_recording(
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
		epicinium_lib::automaton_set_challenge(&mut self.0, challenge_id)
	}

	pub fn is_active(&mut self) -> bool
	{
		epicinium_lib::automaton_is_active(&mut self.0)
	}

	pub fn is_replay_active(&mut self) -> bool
	{
		epicinium_lib::automaton_is_replay_active(&mut self.0)
	}

	pub fn act(&mut self) -> Result<ChangeSet, InterfaceError>
	{
		epicinium_lib::automaton_act(&mut self.0)
	}

	pub fn is_gameover(&mut self) -> bool
	{
		epicinium_lib::automaton_is_gameover(&mut self.0)
	}

	pub fn is_defeated(&mut self, player: PlayerColor) -> bool
	{
		epicinium_lib::automaton_is_defeated(&mut self.0, player)
	}

	pub fn current_round(&mut self) -> u32
	{
		epicinium_lib::automaton_current_round(&mut self.0)
	}

	pub fn global_score(&mut self) -> i32
	{
		epicinium_lib::automaton_global_score(&mut self.0)
	}

	pub fn score(&mut self, player: PlayerColor) -> i32
	{
		epicinium_lib::automaton_score(&mut self.0, player)
	}

	pub fn award(&mut self, player: PlayerColor) -> i32
	{
		epicinium_lib::automaton_award(&mut self.0, player)
	}

	pub fn hibernate(&mut self) -> Result<ChangeSet, InterfaceError>
	{
		epicinium_lib::automaton_hibernate(&mut self.0)
	}

	pub fn awake(&mut self) -> Result<ChangeSet, InterfaceError>
	{
		epicinium_lib::automaton_awake(&mut self.0)
	}

	pub fn receive(
		&mut self,
		player: PlayerColor,
		orders: Vec<Order>,
	) -> Result<(), InterfaceError>
	{
		epicinium_lib::automaton_receive(&mut self.0, player, orders)
	}

	pub fn prepare(&mut self) -> Result<ChangeSet, InterfaceError>
	{
		epicinium_lib::automaton_prepare(&mut self.0)
	}

	pub fn resign(&mut self, player: PlayerColor)
	{
		epicinium_lib::automaton_resign(&mut self.0, player)
	}

	pub fn rejoin(
		&mut self,
		player: PlayerColor,
	) -> Result<ChangeSet, InterfaceError>
	{
		epicinium_lib::automaton_rejoin(&mut self.0, player)
	}
}
