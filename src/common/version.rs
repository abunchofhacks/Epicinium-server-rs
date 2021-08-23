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

use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use std::result::Result;

#[derive(PartialEq, Eq, PartialOrd, Debug, Copy, Clone)]
pub struct Version
{
	pub major: u8,
	pub minor: u8,
	pub patch: u8,
	pub release: u8,
}

impl Version
{
	pub fn current() -> Version
	{
		if cfg!(feature = "version-is-dev")
		{
			Version::dev()
		}
		else if cfg!(feature = "candidate")
		{
			Version::latest()
		}
		else if cfg!(debug_assertions)
		{
			Version::dev()
		}
		else
		{
			Version::latest().release()
		}
	}

	pub fn latest() -> Version
	{
		Version {
			major: 1,
			minor: 0,
			patch: 12,
			release: 1,
		}
	}

	pub fn dev() -> Version
	{
		Version {
			major: 255,
			minor: 255,
			patch: 255,
			release: 1,
		}
	}

	pub fn undefined() -> Version
	{
		Version {
			major: 255,
			minor: 255,
			patch: 255,
			release: 255,
		}
	}

	pub fn release(&self) -> Version
	{
		Version {
			major: self.major,
			minor: self.minor,
			patch: self.patch,
			release: 0,
		}
	}

	pub fn exact(major: u8, minor: u8, patch: u8, release: u8) -> Version
	{
		Version {
			major,
			minor,
			patch,
			release,
		}
	}

	pub fn is_release(&self) -> bool
	{
		self.release == 0
	}
}

impl std::fmt::Display for Version
{
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
	{
		if self.release == 0
		{
			write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
		}
		else
		{
			write!(
				f,
				"{}.{}.{}-rc{}",
				self.major, self.minor, self.patch, self.release
			)
		}
	}
}

#[derive(Debug)]
pub enum ParseError
{
	Int
	{
		error: std::num::ParseIntError
	},
	Separator
	{
		source: String
	},
}

impl std::error::Error for ParseError {}

impl std::fmt::Display for ParseError
{
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
	{
		match self
		{
			ParseError::Int { error } => error.fmt(f),
			ParseError::Separator { source } => write!(
				f,
				"failed to parse '{}' as a dot-separated version",
				source
			),
		}
	}
}

impl From<std::num::ParseIntError> for ParseError
{
	fn from(err: std::num::ParseIntError) -> ParseError
	{
		ParseError::Int { error: err }
	}
}

impl std::str::FromStr for Version
{
	type Err = ParseError;

	fn from_str(s: &str) -> Result<Version, ParseError>
	{
		let parts: Vec<&str> = s
			.trim_matches(|p| p == 'v')
			.split("-rc")
			.flat_map(|p| p.split('.'))
			.collect();

		if parts.len() == 3
		{
			Ok(Version {
				major: parts[0].parse::<u8>()?,
				minor: parts[1].parse::<u8>()?,
				patch: parts[2].parse::<u8>()?,
				release: 0,
			})
		}
		else if parts.len() == 4
		{
			Ok(Version {
				major: parts[0].parse::<u8>()?,
				minor: parts[1].parse::<u8>()?,
				patch: parts[2].parse::<u8>()?,
				release: parts[3].parse::<u8>()?,
			})
		}
		else
		{
			Err(ParseError::Separator {
				source: s.to_string(),
			})
		}
	}
}

impl Serialize for Version
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.to_string().serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for Version
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?;
		std::str::FromStr::from_str(&s).map_err(::serde::de::Error::custom)
	}
}
