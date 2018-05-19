/* Board */

use std;
use logic::space::*;
use logic::change::*;
use logic::position::*;
use logic::unit::*;
use logic::player::*;


#[derive(Debug)]
pub struct Board
{
	pub cols : i8,
	pub rows : i8,

	pub edge : Space,
	pub cells : Vec<Space>,
}

impl Board
{
	fn index(& self, position : Position) -> i16
	{
		if position.row < 0 || position.row >= self.rows
			|| position.col < 0 || position.col >= self.cols
		{
			return -1;
		}

		(position.row as i16) * (self.cols as i16) + (position.col as i16)
	}

	fn cell(& self, index : i16) -> & Space
	{
		if index < 0 || index as usize > self.cells.len()
		{
			return & self.edge;
		}

		& self.cells[index as usize]
	}

	fn cell_mut(&mut self, index : i16) -> &mut Space
	{
		if index < 0 || index as usize > self.cells.len()
		{
			return &mut self.edge;
		}

		&mut self.cells[index as usize]
	}

	pub fn at(& self, position : Position) -> & Space
	{
		self.cell(self.index(position))
	}

	fn at_mut(&mut self, position : Position) -> &mut Space
	{
		let i = self.index(position);

		self.cell_mut(i)
	}

	pub fn enact(&mut self, change : & Change)
	{
		match change
		{
			& Change::NONE {..} => {},
			& Change::STARTS {..} => {},

			& Change::MOVES {
					subject,
					target,
				} =>
			{
				let s = self.index(subject.position);
				let t = self.index(target.position);

				if s < 0 || s as usize > self.cells.len()
					|| t < 0 || t as usize > self.cells.len()
					|| s == t
				{
					return;
				}

				let min = std::cmp::min(s as usize, t as usize);
				let max = std::cmp::max(s as usize, t as usize);
				let (fst, snd) = self.cells[min..].split_at_mut(max);
				let from = &mut fst[0];
				let to = &mut snd[0];

				swap(from.unit_mut(subject.typ), to.unit_mut(target.typ));
			},

			& Change::REVEAL {
					subject,
					tile,
					snow,
					frostbite,
					firestorm,
					bonedrought,
					death,
					gas,
					radiation,
					temperature,
					humidity,
					chaos,
				} =>
			{
				let space = self.at_mut(subject.position);

				space.vision.add(Player::SELF);

				space.tile = tile;
				space.snow = snow;
				space.frostbite = frostbite;
				space.firestorm = firestorm;
				space.bonedrought = bonedrought;
				space.death = death;
				space.gas = gas;
				space.radiation = radiation;
				space.temperature = temperature;
				space.humidity = humidity;
				space.chaos = chaos;
			},

			& Change::OBSCURE {
					subject,
				} =>
			{
				let space = self.at_mut(subject.position);

				space.vision.remove(Player::SELF);
			}

			& Change::VISION {
					subject,
					ref vision,
				} =>
			{
				let space = self.at_mut(subject.position);

				space.vision = (*vision).clone();
			}

			_ => unimplemented!(),
		}
	}
}
