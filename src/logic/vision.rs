/* Order */

use crate::logic::player::PlayerColor;

#[derive(Default, Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub struct Vision(Vec<PlayerColor>);

impl Vision
{
	pub fn and(mut self, player: PlayerColor) -> Vision
	{
		if !self.0.contains(&player)
		{
			self.0.push(player);
		}
		self
	}

	pub fn add(&mut self, player: PlayerColor) -> &mut Vision
	{
		if !self.0.contains(&player)
		{
			self.0.push(player);
		}
		self
	}

	pub fn remove(&mut self, player: PlayerColor) -> &mut Vision
	{
		self.0.retain(|p| p != &player);
		self
	}

	pub fn contains(&self, player: PlayerColor) -> bool
	{
		self.0.contains(&player)
	}
}
