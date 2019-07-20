/* Order */

use logic::player::Player;


#[derive(Default, Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub struct Vision(Vec<Player>);
// TODO replace with a bit_set::BitSet once the serde pull request is merged

impl Vision
{
	pub fn and(mut self, player : Player) -> Vision
	{
		if !self.0.contains(&player)
		{
			self.0.push(player);
		}
		self
	}

	pub fn add(&mut self, player : Player) -> &mut Vision
	{
		if !self.0.contains(&player)
		{
			self.0.push(player);
		}
		self
	}

	pub fn remove(&mut self, player : Player) -> &mut Vision
	{
		self.0.retain(|p| p != &player);
		self
	}

	pub fn contains(& self, player : Player) -> bool
	{
		self.0.contains(&player)
	}
}
