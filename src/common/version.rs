/* Version */

use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;
use std::fmt::Display;
use std::fmt::Formatter;
use std::num::ParseIntError;
use std::result::Result;
use std::str::FromStr;

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
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
		Version {
			major: 0,
			minor: 1,
			patch: 0,
			release: 0,
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
}

impl Default for Version
{
	fn default() -> Version
	{
		Version::current()
	}
}

impl ToString for Version
{
	fn to_string(&self) -> String
	{
		if self.release == 0
		{
			format!("{}.{}.{}", self.major, self.minor, self.patch)
		}
		else
		{
			format!(
				"{}.{}.{}-rc{}",
				self.major, self.minor, self.patch, self.release
			)
		}
	}
}

#[derive(Debug)]
pub enum VersionParseError
{
	IntError
	{
		error: ParseIntError
	},
	ParseError
	{
		message: String
	},
}

impl Display for VersionParseError
{
	fn fmt(&self, f: &mut Formatter) -> ::std::fmt::Result
	{
		match self
		{
			&VersionParseError::IntError { ref error } => error.fmt(f),
			&VersionParseError::ParseError { ref message } => message.fmt(f),
		}
	}
}

impl From<ParseIntError> for VersionParseError
{
	fn from(err: ParseIntError) -> VersionParseError
	{
		VersionParseError::IntError { error: err }
	}
}

impl FromStr for Version
{
	type Err = VersionParseError;

	fn from_str(s: &str) -> Result<Version, VersionParseError>
	{
		let parts: Vec<&str> = s
			.trim_matches(|p| p == 'v')
			.split("-rc")
			.flat_map(|p| p.split("."))
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
			Err(VersionParseError::ParseError {
				message: "Cannot parse ".to_owned() + s,
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
		FromStr::from_str(&s).map_err(::serde::de::Error::custom)
	}
}
