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
