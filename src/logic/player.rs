/* Player */

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug, Enum)]
#[serde(rename_all = "lowercase")]
pub enum PlayerColor
{
	/* No player. */
	None = 0,
	/* Player colors. */
	Red,
	Blue,
	Yellow,
	Teal,
	Black,
	Pink,
	Indigo,
	Purple,
	/* Non-player vision types used by the Automaton. */
	Blind,
	Observer,
}

pub const PLAYER_MAX: usize = 8;

impl Default for PlayerColor
{
	fn default() -> PlayerColor
	{
		PlayerColor::None
	}
}
