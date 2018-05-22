/* Board */

use std;
use logic::space::*;
use logic::change::*;
use logic::position::*;
use logic::unit::*;
use logic::player::*;
use logic::descriptor::*;


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
			},

			& Change::VISION {
					subject,
					ref vision,
				} =>
			{
				let space = self.at_mut(subject.position);

				space.vision = (*vision).clone();
			},

			& Change::TRANSFORMED {
					subject,
					tile,
				} |
			& Change::CONSUMED {
					subject,
					tile,
				} |
			& Change::SHAPED {
					subject,
					tile,
				} |
			& Change::SETTLED {
					subject,
					tile,
				} |
			& Change::EXPANDED {
					subject,
					tile,
				} |
			& Change::UPGRADED {
					subject,
					tile,
				} |
			& Change::CULTIVATED {
					subject,
					tile,
				} =>
			{
				let space = self.at_mut(subject.position);

				space.tile = tile;
			},

			& Change::CAPTURED {
					subject,
					player
				} =>
			{
				let space = self.at_mut(subject.position);

				space.tile.owner = player;
			},

			& Change::CAPTURES {..} => {},
			& Change::SHAPES {..} => {},

			& Change::EXPANDS {
					subject,
					power,
				} |
			& Change::UPGRADES {
					subject,
					power,
				} |
			& Change::CULTIVATES {
					subject,
					power,
				} |
			& Change::PRODUCES {
					subject,
					power,
				} =>
			{
				let space = self.at_mut(subject.position);

				space.tile.power += power;
			},

			& Change::PRODUCED {
					subject,
					unit
				} |
			& Change::ENTERED {
					subject,
					unit
				} =>
			{
				let space = self.at_mut(subject.position);

				*(space.unit_mut(subject.typ)) = unit;
			},

			& Change::SETTLES {
					subject,
				} |
			& Change::EXITED {
					subject,
				} |
			& Change::DIED {
					subject,
				} =>
			{
				let space = self.at_mut(subject.position);

				*(space.unit_mut(subject.typ)) = UnitToken::default();
			},

			& Change::DESTROYED {
					subject,
					tile,
				} =>
			{
				let space = self.at_mut(subject.position);

				space.tile = tile;
			},

			& Change::SURVIVED {..} => {},

			& Change::AIMS {..} => {},
			& Change::ATTACKS {..} => {},
			& Change::TRAMPLES {..} => {},
			& Change::SHELLS {..} => {},
			& Change::BOMBARDS {..} => {},
			& Change::BOMBS {..} => {},

			& Change::ATTACKED {
					subject,
					killed,
					depowered,
					..
				} |
			& Change::TRAMPLED {
					subject,
					killed,
					depowered,
					..
				} |
			& Change::SHELLED {
					subject,
					killed,
					depowered,
					..
				} |
			& Change::BOMBARDED {
					subject,
					killed,
					depowered,
					..
				} |
			& Change::BOMBED {
					subject,
					killed,
					depowered,
					..
				} |
			& Change::FROSTBITTEN {
					subject,
					killed,
					depowered,
					..
				} |
			& Change::BURNED {
					subject,
					killed,
					depowered,
					..
				} |
			& Change::GASSED {
					subject,
					killed,
					depowered,
					..
				} |
			& Change::IRRADIATED {
					subject,
					killed,
					depowered,
					..
				} =>
			{
				let space = self.at_mut(subject.position);

				match subject.typ
				{
					// This indicates that the shot missed.
					Type::CELL => {},

					// If the tile was hit, it might remove a stack
					// and or depower a powered stack.
					Type::TILE =>
					{
						if killed
						{
							space.tile.stacks -= 1;
						}
						if depowered
						{
							space.tile.power -= 1;
						}
					},

					// If a unit was hit, it might remove a stack.
					_ =>
					{
						if killed
						{
							space.unit(subject.typ).stacks -= 1;
						}
					},
				}
			},

			& Change::GROWS {
					subject,
					stacks,
					power,
				} =>
			{
				let space = self.at_mut(subject.position);

				space.tile.stacks += stacks;
				space.tile.power += power;
			},

			& Change::SNOW {
					subject,
					snow,
				} =>
			{
				let space = self.at_mut(subject.position);

				space.snow = snow;
			},

			& Change::FROSTBITE {
					subject,
					frostbite,
				} =>
			{
				let space = self.at_mut(subject.position);

				space.frostbite = frostbite;
			},

			& Change::FIRESTORM {
					subject,
					firestorm,
				} =>
			{
				let space = self.at_mut(subject.position);

				space.firestorm = firestorm;
			},

			& Change::BONEDROUGHT {
					subject,
					bonedrought,
				} =>
			{
				let space = self.at_mut(subject.position);

				space.bonedrought = bonedrought;
			},

			& Change::DEATH {
					subject,
					death,
				} =>
			{
				let space = self.at_mut(subject.position);

				space.death = death;
			},

			& Change::GAS {
					subject,
					gas,
				} =>
			{
				let space = self.at_mut(subject.position);

				space.gas = gas;
			},

			& Change::RADIATION {
					subject,
					radiation,
				} =>
			{
				let space = self.at_mut(subject.position);

				space.radiation = radiation;
			},

			& Change::TEMPERATURE {
					subject,
					temperature,
				} =>
			{
				let space = self.at_mut(subject.position);

				space.temperature = temperature;
			},

			& Change::HUMIDITY {
					subject,
					humidity,
				} =>
			{
				let space = self.at_mut(subject.position);

				space.humidity = humidity;
			},

			& Change::CHAOS {
					subject,
					chaos,
				} =>
			{
				let space = self.at_mut(subject.position);

				space.chaos = chaos;
			},

			& Change::CORNER {
					subject,
				} =>
			{
				let cols = subject.position.col + 1;
				let rows = subject.position.row + 1;

				if self.cols != cols || self.rows != rows
				{
					self.resize(cols, rows);
				}
			},

			& Change::BORDER {..} => {},
			& Change::CHAOSREPORT {..} => {},
			& Change::YEAR {..} => {},
			& Change::SEASON {..} => {},
			& Change::DAYTIME {..} => {},
			& Change::PHASE {..} => {},
			& Change::INITIATIVE {..} => {},
			& Change::FUNDS {..} => {},
			& Change::INCOME {..} => {},
			& Change::EXPENDITURE {..} => {},
			& Change::SLEEPING {..} => {},
			& Change::ACTING {..} => {},
			& Change::FINISHED {..} => {},
			& Change::DISCARDED {..} => {},
			& Change::POSTPONED {..} => {},
			& Change::UNFINISHED {..} => {},
			& Change::SCORED {..} => {},
			& Change::DEFEAT {..} => {},
			& Change::VICTORY {..} => {},
			& Change::GAMEOVER {..} => {},
		}
	}

	fn resize(&mut self, cols : i8, rows : i8)
	{
		self.cols = std::cmp::max(0, cols);
		self.rows = std::cmp::max(0, rows);

		self.cells.clear();
		self.cells.resize(cols as usize * rows as usize, Space::default());
	}
}
