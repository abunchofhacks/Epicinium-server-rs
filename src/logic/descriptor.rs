/* Descriptor */

use logic::position::*;


#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Type
{
	NONE = 0,
	CELL,
	TILE,
	GROUND,
	AIR,
	BYPASS,
}

impl Default for Type
{
	fn default() -> Type { Type::NONE }
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Descriptor
{
	#[serde(rename = "type")]
	pub typ: Type,

	pub position: Position,
}
