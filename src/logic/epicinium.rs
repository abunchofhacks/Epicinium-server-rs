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

#[link(name = "epicinium", kind = "static")]
extern "C" {
	fn epicinium_map_pool_size() -> usize;
	fn epicinium_map_pool_get(i: usize) -> *const c_char;

	fn epicinium_current_challenge_id() -> u16;
	fn epicinium_challenge_key(i: u16) -> *const c_char;
	fn epicinium_challenge_display_name(i: u16) -> *const c_char;
	fn epicinium_challenge_panel_picture_name(i: u16) -> *const c_char;
	fn epicinium_challenge_discord_image_key(i: u16) -> *const c_char;
}
