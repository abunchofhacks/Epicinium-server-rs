/* Space */

use logic::position::Position;
use logic::vision::Vision;
use logic::unit::*;
use logic::tile::*;


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
