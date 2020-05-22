/* Challenge */

use crate::logic::epicinium;
use crate::logic::epicinium::ChallengeId;

#[derive(Debug)]
pub struct Challenge
{
	pub key: String,
	pub metadata: Metadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct Metadata
{
	display_name: String,
	panel_picture_name: String,
	discord_image_key: String,
}

pub fn get_current_key() -> String
{
	epicinium::challenge_key(epicinium::current_challenge_id())
}

pub fn load_current() -> Challenge
{
	let id = epicinium::current_challenge_id();
	let key = epicinium::challenge_key(id);
	let display_name = epicinium::challenge_display_name(id);
	let panel_picture_name = epicinium::challenge_panel_picture_name(id);
	let discord_image_key = epicinium::challenge_discord_image_key(id);

	Challenge {
		key,
		metadata: Metadata {
			display_name,
			panel_picture_name,
			discord_image_key,
		},
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionBriefing(serde_json::Value);

pub fn load_briefing(id: ChallengeId) -> MissionBriefing
{
	MissionBriefing(epicinium::challenge_mission_briefing(id))
}
