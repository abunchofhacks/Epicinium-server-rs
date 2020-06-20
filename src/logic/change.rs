/* Change */

use crate::logic::player::*;
use crate::logic::vision::*;

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct Change(serde_json::Value);

#[derive(Serialize, Deserialize, Debug)]
pub struct ChangeSet(Vec<ChangeSetItem>);

#[derive(Serialize, Deserialize, Debug)]
struct ChangeSetItem
{
	change: Change,
	vision: Vision,
}

impl ChangeSet
{
	pub fn get(&self, player: PlayerColor) -> Vec<Change>
	{
		self.0
			.iter()
			.filter(|&item| item.vision.contains(player))
			.map(|item| item.change.clone())
			.collect()
	}
}
