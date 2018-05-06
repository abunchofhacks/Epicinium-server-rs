/* Space */

use logic::position::Position;
use logic::vision::Vision;
use logic::unit::*;
use logic::tile::*;
use logic::descriptor::*;


#[derive(Default, Debug)]
pub struct Space
{
	pub vision : Vision,

	pub position : Position,
	pub index : i16,

	pub temperature : i8,
	pub humidity : i8,
	pub chaos : i8,
	pub gas : i8,
	pub radiation : i8,

	pub snow : bool,
	pub frostbite : bool,
	pub firestorm : bool,
	pub bonedrought : bool,
	pub death : bool,

	pub tile : TileToken,
	pub ground : UnitToken,
	pub air : UnitToken,
	pub bypass : UnitToken,
}

impl Space
{
	pub fn unit(& self, typ : Type) -> UnitToken
	{
		match typ
		{
			Type::NONE |
			Type::CELL |
			Type::TILE =>
			{
				debug_assert!(false);
				self.bypass
			},

			Type::GROUND => self.ground,
			Type::AIR => self.air,
			Type::BYPASS => self.bypass,
		}
	}

	pub fn unit_mut(&mut self, typ : Type) -> &mut UnitToken
	{
		match typ
		{
			Type::NONE |
			Type::CELL |
			Type::TILE =>
			{
				debug_assert!(false);
				&mut self.bypass
			},

			Type::GROUND => &mut self.ground,
			Type::AIR => &mut self.air,
			Type::BYPASS => &mut self.bypass,
		}
	}
}
