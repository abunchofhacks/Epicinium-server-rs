/* Message */

use common::header::*;
use common::version::*;

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

		#[serde(default, skip_serializing_if = "is_zero")]
		metadata: Option<PlatformMetadata>,
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
	Quit,
	Chat
	{
		content: String,

		#[serde(default, skip_serializing_if = "is_zero")]
		sender: Option<String>,

		target: ChatTarget,
	},
	Stamp
	{
		metadata: StampMetadata,
	},
	Download
	{
		content: String,
		metadata: DownloadMetadata,
	},
	Request
	{
		content: String,
	},
	RequestDenied
	{
		content: String,
		metadata: DenyMetadata,
	},
	RequestFulfilled
	{
		content: String,
		metadata: DownloadMetadata,
	},
}

#[derive(PartialEq, Eq, Copy, Clone, Serialize, Deserialize, Default, Debug)]
pub struct PlatformMetadata
{
	#[serde(default, skip_serializing_if = "is_zero")]
	pub platform: Platform,

	#[serde(default)]
	pub patchmode: Patchmode,
}

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

#[derive(PartialEq, Eq, Copy, Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Patchmode
{
	None,
	Server,
	Itchio,
	Gamejolt,
}

impl Default for Patchmode
{
	fn default() -> Patchmode
	{
		Patchmode::None
	}
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

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Debug)]
pub struct StampMetadata
{
	pub image: String,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub tooltip: Option<String>,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub url: Option<String>,
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Debug)]
pub struct DownloadMetadata
{
	#[serde(default, skip_serializing_if = "is_zero")]
	pub name: Option<String>,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub offset: Option<usize>,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub signature: Option<String>,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub compressed: bool,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub executable: bool,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub symbolic: bool,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub progressmask: Option<u16>,
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Debug)]
pub struct DenyMetadata
{
	pub reason: String,
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

// TODO implement deserialize but using reverse unlock_id
type Unlocks = EnumSet<Unlock>;

#[derive(Clone, Debug)]
pub struct LoginData
{
	pub status: ResponseStatus,
	pub account_id: String,
	pub response_data: LoginResponseData,
}

#[derive(Clone, Deserialize, Debug)]
pub struct LoginResponseData
{
	pub username: String,
	pub unlocks: Vec<u8>,
	pub rating: f32,
	pub stars: i32,
	pub recent_stars: i32,
}
