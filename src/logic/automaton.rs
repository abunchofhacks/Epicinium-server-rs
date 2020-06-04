/* Automaton */

pub use crate::logic::epicinium::InterfaceError;

use crate::logic::change::ChangeSet;
use crate::logic::epicinium;
use crate::logic::epicinium::AllocatedAutomaton;
use crate::logic::order::Order;
use crate::logic::player::PlayerColor;

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
		epicinium::grant_global_vision(&mut self.0, player);
	}

	pub fn load(
		&mut self,
		map_name: String,
		shuffleplayers: bool,
		metadata: Metadata,
	) -> Result<(), InterfaceError>
	{
		epicinium::load_map(&mut self.0, map_name, shuffleplayers, metadata)
	}

	pub fn is_active(&mut self) -> bool
	{
		epicinium::automaton_is_active(&mut self.0)
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
		// TODO implement
		println!("{:?} resigns", player);
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata {
	// TODO metadata
}
