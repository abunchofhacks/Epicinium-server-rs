/* Epicinium-as-a-library */

use crate::logic::ai;
use crate::logic::automaton;
use crate::logic::change::{Change, ChangeSet};
use crate::logic::order::Order;
use crate::logic::ruleset::InitializationError;

use log::*;

use libc::c_char;
use std::ffi::{CStr, CString};

pub fn allocate_automaton(
	players: Vec<PlayerColor>,
	ruleset_name: &str,
) -> Result<AllocatedAutomaton, InterfaceError>
{
	let _scoped = LogGuard::create();
	let playercount = players.len();
	let ruleset_name: CString = CString::new(ruleset_name)?;
	let ptr = unsafe {
		epicinium_automaton_allocate(playercount, ruleset_name.as_ptr())
	};
	if ptr == std::ptr::null_mut()
	{
		return Err(InterfaceError::AllocationFailed);
	}
	for player in players
	{
		let player: u8 = unsafe { std::mem::transmute(player) };
		unsafe {
			epicinium_automaton_add_player(ptr, player);
		}
	}
	let buffer = unsafe { epicinium_buffer_allocate() };
	if buffer == std::ptr::null_mut()
	{
		return Err(InterfaceError::AllocationFailed);
	}
	Ok(AllocatedAutomaton { ptr, buffer })
}

pub fn automaton_grant_global_vision(
	automaton: &mut AllocatedAutomaton,
	player: PlayerColor,
)
{
	let _scoped = LogGuard::create();
	let player: u8 = unsafe { std::mem::transmute(player) };
	unsafe { epicinium_automaton_grant_global_vision(automaton.ptr, player) }
}

pub fn automaton_load_map(
	automaton: &mut AllocatedAutomaton,
	map_name: String,
	shuffleplayers: bool,
) -> Result<(), InterfaceError>
{
	let _scoped = LogGuard::create();
	let map_name: CString = CString::new(map_name)?;
	unsafe {
		epicinium_automaton_load_map(
			automaton.ptr,
			map_name.as_ptr(),
			shuffleplayers,
		)
	}
	Ok(())
}

pub fn automaton_restore(
	automaton: &mut AllocatedAutomaton,
	recording_name: String,
) -> Result<(), InterfaceError>
{
	let _scoped = LogGuard::create();
	let recording_name: CString = CString::new(recording_name)?;
	unsafe {
		epicinium_automaton_restore(automaton.ptr, recording_name.as_ptr())
	}
	Ok(())
}

pub fn automaton_load_replay(
	automaton: &mut AllocatedAutomaton,
	recording_name: String,
) -> Result<(), InterfaceError>
{
	let _scoped = LogGuard::create();
	let recording_name: CString = CString::new(recording_name)?;
	unsafe {
		epicinium_automaton_load_replay(automaton.ptr, recording_name.as_ptr())
	}
	Ok(())
}

pub fn automaton_start_recording(
	automaton: &mut AllocatedAutomaton,
	metadata: automaton::Metadata,
	recording_name: String,
) -> Result<(), InterfaceError>
{
	let _scoped = LogGuard::create();
	let metadata: String = serde_json::to_string(&metadata)?;
	let metadata: CString = CString::new(metadata)?;
	let recording_name: CString = CString::new(recording_name)?;
	unsafe {
		epicinium_automaton_start_recording(
			automaton.ptr,
			metadata.as_ptr(),
			recording_name.as_ptr(),
		)
	}
	Ok(())
}

pub fn automaton_set_challenge(
	automaton: &mut AllocatedAutomaton,
	challenge_id: ChallengeId,
) -> Result<(), InterfaceError>
{
	let _scoped = LogGuard::create();
	unsafe { epicinium_automaton_set_challenge(automaton.ptr, challenge_id.0) }
	Ok(())
}

pub fn automaton_is_active(automaton: &mut AllocatedAutomaton) -> bool
{
	let _scoped = LogGuard::create();
	unsafe { epicinium_automaton_is_active(automaton.ptr) }
}

pub fn automaton_is_replay_active(automaton: &mut AllocatedAutomaton) -> bool
{
	let _scoped = LogGuard::create();
	unsafe { epicinium_automaton_is_replay_active(automaton.ptr) }
}

pub fn automaton_act(
	automaton: &mut AllocatedAutomaton,
) -> Result<ChangeSet, InterfaceError>
{
	let _scoped = LogGuard::create();
	let s: &CStr = unsafe {
		CStr::from_ptr(epicinium_automaton_act(automaton.ptr, automaton.buffer))
	};
	let jsonstr: String = s.to_string_lossy().to_string();
	let cset: ChangeSet = serde_json::from_str(&jsonstr)?;
	Ok(cset)
}

pub fn automaton_is_gameover(automaton: &mut AllocatedAutomaton) -> bool
{
	let _scoped = LogGuard::create();
	unsafe { epicinium_automaton_is_gameover(automaton.ptr) }
}

pub fn automaton_is_defeated(
	automaton: &mut AllocatedAutomaton,
	player: PlayerColor,
) -> bool
{
	let _scoped = LogGuard::create();
	let player: u8 = unsafe { std::mem::transmute(player) };
	unsafe { epicinium_automaton_is_defeated(automaton.ptr, player) }
}

pub fn automaton_current_round(automaton: &mut AllocatedAutomaton) -> u32
{
	let _scoped = LogGuard::create();
	unsafe { epicinium_automaton_current_round(automaton.ptr) }
}

pub fn automaton_global_score(automaton: &mut AllocatedAutomaton) -> i32
{
	let _scoped = LogGuard::create();
	unsafe { epicinium_automaton_global_score(automaton.ptr) }
}

pub fn automaton_score(
	automaton: &mut AllocatedAutomaton,
	player: PlayerColor,
) -> i32
{
	let _scoped = LogGuard::create();
	let player: u8 = unsafe { std::mem::transmute(player) };
	unsafe { epicinium_automaton_score(automaton.ptr, player) }
}

pub fn automaton_award(
	automaton: &mut AllocatedAutomaton,
	player: PlayerColor,
) -> i32
{
	let _scoped = LogGuard::create();
	let player: u8 = unsafe { std::mem::transmute(player) };
	unsafe { epicinium_automaton_award(automaton.ptr, player) }
}

pub fn automaton_resign(automaton: &mut AllocatedAutomaton, player: PlayerColor)
{
	let _scoped = LogGuard::create();
	let player: u8 = unsafe { std::mem::transmute(player) };
	unsafe { epicinium_automaton_resign(automaton.ptr, player) }
}

pub fn automaton_hibernate(
	automaton: &mut AllocatedAutomaton,
) -> Result<ChangeSet, InterfaceError>
{
	let _scoped = LogGuard::create();
	let s: &CStr = unsafe {
		CStr::from_ptr(epicinium_automaton_hibernate(
			automaton.ptr,
			automaton.buffer,
		))
	};
	let jsonstr: String = s.to_string_lossy().to_string();
	let cset: ChangeSet = serde_json::from_str(&jsonstr)?;
	Ok(cset)
}

pub fn automaton_awake(
	automaton: &mut AllocatedAutomaton,
) -> Result<ChangeSet, InterfaceError>
{
	let _scoped = LogGuard::create();
	let s: &CStr = unsafe {
		CStr::from_ptr(epicinium_automaton_awake(
			automaton.ptr,
			automaton.buffer,
		))
	};
	let jsonstr: String = s.to_string_lossy().to_string();
	let cset: ChangeSet = serde_json::from_str(&jsonstr)?;
	Ok(cset)
}

pub fn automaton_receive(
	automaton: &mut AllocatedAutomaton,
	player: PlayerColor,
	orders: Vec<Order>,
) -> Result<(), InterfaceError>
{
	let _scoped = LogGuard::create();
	let player: u8 = unsafe { std::mem::transmute(player) };
	let orders: String = serde_json::to_string(&orders)?;
	let orders: CString = CString::new(orders)?;
	unsafe {
		epicinium_automaton_receive(automaton.ptr, player, orders.as_ptr());
	}
	Ok(())
}

pub fn automaton_prepare(
	automaton: &mut AllocatedAutomaton,
) -> Result<ChangeSet, InterfaceError>
{
	let _scoped = LogGuard::create();
	let s: &CStr = unsafe {
		CStr::from_ptr(epicinium_automaton_prepare(
			automaton.ptr,
			automaton.buffer,
		))
	};
	let jsonstr: String = s.to_string_lossy().to_string();
	let cset: ChangeSet = serde_json::from_str(&jsonstr)?;
	Ok(cset)
}

pub fn automaton_rejoin(
	automaton: &mut AllocatedAutomaton,
	player: PlayerColor,
) -> Result<ChangeSet, InterfaceError>
{
	let _scoped = LogGuard::create();
	let player: u8 = unsafe { std::mem::transmute(player) };
	let s: &CStr = unsafe {
		CStr::from_ptr(epicinium_automaton_rejoin(
			automaton.ptr,
			player,
			automaton.buffer,
		))
	};
	let jsonstr: String = s.to_string_lossy().to_string();
	let cset: ChangeSet = serde_json::from_str(&jsonstr)?;
	Ok(cset)
}

pub fn map_pool() -> Vec<String>
{
	let _scoped = LogGuard::create();
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
	let _scoped = LogGuard::create();
	let success = unsafe { epicinium_ruleset_initialize_collection() };
	if success
	{
		Ok(())
	}
	else
	{
		Err(InitializationError::Failed)
	}
}

pub fn name_current_ruleset() -> String
{
	let _scoped = LogGuard::create();
	let s: &CStr = unsafe { CStr::from_ptr(epicinium_ruleset_current_name()) };
	s.to_string_lossy().to_string()
}

pub fn ruleset_exists(name: &str) -> bool
{
	let _scoped = LogGuard::create();
	let name = match CString::new(name)
	{
		Ok(name) => name,
		Err(error) =>
		{
			error!("Ruleset with nul character: {}, {:?}", name, error);
			return false;
		}
	};
	unsafe { epicinium_ruleset_exists(name.as_ptr()) }
}

pub fn ai_pool() -> Vec<String>
{
	let _scoped = LogGuard::create();
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
	let _scoped = LogGuard::create();
	let name = match CString::new(name)
	{
		Ok(name) => name,
		Err(error) =>
		{
			error!("AI with nul character: {}, {:?}", name, error);
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
) -> Result<AllocatedAi, InterfaceError>
{
	let _scoped = LogGuard::create();
	let ai_name: CString = CString::new(name)?;
	let player: u8 = unsafe { std::mem::transmute(player) };
	let difficulty: u8 = unsafe { std::mem::transmute(difficulty) };
	let ruleset_name: CString = CString::new(ruleset_name)?;
	let character: c_char = unsafe { std::mem::transmute(character) };
	let ptr = unsafe {
		epicinium_ai_allocate(
			ai_name.as_ptr(),
			player,
			difficulty,
			ruleset_name.as_ptr(),
			character,
		)
	};
	if ptr == std::ptr::null_mut()
	{
		return Err(InterfaceError::AllocationFailed);
	}
	let buffer = unsafe { epicinium_buffer_allocate() };
	if buffer == std::ptr::null_mut()
	{
		return Err(InterfaceError::AllocationFailed);
	}
	Ok(AllocatedAi { ptr, buffer })
}

pub fn ai_receive(
	ai: &mut AllocatedAi,
	changes: Vec<Change>,
) -> Result<(), InterfaceError>
{
	let _scoped = LogGuard::create();
	let changes: String = serde_json::to_string(&changes)?;
	let changes: CString = CString::new(changes)?;
	unsafe {
		epicinium_ai_receive(ai.ptr, changes.as_ptr());
	}
	Ok(())
}

pub fn ai_prepare_orders(ai: &mut AllocatedAi)
{
	let _scoped = LogGuard::create();
	unsafe { epicinium_ai_prepare_orders(ai.ptr) }
}

pub fn ai_retrieve_orders(
	ai: &mut AllocatedAi,
) -> Result<Vec<Order>, InterfaceError>
{
	let _scoped = LogGuard::create();
	let s: &CStr = unsafe {
		CStr::from_ptr(epicinium_ai_retrieve_orders(ai.ptr, ai.buffer)) //
	};
	let jsonstr: String = s.to_string_lossy().to_string();
	let orders: Vec<Order> = serde_json::from_str(&jsonstr)?;
	Ok(orders)
}

pub fn ai_descriptive_name(
	ai: &mut AllocatedAi,
) -> Result<String, InterfaceError>
{
	let _scoped = LogGuard::create();
	let s: &CStr = unsafe {
		CStr::from_ptr(epicinium_ai_descriptive_name(ai.ptr, ai.buffer)) //
	};
	let s: &str = s.to_str()?;
	Ok(s.to_string())
}

pub fn ai_descriptive_metadata(
	ai: &mut AllocatedAi,
) -> Result<ai::Metadata, InterfaceError>
{
	let _scoped = LogGuard::create();
	let s: &CStr = unsafe {
		CStr::from_ptr(epicinium_ai_descriptive_metadata(ai.ptr, ai.buffer)) //
	};
	let jsonstr: String = s.to_string_lossy().to_string();
	let metadata: ai::Metadata = serde_json::from_str(&jsonstr)?;
	Ok(metadata)
}

#[derive(Debug)]
pub enum InterfaceError
{
	AllocationFailed,
	ArgumentNulError(std::ffi::NulError),
	Json(serde_json::Error),
	Utf8(std::str::Utf8Error),
}

impl From<std::ffi::NulError> for InterfaceError
{
	fn from(error: std::ffi::NulError) -> Self
	{
		InterfaceError::ArgumentNulError(error)
	}
}

impl From<serde_json::Error> for InterfaceError
{
	fn from(error: serde_json::Error) -> Self
	{
		InterfaceError::Json(error)
	}
}

impl From<std::str::Utf8Error> for InterfaceError
{
	fn from(error: std::str::Utf8Error) -> Self
	{
		InterfaceError::Utf8(error)
	}
}

impl std::fmt::Display for InterfaceError
{
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
	{
		match self
		{
			InterfaceError::AllocationFailed => write!(f, "allocation failed"),
			InterfaceError::ArgumentNulError(error) => error.fmt(f),
			InterfaceError::Json(error) => error.fmt(f),
			InterfaceError::Utf8(error) => error.fmt(f),
		}
	}
}

impl std::error::Error for InterfaceError {}

#[derive(
	Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord,
)]
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
	SelfInternalDoNotUse,
}

#[derive(Debug, Clone, Copy, PartialEq)]
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
	let _scoped = LogGuard::create();
	let id = unsafe { epicinium_current_challenge_id() };
	ChallengeId(id)
}

pub fn challenge_key(id: ChallengeId) -> String
{
	let _scoped = LogGuard::create();
	let s: &CStr = unsafe {
		CStr::from_ptr(epicinium_challenge_key(id.0)) //
	};
	s.to_string_lossy().to_string()
}

pub fn challenge_num_bots(id: ChallengeId) -> usize
{
	let _scoped = LogGuard::create();
	unsafe { epicinium_challenge_num_bots(id.0) }
}

pub fn challenge_bot_name(id: ChallengeId) -> String
{
	let _scoped = LogGuard::create();
	let s: &CStr = unsafe {
		CStr::from_ptr(epicinium_challenge_bot_name(id.0)) //
	};
	s.to_string_lossy().to_string()
}

pub fn challenge_bot_difficulty(id: ChallengeId) -> Difficulty
{
	let _scoped = LogGuard::create();
	unsafe { std::mem::transmute(epicinium_challenge_bot_difficulty(id.0)) }
}

pub fn challenge_map_name(id: ChallengeId) -> String
{
	let _scoped = LogGuard::create();
	let s: &CStr = unsafe {
		CStr::from_ptr(epicinium_challenge_map_name(id.0)) //
	};
	s.to_string_lossy().to_string()
}

pub fn challenge_ruleset_name(id: ChallengeId) -> Option<String>
{
	let _scoped = LogGuard::create();
	let s: &CStr = unsafe {
		CStr::from_ptr(epicinium_challenge_ruleset_name(id.0)) //
	};
	Some(s.to_string_lossy().to_string()).filter(|s| !s.is_empty())
}

pub fn challenge_display_name(id: ChallengeId) -> String
{
	let _scoped = LogGuard::create();
	let s: &CStr = unsafe {
		CStr::from_ptr(epicinium_challenge_display_name(id.0)) //
	};
	s.to_string_lossy().to_string()
}

pub fn challenge_panel_picture_name(id: ChallengeId) -> String
{
	let _scoped = LogGuard::create();
	let s: &CStr = unsafe {
		CStr::from_ptr(epicinium_challenge_panel_picture_name(id.0)) //
	};
	s.to_string_lossy().to_string()
}

pub fn challenge_discord_image_key(id: ChallengeId) -> String
{
	let _scoped = LogGuard::create();
	let s: &CStr = unsafe {
		CStr::from_ptr(epicinium_challenge_discord_image_key(id.0)) //
	};
	s.to_string_lossy().to_string()
}

pub fn challenge_mission_briefing(id: ChallengeId) -> serde_json::Value
{
	let _scoped = LogGuard::create();
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
pub struct AllocatedAutomaton
{
	ptr: *mut Automaton,
	buffer: *mut Buffer,
}

unsafe impl Send for AllocatedAutomaton {}

impl Drop for AllocatedAutomaton
{
	fn drop(&mut self)
	{
		let _scoped = LogGuard::create();
		unsafe { epicinium_automaton_deallocate(self.ptr) }
		unsafe { epicinium_buffer_deallocate(self.buffer) }
	}
}

enum Automaton {}

#[derive(Debug)]
pub struct AllocatedAi
{
	ptr: *mut AICommander,
	buffer: *mut Buffer,
}

unsafe impl Send for AllocatedAi {}

impl Drop for AllocatedAi
{
	fn drop(&mut self)
	{
		let _scoped = LogGuard::create();
		unsafe { epicinium_ai_deallocate(self.ptr) }
		unsafe { epicinium_buffer_deallocate(self.buffer) }
	}
}

enum AICommander {}

enum Buffer {}

#[repr(u8)]
enum PlogSeverity
{
	None,
	Fatal,
	Error,
	Warning,
	Info,
	Debug,
	Verbose,
}

pub fn log_initialize()
{
	// TODO loglevel
	let severity = PlogSeverity::Verbose;

	let severity: u8 = unsafe { std::mem::transmute(severity) };
	unsafe { epicinium_log_initialize(severity) };
}

extern "C" fn log_callback(severity: u8, record: *const c_char)
{
	let severity: PlogSeverity = unsafe { std::mem::transmute(severity) };
	let record: &CStr = unsafe { CStr::from_ptr(record) };
	let record = record.to_string_lossy();
	match severity
	{
		PlogSeverity::None | PlogSeverity::Fatal | PlogSeverity::Error =>
		{
			error!("{}", record)
		}
		PlogSeverity::Warning => warn!("{}", record),
		PlogSeverity::Info => info!("{}", record),
		PlogSeverity::Debug => debug!("{}", record),
		PlogSeverity::Verbose => trace!("{}", record),
	}
}

struct LogGuard {}

impl LogGuard
{
	fn create()
	{
		unsafe { epicinium_log_callback_install(log_callback) }
	}
}

impl Drop for LogGuard
{
	fn drop(&mut self)
	{
		unsafe { epicinium_log_callback_deinstall() }
	}
}

#[link(name = "epicinium", kind = "static")]
extern "C" {
	fn epicinium_automaton_allocate(
		playercount: usize,
		ruleset_name: *const c_char,
	) -> *mut Automaton;
	fn epicinium_automaton_deallocate(automaton: *mut Automaton);

	fn epicinium_automaton_add_player(automaton: *mut Automaton, player: u8);
	fn epicinium_automaton_grant_global_vision(
		automaton: *mut Automaton,
		player: u8,
	);
	fn epicinium_automaton_load_map(
		automaton: *mut Automaton,
		map_name: *const c_char,
		shuffleplayers: bool,
	);
	fn epicinium_automaton_restore(
		automaton: *mut Automaton,
		recording_name: *const c_char,
	);
	fn epicinium_automaton_load_replay(
		automaton: *mut Automaton,
		recording_name: *const c_char,
	);
	fn epicinium_automaton_start_recording(
		automaton: *mut Automaton,
		metadata: *const c_char,
		recording_name: *const c_char,
	);
	fn epicinium_automaton_set_challenge(
		automaton: *mut Automaton,
		challenge_id: u16,
	);
	fn epicinium_automaton_is_active(automaton: *mut Automaton) -> bool;
	fn epicinium_automaton_is_replay_active(automaton: *mut Automaton) -> bool;
	fn epicinium_automaton_act(
		automaton: *mut Automaton,
		buffer: *mut Buffer,
	) -> *const c_char;
	fn epicinium_automaton_is_gameover(automaton: *mut Automaton) -> bool;
	fn epicinium_automaton_is_defeated(
		automaton: *mut Automaton,
		player: u8,
	) -> bool;
	fn epicinium_automaton_current_round(automaton: *mut Automaton) -> u32;
	fn epicinium_automaton_global_score(automaton: *mut Automaton) -> i32;
	fn epicinium_automaton_score(automaton: *mut Automaton, player: u8) -> i32;
	fn epicinium_automaton_award(automaton: *mut Automaton, player: u8) -> i32;
	fn epicinium_automaton_resign(automaton: *mut Automaton, player: u8);
	fn epicinium_automaton_hibernate(
		automaton: *mut Automaton,
		buffer: *mut Buffer,
	) -> *const c_char;
	fn epicinium_automaton_awake(
		automaton: *mut Automaton,
		buffer: *mut Buffer,
	) -> *const c_char;
	fn epicinium_automaton_receive(
		automaton: *mut Automaton,
		player: u8,
		orders: *const c_char,
	);
	fn epicinium_automaton_prepare(
		automaton: *mut Automaton,
		buffer: *mut Buffer,
	) -> *const c_char;
	fn epicinium_automaton_rejoin(
		automaton: *mut Automaton,
		player: u8,
		buffer: *mut Buffer,
	) -> *const c_char;

	fn epicinium_map_pool_size() -> usize;
	fn epicinium_map_pool_get(i: usize) -> *const c_char;

	fn epicinium_ruleset_initialize_collection() -> bool;
	fn epicinium_ruleset_current_name() -> *const c_char;
	fn epicinium_ruleset_exists(name: *const c_char) -> bool;

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

	fn epicinium_ai_receive(ai: *mut AICommander, changes: *const c_char);
	fn epicinium_ai_prepare_orders(ai: *mut AICommander);
	fn epicinium_ai_retrieve_orders(
		ai: *mut AICommander,
		buffer: *mut Buffer,
	) -> *const c_char;
	fn epicinium_ai_descriptive_name(
		ai: *mut AICommander,
		buffer: *mut Buffer,
	) -> *const c_char;
	fn epicinium_ai_descriptive_metadata(
		ai: *mut AICommander,
		buffer: *mut Buffer,
	) -> *const c_char;

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

	fn epicinium_buffer_allocate() -> *mut Buffer;
	fn epicinium_buffer_deallocate(tmp: *mut Buffer);

	fn epicinium_log_initialize(severity: u8);
	fn epicinium_log_callback_install(
		callback: extern "C" fn(u8, *const c_char),
	);
	fn epicinium_log_callback_deinstall();
}
