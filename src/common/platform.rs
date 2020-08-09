/* Platform */

use serde_derive::{Deserialize, Serialize};

#[derive(PartialEq, Eq, Copy, Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Platform
{
	Unknown,
	Windows32,
	Windows64,
	Osx32,
	Osx64,
	Debian32,
	Debian64,
}

impl Default for Platform
{
	fn default() -> Platform
	{
		Platform::Unknown
	}
}

impl Platform
{
	pub fn current() -> Platform
	{
		if cfg!(target_os = "windows")
		{
			if cfg!(target_pointer_width = "64")
			{
				Platform::Windows64
			}
			else if cfg!(target_pointer_width = "32")
			{
				Platform::Windows32
			}
			else
			{
				Platform::Unknown
			}
		}
		else if cfg!(target_os = "macos")
		{
			if cfg!(target_pointer_width = "64")
			{
				Platform::Osx64
			}
			else if cfg!(target_pointer_width = "32")
			{
				Platform::Osx32
			}
			else
			{
				Platform::Unknown
			}
		}
		else if cfg!(target_os = "linux")
		{
			if cfg!(target_pointer_width = "64")
			{
				Platform::Debian64
			}
			else if cfg!(target_pointer_width = "32")
			{
				Platform::Debian32
			}
			else
			{
				Platform::Unknown
			}
		}
		else
		{
			Platform::Unknown
		}
	}
}
