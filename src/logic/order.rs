/* Order */

use logic::descriptor::Descriptor;
use logic::tile::TileType;
use logic::unit::UnitType;


#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Move
{
	EAST,
	SOUTH,
	WEST,
	NORTH,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum Order
{
	NONE
	{},
	MOVE
	{
		subject: Descriptor,
		target: Descriptor,
		moves: Vec<Move>,
	},
	GUARD
	{
		subject: Descriptor,
		target: Descriptor,
	},
	FOCUS
	{
		subject: Descriptor,
		target: Descriptor,
	},
	SHELL
	{
		subject: Descriptor,
		target: Descriptor,
	},
	BOMBARD
	{
		subject: Descriptor,
		target: Descriptor,
	},
	BOMB
	{
		subject: Descriptor,
	},
	CAPTURE
	{
		subject: Descriptor,
	},
	SHAPE
	{
		subject: Descriptor,
		tiletype: TileType,
	},
	SETTLE
	{
		subject: Descriptor,
		tiletype: TileType,
	},
	EXPAND
	{
		subject: Descriptor,
		target: Descriptor,
		tiletype: TileType,
	},
	UPGRADE
	{
		subject: Descriptor,
		tiletype: TileType,
	},
	CULTIVATE
	{
		subject: Descriptor,
		tiletype: TileType,
	},
	PRODUCE
	{
		subject: Descriptor,
		tiletype: UnitType,
	},
	HALT
	{
		subject: Descriptor,
	},
}

impl Default for Order
{
	fn default() -> Order { Order::NONE {} }
}
