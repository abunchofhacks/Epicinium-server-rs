/* Unit */

use logic::player::Player;

#[allow(dead_code)]
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum UnitType
{
	NONE,
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
	#[serde(skip)]
	pub id: u32,

	#[serde(rename = "type")]
	pub typ: UnitType,

	#[serde(skip_serializing_if = "is_zero")]
	pub owner: Player,

	#[serde(skip_serializing_if = "is_zero")]
	pub stacks: i8,
}

impl Default for UnitToken
{
	fn default() -> UnitToken
	{
		UnitToken {
			id: 0,
			typ: UnitType::NONE,
			owner: Player::NONE,
			stacks: 0,
		}
	}
}

fn is_zero<'a, T> (x: &'a T) -> bool
	where T: Default, for <'b> &'b T: PartialEq<&'b T>
{
	x == &T::default()
}
