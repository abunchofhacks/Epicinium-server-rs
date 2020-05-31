/* AI */

pub use crate::logic::epicinium::InterfaceError;

use crate::logic::change::Change;
use crate::logic::difficulty::Difficulty;
use crate::logic::epicinium;
use crate::logic::epicinium::AllocatedAi;
use crate::logic::order::Order;
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
	) -> Result<Commander, InterfaceError>
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

	pub fn receive(
		&mut self,
		changes: Vec<Change>,
	) -> Result<(), InterfaceError>
	{
		epicinium::ai_receive(&mut self.0, changes)
	}

	pub fn prepare_orders(&mut self)
	{
		epicinium::ai_prepare_orders(&mut self.0)
	}

	pub fn retrieve_orders(&mut self) -> Result<Vec<Order>, InterfaceError>
	{
		epicinium::ai_retrieve_orders(&mut self.0)
	}
}
