/* Unit */

use crate::common::header::*;
use crate::logic::player::PlayerColor;

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub struct UnitType(Option<String>);

impl Default for UnitType
{
	fn default() -> UnitType
	{
		UnitType(None)
	}
}

#[derive(Default, Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct UnitToken
{
	#[serde(rename = "type")]
	pub typ: UnitType,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub owner: PlayerColor,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub stacks: i8,
}
