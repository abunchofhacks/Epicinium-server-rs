/*
 * Part of epicinium_server
 * developed by A Bunch of Hacks.
 *
 * Copyright (c) 2018-2021 A Bunch of Hacks
 *
 * This library is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This library is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 * [authors:]
 * Sander in 't Veld (sander@abunchofhacks.coop)
 */

pub use epicinium_lib::error::InterfaceError;
pub use epicinium_lib::logic::ai::Metadata;

use crate::logic::change::Change;
use crate::logic::difficulty::Difficulty;
use crate::logic::order::Order;
use crate::logic::player::PlayerColor;

use epicinium_lib::AllocatedAi;

pub fn exists(ainame: &str) -> bool
{
	epicinium_lib::ai_exists(ainame)
}

pub fn load_pool() -> Vec<String>
{
	epicinium_lib::ai_pool()
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
		let allocated = epicinium_lib::allocate_ai(
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
		epicinium_lib::ai_receive(&mut self.0, changes)
	}

	pub fn prepare_orders(&mut self)
	{
		epicinium_lib::ai_prepare_orders(&mut self.0)
	}

	pub fn retrieve_orders(&mut self) -> Result<Vec<Order>, InterfaceError>
	{
		epicinium_lib::ai_retrieve_orders(&mut self.0)
	}

	pub fn descriptive_name(&mut self) -> Result<String, InterfaceError>
	{
		epicinium_lib::ai_descriptive_name(&mut self.0)
	}

	pub fn metadata(&mut self) -> Result<Metadata, InterfaceError>
	{
		epicinium_lib::ai_descriptive_metadata(&mut self.0)
	}
}
