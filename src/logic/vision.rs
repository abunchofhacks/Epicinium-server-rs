/* Order */

use crate::logic::player::PlayerColor;

use serde_derive::{Deserialize, Serialize};

#[derive(Default, Clone, Serialize, Deserialize, Debug)]
pub struct Vision(Vec<PlayerColor>);

impl Vision
{
	pub fn contains(&self, player: PlayerColor) -> bool
	{
		self.0.contains(&player)
	}
}
