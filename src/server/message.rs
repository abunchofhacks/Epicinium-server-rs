/* Message */

use crate::common::header::*;
use crate::common::keycode::*;
use crate::common::version::*;
use crate::logic::challenge;

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
	JoinLobby
	{
		#[serde(default, skip_serializing_if = "is_zero", rename = "content")]
		lobby_id: Option<Keycode>,

		#[serde(default, skip_serializing_if = "is_zero", rename = "sender")]
		username: Option<String>,

		#[serde(default, skip_serializing_if = "is_zero")]
		metadata: Option<JoinMetadata>,
	},
	LeaveLobby
	{
		#[serde(default, skip_serializing_if = "is_zero", rename = "content")]
		lobby_id: Option<Keycode>,

		#[serde(default, skip_serializing_if = "is_zero", rename = "sender")]
		username: Option<String>,
	},
	MakeLobby
	{
		#[serde(default, skip_serializing_if = "is_zero", rename = "content")]
		lobby_id: Option<Keycode>,
	},
	DisbandLobby
	{
		#[serde(rename = "content")]
		lobby_id: Keycode,
	},
	EditLobby
	{
		#[serde(rename = "content")]
		lobby_id: Keycode,
	},
	SaveLobby
	{
		#[serde(default, skip_serializing_if = "is_zero", rename = "content")]
		lobby_id: Option<Keycode>,
	},
	LockLobby
	{
		#[serde(default, skip_serializing_if = "is_zero", rename = "content")]
		lobby_id: Option<Keycode>,
	},
	UnlockLobby
	{
		#[serde(default, skip_serializing_if = "is_zero", rename = "content")]
		lobby_id: Option<Keycode>,
	},
	NameLobby
	{
		#[serde(rename = "content")]
		lobbyname: String,

		#[serde(default, skip_serializing_if = "is_zero", rename = "sender")]
		lobby_id: Option<Keycode>,
	},
	MaxPlayers
	{
		#[serde(rename = "content")]
		lobby_id: Keycode,

		#[serde(rename = "time")]
		value: i32,
	},
	NumPlayers
	{
		#[serde(rename = "content")]
		lobby_id: Keycode,

		#[serde(rename = "time")]
		value: i32,
	},
	ListChallenge
	{
		#[serde(rename = "content")]
		key: String,

		metadata: challenge::Metadata,
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

#[derive(EnumSetType, Debug, Serialize, Deserialize)]
pub enum Unlock
{
	Unknown,
	Dev,
	BetaAccess,
	Guest,
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
	pub unlocks: EnumSet<Unlock>,
	pub rating: f32,
	pub stars: i32,
	pub recent_stars: i32,
}
