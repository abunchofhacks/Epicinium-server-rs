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
