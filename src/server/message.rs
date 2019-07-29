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
	},
	Request
	{
		content: String,
	},
	RequestDenied
	{
		content: String,
	},
	RequestFulfilled
	{
		content: String,
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
