/* Epicinium-as-a-library */

use crate::logic::ruleset::InitializationError;

use libc::c_char;
use std::ffi::{CStr, CString};

pub fn allocate_automaton(
	players: Vec<PlayerColor>,
	ruleset_name: &str,
) -> Result<AllocatedAutomaton, AllocationError>
{
	let playercount = players.len();
	let ruleset_name_as_cstr = CString::new(ruleset_name)?;
	let ptr = unsafe {
		epicinium_automaton_allocate(playercount, ruleset_name_as_cstr.as_ptr())
	};
	if ptr == std::ptr::null_mut()
	{
		return Err(AllocationError::AllocationFailed);
	}
	for player in players
	{
		unsafe {
			let player_as_u8: u8 = std::mem::transmute(player);
			epicinium_automaton_add_player(ptr, player_as_u8);
		}
	}
	Ok(AllocatedAutomaton(ptr))
}

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

pub fn initialize_ruleset_collection() -> Result<(), InitializationError>
{
	let success = unsafe { epicinium_initialize_ruleset_collection() };
	if success
	{
		Ok(())
	}
	else
	{
		Err(InitializationError::Failed)
	}
}

pub fn ai_pool() -> Vec<String>
{
	let len = unsafe { epicinium_ai_pool_size() };
	let mut pool = Vec::with_capacity(len);
	for i in 0..len
	{
		let s: &CStr = unsafe { CStr::from_ptr(epicinium_ai_pool_get(i)) };
		pool.push(s.to_string_lossy().to_string());
	}
	pool
}

pub fn ai_exists(name: &str) -> bool
{
	let name = match CString::new(name)
	{
		Ok(name) => name,
		Err(error) =>
		{
			eprintln!("AI with nul character: {}, {:?}", name, error);
			return false;
		}
	};
	unsafe { epicinium_ai_exists(name.as_ptr()) }
}

pub fn allocate_ai(
	name: &str,
	player: PlayerColor,
	difficulty: Difficulty,
	ruleset_name: &str,
	character: u8,
) -> Result<AllocatedAi, AllocationError>
{
	let ai_name_as_cstr = CString::new(name)?;
	let player_as_u8 = unsafe { std::mem::transmute(player) };
	let difficulty_as_u8 = unsafe { std::mem::transmute(difficulty) };
	let ruleset_name_as_cstr = CString::new(ruleset_name)?;
	let character_as_char = unsafe { std::mem::transmute(character) };
	let ptr = unsafe {
		epicinium_ai_allocate(
			ai_name_as_cstr.as_ptr(),
			player_as_u8,
			difficulty_as_u8,
			ruleset_name_as_cstr.as_ptr(),
			character_as_char,
		)
	};
	if ptr != std::ptr::null_mut()
	{
		Ok(AllocatedAi(ptr))
	}
	else
	{
		Err(AllocationError::AllocationFailed)
	}
}

#[derive(Debug)]
pub enum AllocationError
{
	AllocationFailed,
	ArgumentNulError(std::ffi::NulError),
}

impl From<std::ffi::NulError> for AllocationError
{
	fn from(error: std::ffi::NulError) -> Self
	{
		AllocationError::ArgumentNulError(error)
	}
}

impl std::fmt::Display for AllocationError
{
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
	{
		match self
		{
			AllocationError::AllocationFailed => write!(f, "allocation failed"),
			AllocationError::ArgumentNulError(error) => error.fmt(f),
		}
	}
}

impl std::error::Error for AllocationError {}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
#[repr(u8)]
pub enum PlayerColor
{
	/* No player. */
	None = 0,
	/* Player colors. */
	Red,
	Blue,
	Yellow,
	Teal,
	Black,
	Pink,
	Indigo,
	Purple,
	/* Non-player vision types used by the Automaton. */
	Blind,
	Observer,
	// Non-player vision type used by the Board/Level to keep track of its
	// owner's vision.
	SELF, // DO NOT USE
}

#[derive(Debug, Clone, Copy)]
pub struct ChallengeId(u16);

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug)]
pub struct AllocatedAutomaton(*mut Automaton);

unsafe impl Send for AllocatedAutomaton {}

impl Drop for AllocatedAutomaton
{
	fn drop(&mut self)
	{
		unsafe { epicinium_automaton_deallocate(self.0) }
	}
}

enum Automaton {}

#[derive(Debug)]
pub struct AllocatedAi(*mut AICommander);

unsafe impl Send for AllocatedAi {}

impl Drop for AllocatedAi
{
	fn drop(&mut self)
	{
		unsafe { epicinium_ai_deallocate(self.0) }
	}
}

enum AICommander {}

#[link(name = "epicinium", kind = "static")]
extern "C" {
	fn epicinium_automaton_allocate(
		playercount: usize,
		ruleset_name: *const c_char,
	) -> *mut Automaton;
	fn epicinium_automaton_add_player(automaton: *mut Automaton, player: u8);
	fn epicinium_automaton_deallocate(automaton: *mut Automaton);

	fn epicinium_map_pool_size() -> usize;
	fn epicinium_map_pool_get(i: usize) -> *const c_char;

	fn epicinium_initialize_ruleset_collection() -> bool;

	fn epicinium_ai_pool_size() -> usize;
	fn epicinium_ai_pool_get(i: usize) -> *const c_char;
	fn epicinium_ai_exists(name: *const c_char) -> bool;

	fn epicinium_ai_allocate(
		name: *const c_char,
		player: u8,
		difficulty: u8,
		ruleset_name: *const c_char,
		character: c_char,
	) -> *mut AICommander;
	fn epicinium_ai_deallocate(ai: *mut AICommander);

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
