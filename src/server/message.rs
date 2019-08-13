/* Message */

use common::header::*;
use common::version::*;

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
		status: Option<i32>,

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
#[serde(rename_all = "lowercase")]
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
#[serde(rename_all = "lowercase")]
pub struct JoinMetadata
{
	#[serde(default, skip_serializing_if = "is_zero")]
	pub dev: bool,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub guest: bool,
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub struct StampMetadata
{
	pub image: String,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub tooltip: Option<String>,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub url: Option<String>,
}

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
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
#[serde(rename_all = "lowercase")]
pub struct DenyMetadata
{
	pub reason: String,
}

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
