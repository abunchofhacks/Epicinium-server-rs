/* Epicinium-as-a-library */

use libc::c_char;
use std::ffi::CStr;

pub fn current_challenge_key() -> String
{
	let s: &CStr = unsafe {
		CStr::from_ptr(epicinium_current_challenge_key()) //
	};
	s.to_string_lossy().to_string()
}

pub fn current_challenge_display_name() -> String
{
	let s: &CStr = unsafe {
		CStr::from_ptr(epicinium_current_challenge_display_name()) //
	};
	s.to_string_lossy().to_string()
}

pub fn current_challenge_panel_picture_name() -> String
{
	let s: &CStr = unsafe {
		CStr::from_ptr(epicinium_current_challenge_panel_picture_name()) //
	};
	s.to_string_lossy().to_string()
}

pub fn current_challenge_discord_image_key() -> String
{
	let s: &CStr = unsafe {
		CStr::from_ptr(epicinium_current_challenge_discord_image_key()) //
	};
	s.to_string_lossy().to_string()
}

#[link(name = "epiciniumlib", kind = "static")]
extern "C" {
	fn epicinium_current_challenge_key() -> *const c_char;
	fn epicinium_current_challenge_display_name() -> *const c_char;
	fn epicinium_current_challenge_panel_picture_name() -> *const c_char;
	fn epicinium_current_challenge_discord_image_key() -> *const c_char;
}
