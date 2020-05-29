/* Automaton */

pub use crate::logic::epicinium::AllocationError;

use crate::logic::epicinium;
use crate::logic::epicinium::AllocatedAutomaton;
use crate::logic::player::PlayerColor;

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
}
