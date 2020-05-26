/* Epicinium-as-a-library */

use libc::c_char;
use std::ffi::CStr;

pub fn map_pool() -> Vec<String>
{
	let len = unsafe { epicinium_map_pool_size() };
	let mut pool = Vec::with_capacity(len);
	for i in 0..len
	{
		let s: &CStr = unsafe { CStr::from_ptr(epicinium_map_pool_get(i)) };
		pool.push(s.to_string_lossy().to_string());
	}
	pool
}

#[derive(Debug, Clone, Copy)]
pub struct ChallengeId(u16);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[repr(u8)]
pub enum Difficulty
{
	None,
	Easy,
	Medium,
	Hard,
}

pub fn current_challenge_id() -> ChallengeId
{
	let id = unsafe { epicinium_current_challenge_id() };
	ChallengeId(id)
}

pub fn challenge_key(id: ChallengeId) -> String
{
	let s: &CStr = unsafe {
		CStr::from_ptr(epicinium_challenge_key(id.0)) //
	};
	s.to_string_lossy().to_string()
}

pub fn challenge_num_bots(id: ChallengeId) -> usize
{
	unsafe { epicinium_challenge_num_bots(id.0) }
}

pub fn challenge_bot_name(id: ChallengeId) -> String
{
	let s: &CStr = unsafe {
		CStr::from_ptr(epicinium_challenge_bot_name(id.0)) //
	};
	s.to_string_lossy().to_string()
}

pub fn challenge_bot_difficulty(id: ChallengeId) -> Difficulty
{
	unsafe { std::mem::transmute(epicinium_challenge_bot_difficulty(id.0)) }
}

pub fn challenge_map_name(id: ChallengeId) -> String
{
	let s: &CStr = unsafe {
		CStr::from_ptr(epicinium_challenge_map_name(id.0)) //
	};
	s.to_string_lossy().to_string()
}

pub fn challenge_ruleset_name(id: ChallengeId) -> Option<String>
{
	let s: &CStr = unsafe {
		CStr::from_ptr(epicinium_challenge_ruleset_name(id.0)) //
	};
	Some(s.to_string_lossy().to_string()).filter(|s| !s.is_empty())
}

pub fn challenge_display_name(id: ChallengeId) -> String
{
	let s: &CStr = unsafe {
		CStr::from_ptr(epicinium_challenge_display_name(id.0)) //
	};
	s.to_string_lossy().to_string()
}

pub fn challenge_panel_picture_name(id: ChallengeId) -> String
{
	let s: &CStr = unsafe {
		CStr::from_ptr(epicinium_challenge_panel_picture_name(id.0)) //
	};
	s.to_string_lossy().to_string()
}

pub fn challenge_discord_image_key(id: ChallengeId) -> String
{
	let s: &CStr = unsafe {
		CStr::from_ptr(epicinium_challenge_discord_image_key(id.0)) //
	};
	s.to_string_lossy().to_string()
}

pub fn challenge_mission_briefing(id: ChallengeId) -> serde_json::Value
{
	let num = unsafe { epicinium_challenge_briefing_size(id.0) };
	let mut data = serde_json::Map::with_capacity(num);
	for i in 0..num
	{
		let key: &CStr = unsafe {
			CStr::from_ptr(epicinium_challenge_briefing_key(id.0, i)) //
		};
		let key = key.to_string_lossy().to_string();
		let value: &CStr = unsafe {
			CStr::from_ptr(epicinium_challenge_briefing_value(id.0, i)) //
		};
		let value = value.to_string_lossy().to_string();
		data.insert(key, serde_json::Value::String(value));
	}
	serde_json::Value::Object(data)
}

#[link(name = "epicinium", kind = "static")]
extern "C" {
	fn epicinium_map_pool_size() -> usize;
	fn epicinium_map_pool_get(i: usize) -> *const c_char;

	fn epicinium_current_challenge_id() -> u16;
	fn epicinium_challenge_key(id: u16) -> *const c_char;
	fn epicinium_challenge_num_bots(id: u16) -> usize;
	fn epicinium_challenge_bot_name(id: u16) -> *const c_char;
	fn epicinium_challenge_bot_difficulty(id: u16) -> u8;
	fn epicinium_challenge_map_name(id: u16) -> *const c_char;
	fn epicinium_challenge_ruleset_name(id: u16) -> *const c_char;
	fn epicinium_challenge_display_name(id: u16) -> *const c_char;
	fn epicinium_challenge_panel_picture_name(id: u16) -> *const c_char;
	fn epicinium_challenge_discord_image_key(id: u16) -> *const c_char;
	fn epicinium_challenge_briefing_size(id: u16) -> usize;
	fn epicinium_challenge_briefing_key(id: u16, i: usize) -> *const c_char;
	fn epicinium_challenge_briefing_value(id: u16, i: usize) -> *const c_char;
}
