/* Message */

use common::header::*;
use common::version::*;

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
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
	JoinServer,
	LeaveServer,
	Closing,
	Quit,
	Chat
	{
		content: String,
		sender: String,
		target: ChatTarget,
	},
}

#[derive(PartialEq, Eq, Copy, Clone, Serialize, Deserialize, Default, Debug)]
#[serde(rename_all = "lowercase")]
pub struct PlatformMetadata
{
	pub platform: Platform,
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
