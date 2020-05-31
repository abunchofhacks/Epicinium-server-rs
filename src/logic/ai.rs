/* AI */

pub use crate::logic::epicinium::AllocationError;

use crate::logic::change::*;
use crate::logic::difficulty::Difficulty;
use crate::logic::epicinium;
use crate::logic::epicinium::AllocatedAi;
use crate::logic::order::*;
use crate::logic::player::PlayerColor;

pub fn exists(ainame: &str) -> bool
{
	epicinium::ai_exists(ainame)
}

pub fn load_pool() -> Vec<String>
{
	epicinium::ai_pool()
}

#[derive(Debug)]
pub struct Commander(AllocatedAi);

impl Commander
{
	pub fn create(
		name: &str,
		player: PlayerColor,
		difficulty: Difficulty,
		ruleset_name: &str,
		character: u8,
	) -> Result<Commander, AllocationError>
	{
		let allocated = epicinium::allocate_ai(
			name,
			player,
			difficulty,
			ruleset_name,
			character,
		)?;
		Ok(Commander(allocated))
	}

	pub fn receive(&mut self, _changes: Vec<Change>)
	{
		unimplemented!()
	}

	pub fn prepare_orders(&mut self)
	{
		unimplemented!()
	}

	pub fn retrieve_orders(&mut self) -> Vec<Order>
	{
		unimplemented!()
	}
}
