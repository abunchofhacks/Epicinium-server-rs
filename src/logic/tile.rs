/* Tile */

use logic::header::*;
use logic::player::Player;


#[derive(PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum TileType
{
	NONE = 0,
	GRASS,
	DIRT,
	DESERT,
	STONE,
	RUBBLE,
	RIDGE,
	MOUNTAIN,
	WATER,
	FOREST,
	CITY,
	TOWN,
	SETTLEMENT,
	INDUSTRY,
	EMBASSY,
	BARRACKS,
	AIRFIELD,
	REACTOR,
	FARM,
	SOIL,
	CROPS,
	TRENCHES,
}

impl Default for TileType
{
	fn default() -> TileType { TileType::NONE }
}

#[derive(Default, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct TileToken
{
	#[serde(rename = "type")]
	pub typ: TileType,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub owner: Player,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub stacks: i8,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub power: i8,
}
