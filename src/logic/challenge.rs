/*
 * Part of epicinium_server
 * developed by A Bunch of Hacks.
 *
 * Copyright (c) 2018-2021 A Bunch of Hacks
 *
 * This library is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This library is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 * [authors:]
 * Sander in 't Veld (sander@abunchofhacks.coop)
 */

pub use epicinium_lib::error::InterfaceError;
pub use epicinium_lib::ChallengeId;

use crate::logic::difficulty::Difficulty;

use serde_derive::{Deserialize, Serialize};

#[derive(Debug)]
pub struct Challenge
{
	pub id: ChallengeId,
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
	max_stars: i32,
}

pub fn load_pool() -> Result<Vec<Challenge>, InterfaceError>
{
	let mut pool = Vec::new();
	for id in epicinium_lib::challenge_pool()
	{
		let key = epicinium_lib::challenge_key(id);
		let display_name = epicinium_lib::challenge_display_name(id)?;
		let panel_picture_name =
			epicinium_lib::challenge_panel_picture_name(id);
		let discord_image_key = epicinium_lib::challenge_discord_image_key(id);

		let briefing = load_briefing(id)?;
		let max_stars = if briefing.0.get("3").is_some()
		{
			3
		}
		else if briefing.0.get("2").is_some()
		{
			2
		}
		else
		{
			1
		};

		let challenge = Challenge {
			id,
			key,
			metadata: Metadata {
				display_name,
				panel_picture_name,
				discord_image_key,
				max_stars,
			},
		};
		pool.push(challenge);
	}
	Ok(pool)
}

pub fn key(id: ChallengeId) -> String
{
	epicinium_lib::challenge_key(id)
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

pub fn load_briefing(id: ChallengeId)
	-> Result<MissionBriefing, InterfaceError>
{
	let briefing = epicinium_lib::challenge_mission_briefing(id)?;
	Ok(MissionBriefing(briefing))
}
