/* Botslot */

use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct Botslot(u8);

pub fn pool() -> Vec<Botslot>
{
	(b'A'..=b'Z').map(|x| Botslot(x)).collect()
}

impl std::fmt::Display for Botslot
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
	{
		debug_assert!(self.0 >= b'A' && self.0 <= b'Z');
		write!(f, "%{}", self.0 as char)
	}
}

impl std::str::FromStr for Botslot
{
	type Err = DecodeError;

	fn from_str(s: &str) -> Result<Botslot, DecodeError>
	{
		let x: u8 = {
			if !s.is_ascii()
			{
				Err(DecodeError::NonAscii {
					source: s.to_string(),
				})
			}
			else if s.len() < 2
			{
				Err(DecodeError::TooShort {
					source: s.to_string(),
				})
			}
			else if s.len() > 2
			{
				Err(DecodeError::TooLong {
					source: s.to_string(),
				})
			}
			else
			{
				Ok(s.as_bytes()[1])
			}
		}?;

		if x >= b'A' && x <= b'Z'
		{
			Ok(Botslot(x))
		}
		else
		{
			Err(DecodeError::InvalidLetter { letter: x })
		}
	}
}

impl Serialize for Botslot
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.to_string().serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for Botslot
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?;
		std::str::FromStr::from_str(&s).map_err(::serde::de::Error::custom)
	}
}

#[derive(Debug)]
pub enum DecodeError
{
	InvalidLetter
	{
		letter: u8
	},
	TooLong
	{
		source: String
	},
	TooShort
	{
		source: String
	},
	NonAscii
	{
		source: String
	},
}

impl std::error::Error for DecodeError {}

impl std::fmt::Display for DecodeError
{
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
	{
		match self
		{
			DecodeError::InvalidLetter { letter } =>
			{
				write!(f, "invalid slot character '{}'", letter)
			}
			DecodeError::TooLong { source } =>
			{
				write!(f, "too many characters in '{}'", source)
			}
			DecodeError::TooShort { source } =>
			{
				write!(f, "not enough characters in '{}'", source)
			}
			DecodeError::NonAscii { source } =>
			{
				write!(f, "non-ASCII characters in '{}'", source)
			}
		}
	}
}
