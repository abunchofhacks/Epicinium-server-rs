/* Challenge */

pub use epicinium_lib::ChallengeId;

use crate::logic::difficulty::Difficulty;

use serde_derive::{Deserialize, Serialize};

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

pub fn current_id() -> ChallengeId
{
	epicinium_lib::current_challenge_id()
}

pub fn get_current_key() -> String
{
	epicinium_lib::challenge_key(epicinium_lib::current_challenge_id())
}

pub fn load_current() -> Challenge
{
	let id = epicinium_lib::current_challenge_id();
	let key = epicinium_lib::challenge_key(id);
	let display_name = epicinium_lib::challenge_display_name(id);
	let panel_picture_name = epicinium_lib::challenge_panel_picture_name(id);
	let discord_image_key = epicinium_lib::challenge_discord_image_key(id);

	Challenge {
		key,
		metadata: Metadata {
			display_name,
			panel_picture_name,
			discord_image_key,
		},
	}
}

pub fn num_bots(id: ChallengeId) -> usize
{
	epicinium_lib::challenge_num_bots(id)
}

pub fn bot_name(id: ChallengeId) -> String
{
	epicinium_lib::challenge_bot_name(id)
}

pub fn bot_difficulty(id: ChallengeId) -> Difficulty
{
	epicinium_lib::challenge_bot_difficulty(id)
}

pub fn map_name(id: ChallengeId) -> String
{
	epicinium_lib::challenge_map_name(id)
}

pub fn ruleset_name(id: ChallengeId) -> Option<String>
{
	epicinium_lib::challenge_ruleset_name(id)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MissionBriefing(serde_json::Value);

pub fn load_briefing(id: ChallengeId) -> MissionBriefing
{
	MissionBriefing(epicinium_lib::challenge_mission_briefing(id))
}
