/**/

use crate::common::base32;
use crate::common::keycode::Keycode;

use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use serde::Serializer;

use rand::Rng;

#[derive(Debug, Clone)]
pub struct Secret
{
	pub lobby_id: Keycode,
	pub client_id: Keycode,
	pub salt: Salt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Salt([u8; 20]);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Secrets
{
	pub join_secret: Secret,
	pub spectate_secret: Secret,
}

#[derive(Debug, Clone)]
pub struct Salts
{
	pub join_secret_salt: Salt,
	pub spectate_secret_salt: Salt,
}

impl Salts
{
	pub fn generate() -> Salts
	{
		let mut rng = rand::thread_rng();
		Salts {
			join_secret_salt: Salt(rng.gen()),
			spectate_secret_salt: Salt(rng.gen()),
		}
	}
}

impl Secrets
{
	pub fn create(
		lobby_id: Keycode,
		client_id: Keycode,
		salts: Salts,
	) -> Secrets
	{
		Secrets {
			join_secret: Secret {
				lobby_id,
				client_id,
				salt: salts.join_secret_salt,
			},
			spectate_secret: Secret {
				lobby_id,
				client_id,
				salt: salts.spectate_secret_salt,
			},
		}
	}
}

impl std::fmt::Display for Secret
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
	{
		write!(f, "{}-{}-{}", self.lobby_id, self.client_id, self.salt)
	}
}

impl std::str::FromStr for Secret
{
	type Err = ParseError;

	fn from_str(s: &str) -> Result<Secret, ParseError>
	{
		let parts: Vec<&str> = s.split('-').collect();
		if parts.len() < 3
		{
			return Err(ParseError::TooFewParts);
		}
		else if parts.len() > 3
		{
			return Err(ParseError::TooManyParts);
		}
		let secret = Secret {
			lobby_id: Keycode::from_str(parts[0])?,
			client_id: Keycode::from_str(parts[1])?,
			salt: Salt::from_str(parts[2])?,
		};
		Ok(secret)
	}
}

impl Serialize for Secret
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.to_string().serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for Secret
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?;
		std::str::FromStr::from_str(&s).map_err(serde::de::Error::custom)
	}
}

impl std::fmt::Display for Salt
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result
	{
		write!(f, "{}", base32::encode(&self.0))
	}
}

impl std::str::FromStr for Salt
{
	type Err = base32::DecodeError;

	fn from_str(s: &str) -> Result<Salt, base32::DecodeError>
	{
		let bytes: Vec<u8> = base32::decode(s)?;

		let mut salt = Salt([0; 20]);
		if bytes.len() < salt.0.len()
		{
			return Err(base32::DecodeError::WordTooShort {
				source: s.to_string(),
				min_length_in_bits: 8 * salt.0.len(),
			});
		}
		else if bytes.len() > salt.0.len()
		{
			return Err(base32::DecodeError::WordTooLong {
				source: s.to_string(),
				max_length_in_bits: 8 * salt.0.len(),
			});
		}
		salt.0.copy_from_slice(&bytes);

		Ok(salt)
	}
}

impl Serialize for Salt
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		self.to_string().serialize(serializer)
	}
}

impl<'de> Deserialize<'de> for Salt
{
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: Deserializer<'de>,
	{
		let s = String::deserialize(deserializer)?;
		std::str::FromStr::from_str(&s).map_err(serde::de::Error::custom)
	}
}

#[derive(Debug)]
pub enum ParseError
{
	TooManyParts,
	TooFewParts,
	Base32(base32::DecodeError),
}

impl std::error::Error for ParseError {}

impl std::fmt::Display for ParseError
{
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
	{
		match self
		{
			ParseError::TooManyParts {} =>
			{
				write!(f, "too many parts in secret")
			}
			ParseError::TooFewParts {} =>
			{
				write!(f, "not enough parts in secret")
			}
			ParseError::Base32(error) => error.fmt(f),
		}
	}
}

impl From<base32::DecodeError> for ParseError
{
	fn from(error: base32::DecodeError) -> Self
	{
		ParseError::Base32(error)
	}
}
