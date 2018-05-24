/* Version */

use std::str::FromStr;
use std::num::ParseIntError;
use std::result::Result;
use serde::Serializer;
use serde::Serialize;
use serde::Deserialize;


// TODO write custom serialize and deserialize
#[derive(PartialEq, Eq, Debug)]
pub struct Version
{
	pub major : i8,
	pub minor : i8,
	pub patch : i8,
	pub release : i8
}

impl Default for Version
{
	fn default() -> Version
	{
		Version{
			major: 0,
			minor: 1,
			patch: 0,
			release: 0
		}
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

impl FromStr for Version
{
	type Err = ParseIntError;
	fn from_str(s : &str) -> Result<Version, ParseIntError>
	{
		let parts : Vec<i8> = s.trim_matches(|p| p == 'v')
				.split("-rc")
				.map(|p| p.split("."))
				.concat()
				.map(|p| p.parse())
				.collect();

		if parts.len() == 3
		{
			Version{
				major: parts[0],
				minor: parts[1],
				patch: parts[2],
				release: 0,
			}
		}
		else if parts.len() == 4
		{
			Version{
				major: parts[0],
				minor: parts[1],
				patch: parts[2],
				release: parts[3],
			}
		}
		else
		{
			ParseIntError{}
		}
	}
}

impl Serialize for Version
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize(self.to_string())
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
