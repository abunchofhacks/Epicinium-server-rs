/* Challenge */

use crate::logic::epicinium;

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

pub fn load_current() -> Challenge
{
	let key = epicinium::current_challenge_key();
	let display_name = epicinium::current_challenge_display_name();
	let panel_picture_name = epicinium::current_challenge_panel_picture_name();
	let discord_image_key = epicinium::current_challenge_discord_image_key();

	Challenge {
		key,
		metadata: Metadata {
			display_name,
			panel_picture_name,
			discord_image_key,
		},
	}
}
