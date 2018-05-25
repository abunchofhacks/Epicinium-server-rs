/* Version */

use std::str::FromStr;
use std::num::ParseIntError;
use std::result::Result;
use serde::Serializer;
use serde::Serialize;
//use serde::Deserialize;


#[derive(PartialEq, Eq, Debug)]
pub struct Version
{
	pub major : i8,
	pub minor : i8,
	pub patch : i8,
	pub release : i8
}

impl Version
{
	pub fn current() -> Version
	{
		Version{
			major: 0,
			minor: 1,
			patch: 0,
			release: 0
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
			format!("{}.{}.{}-rc{}", self.major, self.minor, self.patch,
					self.release)
		}
	}
}

pub enum VersionParseError
{
	INTERROR
	{
		error : ParseIntError,
	},
	PARSEERROR
	{
		message : String,
	},
}

impl From<ParseIntError> for VersionParseError
{
	fn from(err : ParseIntError) -> VersionParseError
	{
		VersionParseError::INTERROR{
			error: err
		}
	}
}

impl FromStr for Version
{
	type Err = VersionParseError;

	fn from_str(s : &str) -> Result<Version, VersionParseError>
	{
		let parts : Vec<&str> = s.trim_matches(|p| p == 'v')
				.split("-rc")
				.flat_map(|p| p.split("."))
				.collect();

		if parts.len() == 3
		{
			Ok(Version{
				major: parts[0].parse::<i8>()?,
				minor: parts[1].parse::<i8>()?,
				patch: parts[2].parse::<i8>()?,
				release: 0,
			})
		}
		else if parts.len() == 4
		{
			Ok(Version{
				major: parts[0].parse::<i8>()?,
				minor: parts[1].parse::<i8>()?,
				patch: parts[2].parse::<i8>()?,
				release: parts[3].parse::<i8>()?,
			})
		}
		else {
			Err(VersionParseError::PARSEERROR{
				message: "Cannot parse ".to_owned() + s
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

/*
impl<'de> Deserialize<'de> for Version
{
	fn deserialize()
	{
		_unimplemented
	}
}
*/
