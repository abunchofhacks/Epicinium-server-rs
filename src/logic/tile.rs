/* Tile */

use crate::common::header::*;
use crate::logic::player::PlayerColor;

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub struct TileType(Option<String>);

impl Default for TileType
{
	fn default() -> TileType
	{
		TileType(None)
	}
}

#[derive(Default, Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct TileToken
{
	#[serde(rename = "type")]
	pub typ: TileType,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub owner: PlayerColor,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub stacks: i8,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub power: i8,
}
