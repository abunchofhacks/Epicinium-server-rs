/* Player */


#[derive(PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Player
{
	/* No player. */
	NONE = 0,
	/* Player colors. */
	RED,
	BLUE,
	YELLOW,
	TEAL,
	BLACK,
	PINK,
	INDIGO,
	PURPLE,
	/* Non-player vision types used by the Automaton. */
	BLIND,
	OBSERVER,
	/* Non-player vision type used by the Board/Level. */
	SELF,
}

impl Default for Player
{
	fn default() -> Player { Player::NONE }
}
