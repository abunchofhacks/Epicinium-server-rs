/* Player */

pub use crate::logic::epicinium::PlayerColor;

pub const PLAYER_MAX: usize = 8;

impl Default for PlayerColor
{
	fn default() -> PlayerColor
	{
		PlayerColor::None
	}
}

pub fn color_pool() -> Vec<PlayerColor>
{
	vec![
		PlayerColor::Red,
		PlayerColor::Blue,
		PlayerColor::Yellow,
		PlayerColor::Teal,
		PlayerColor::Black,
		PlayerColor::Pink,
		PlayerColor::Indigo,
		PlayerColor::Purple,
	]
}
