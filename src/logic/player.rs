/* Player */

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug, Enum)]
#[serde(rename_all = "lowercase")]
pub enum Player
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
	BLIND,
	OBSERVER,
	/* Non-player vision type used by the Board/Level. */
	SELF,
}

pub const PLAYER_MAX: usize = 8;

impl Default for Player
{
	fn default() -> Player
	{
		Player::None
	}
}
