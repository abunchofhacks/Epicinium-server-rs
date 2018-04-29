/* Unit */

use logic::header::*;
use logic::player::Player;


#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug)]
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
	ZEPPELIN,
	GLIDER,
	NUKE,
}

impl Default for UnitType
{
	fn default() -> UnitType { UnitType::NONE }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UnitToken
{
	#[serde(rename = "type")]
	pub typ: UnitType,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub owner: Player,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub stacks: i8,
}

impl Default for UnitToken
{
	fn default() -> UnitToken
	{
		UnitToken {
			typ: UnitType::NONE,
			owner: Player::NONE,
			stacks: 0,
		}
	}
}
