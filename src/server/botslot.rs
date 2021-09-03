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

use log::error;

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

const NAME_POOL: [&str; 26] = [
	"Alice", "Bob", "Carol", "Dave", "Emma", "Frank", "Gwen", "Harold", "Iris",
	"Justin", "Kate", "Leopold", "Mary", "Nick", "Olivia", "Peter", "Quintin",
	"Rachel", "Sasha", "Timothy", "Ursula", "Victor", "Wendy", "Xerxes",
	"Yara", "Zach",
];

impl Botslot
{
	pub fn get_character(&self) -> u8
	{
		self.0
	}

	pub fn get_display_name(&self) -> &'static str
	{
		match self.0
		{
			b'A'..=b'Z' => NAME_POOL[(self.0 - b'A') as usize],
			_ =>
			{
				error!("Invalid botslot {}", self);
				"Eduardo"
			}
		}
	}
}

impl std::fmt::Display for Botslot
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
	{
		match self.0
		{
			b'A'..=b'Z' => write!(f, "%{}", self.0 as char),
			_ => write!(f, "%E"),
		}
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
			else if s.as_bytes()[0] != b'%'
			{
				Err(DecodeError::MissingMarker {
					source: s.to_string(),
				})
			}
			else
			{
				Ok(s.as_bytes()[1])
			}
		}?;

		match x
		{
			b'A'..=b'Z' => Ok(Botslot(x)),
			_ => Err(DecodeError::InvalidLetter { letter: x }),
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
	MissingMarker
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
			DecodeError::MissingMarker { source } =>
			{
				write!(f, "missing marker '%' in '{}'", source)
			}
		}
	}
}

#[derive(Debug, Copy, Clone)]
pub struct EmptyBotslot;

impl std::fmt::Display for EmptyBotslot
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
	{
		write!(f, "")
	}
}

impl std::str::FromStr for EmptyBotslot
{
	type Err = NotEmptyError;

	fn from_str(s: &str) -> Result<EmptyBotslot, NotEmptyError>
	{
		if s.is_empty()
		{
			Ok(EmptyBotslot)
		}
		else
		{
			Err(NotEmptyError::NotEmpty)
		}
	}
}

impl Serialize for EmptyBotslot
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.to_string().serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for EmptyBotslot
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
pub enum NotEmptyError
{
	NotEmpty,
}

impl std::error::Error for NotEmptyError {}

impl std::fmt::Display for NotEmptyError
{
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
	{
		match self
		{
			NotEmptyError::NotEmpty => write!(f, "not empty"),
		}
	}
}
