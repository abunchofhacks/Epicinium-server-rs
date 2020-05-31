/* Change */

use crate::logic::player::*;
use crate::logic::vision::*;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Change(serde_json::Value);

#[derive(Serialize, Deserialize, Debug)]
pub struct ChangeSet(Vec<(Change, Vision)>);

impl ChangeSet
{
	pub fn push(&mut self, change: Change, vision: Vision)
	{
		self.0.push((change, vision));
	}

	pub fn get(&self, player: PlayerColor) -> Vec<Change>
	{
		self.0
			.iter()
			.filter(|&&(_, ref vision)| vision.contains(player))
			.map(|&(ref change, _)| (*change).clone())
			.collect()
	}
}
