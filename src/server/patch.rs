/* Patch */

use std::path::*;

pub fn is_requestable(filepath: &Path) -> bool
{
	(is_picture(&filepath) || is_ruleset(&filepath) || is_fzmodel(&filepath))
		&& filepath.is_relative()
}

fn is_picture(filepath: &Path) -> bool
{
	filepath.starts_with("pictures/")
		&& match filepath.extension()
		{
			Some(x) => x == "png",
			None => false,
		}
}

fn is_ruleset(filepath: &Path) -> bool
{
	filepath.starts_with("rulesets/")
		&& match filepath.extension()
		{
			Some(x) => x == "json",
			None => false,
		}
}

fn is_fzmodel(filepath: &Path) -> bool
{
	filepath.starts_with("sessions/")
		&& match filepath.extension()
		{
			Some(x) => x == "fzm",
			None => false,
		}
}
