/* Message */

use crate::common::header::*;
use crate::common::version::*;

use enumset::*;

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum Message
{
	Pulse,
	Ping,
	Pong,
	Version
	{
		version: Version,
	},
	JoinServer
	{
		#[serde(default, skip_serializing_if = "is_zero")]
		status: Option<ResponseStatus>,

		#[serde(default, skip_serializing_if = "is_zero")]
		content: Option<String>,

		#[serde(default, skip_serializing_if = "is_zero")]
		sender: Option<String>,

		#[serde(default, skip_serializing_if = "is_zero")]
		metadata: Option<JoinMetadata>,
	},
	LeaveServer
	{
		#[serde(default, skip_serializing_if = "is_zero")]
		content: Option<String>,
	},
	Init,
	Closing,
	Closed,
	Quit,
	Chat
	{
		content: String,

		#[serde(default, skip_serializing_if = "is_zero")]
		sender: Option<String>,

		target: ChatTarget,
	},
}

#[derive(PartialEq, Eq, Copy, Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ChatTarget
{
	General,
	Lobby,
}

#[derive(PartialEq, Eq, Copy, Clone, Serialize, Deserialize, Default, Debug)]
pub struct JoinMetadata
{
	#[serde(default, skip_serializing_if = "is_zero")]
	pub dev: bool,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub guest: bool,
}

#[derive(
	PartialEq, Eq, Copy, Clone, Serialize_repr, Deserialize_repr, Debug,
)]
#[repr(u8)]
pub enum ResponseStatus
{
	Success = 0,
	CredsInvalid = 1,
	AccountLocked = 2,
	UsernameTaken = 3,
	EmailTaken = 4,
	AccountDisabled = 5,
	KeyTaken = 6,  // only used for key activation (for now)
	IpBlocked = 7, // only used for key activation (for now)
	KeyRequired = 8,

	DatabaseError = 94,
	MethodInvalid = 95,
	RequestMalformed = 96,
	ResponseMalformed = 97,
	ConnectionFailed = 98,
	UnknownError = 99,
}

#[derive(EnumSetType, Debug)]
pub enum Unlock
{
	Unknown,
	Dev,
	Access,
	Guest,
}

pub fn unlock_id(unlock: Unlock) -> u8
{
	if cfg!(debug_assertions)
	{
		match unlock
		{
			Unlock::Unknown => 0,
			Unlock::Dev => 2,
			Unlock::Access => 9,
			Unlock::Guest => 10,
		}
	}
	else
	{
		match unlock
		{
			Unlock::Unknown => 0,
			Unlock::Dev => 2,
			Unlock::Access => 3,
			Unlock::Guest => 4,
		}
	}
}

pub fn unlock_from_unlock_id(x: u8) -> Unlock
{
	if cfg!(debug_assertions)
	{
		match x
		{
			2 => Unlock::Dev,
			9 => Unlock::Access,
			10 => Unlock::Guest,
			_ => Unlock::Unknown,
		}
	}
	else
	{
		match x
		{
			2 => Unlock::Dev,
			3 => Unlock::Access,
			4 => Unlock::Guest,
			_ => Unlock::Unknown,
		}
	}
}

#[derive(Clone, Deserialize, Debug)]
pub struct LoginResponse
{
	pub status: ResponseStatus,

	#[serde(flatten)]
	pub data: Option<LoginData>,
}

#[derive(Clone, Deserialize, Debug)]
pub struct LoginData
{
	pub username: String,
	// TODO deserialize using unlock_from_unlock_id
	pub unlocks: Vec<u8>,
	pub rating: f32,
	pub stars: i32,
	pub recent_stars: i32,
}
