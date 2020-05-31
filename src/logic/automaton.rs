/* Automaton */

pub use crate::logic::epicinium::AllocationError;

use crate::logic::change::*;
use crate::logic::epicinium;
use crate::logic::epicinium::AllocatedAutomaton;
use crate::logic::order::*;
use crate::logic::player::PlayerColor;

#[derive(Debug)]
pub struct Automaton(AllocatedAutomaton);

impl Automaton
{
	pub fn create(
		players: Vec<PlayerColor>,
		ruleset_name: &str,
	) -> Result<Automaton, AllocationError>
	{
		let allocated = epicinium::allocate_automaton(players, ruleset_name)?;
		Ok(Automaton(allocated))
	}

	pub fn grant_global_vision(&mut self, player: PlayerColor)
	{
		epicinium::grant_global_vision(&mut self.0, player);
	}

	pub fn load(
		&mut self,
		_map_name: String,
		_shuffleplayers: bool,
		_metadata: Metadata,
	)
	{
		unimplemented!()
	}

	pub fn is_active(&self) -> bool
	{
		unimplemented!()
	}

	pub fn act(&mut self) -> ChangeSet
	{
		unimplemented!()
	}

	pub fn is_gameover(&self) -> bool
	{
		unimplemented!()
	}

	pub fn is_defeated(&self, _player: PlayerColor) -> bool
	{
		unimplemented!()
	}

	pub fn hibernate(&mut self) -> ChangeSet
	{
		unimplemented!()
	}

	pub fn awake(&mut self) -> ChangeSet
	{
		unimplemented!()
	}

	pub fn receive(&mut self, _player: PlayerColor, _orders: Vec<Order>)
	{
		unimplemented!()
	}

	pub fn prepare(&mut self) -> ChangeSet
	{
		unimplemented!()
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
	// TODO metadata
}
