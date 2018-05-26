/* Unit */

use std;
use common::header::*;
use logic::player::Player;


#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
#[derive(EnumMap)]
#[serde(rename_all = "lowercase")]
pub enum UnitType
{
	NONE = 0,
	RIFLEMAN,
	GUNNER,
	SAPPER,
	TANK,
	SETTLER,
	DIPLOMAT,
	MILITIA,
	ZEPPELIN,
	GLIDER,
	NUKE,
}

impl Default for UnitType
{
	fn default() -> UnitType { UnitType::NONE }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct UnitToken
{
	#[serde(rename = "type")]
	pub typ: UnitType,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub owner: Player,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub stacks: i8,
}

pub fn swap(a : &mut UnitToken, b : &mut UnitToken)
{
	std::mem::swap(a, b);
}
