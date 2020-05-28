/* Message */

use crate::common::header::*;
use crate::common::keycode::*;
use crate::common::version::*;
use crate::logic::challenge;
use crate::logic::difficulty::Difficulty;
use crate::logic::map;
use crate::server::botslot::Botslot;

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
	MakeLobby {},
	SaveLobby {},
	DisbandLobby
	{
		#[serde(rename = "content")]
		lobby_id: Keycode,
	},
	LockLobby {},
	UnlockLobby {},
	NameLobby
	{
		#[serde(rename = "content")]
		lobby_name: String,
	},
	ListLobby
	{
		#[serde(rename = "content")]
		lobby_id: Keycode,

		#[serde(rename = "sender")]
		lobby_name: String,

		metadata: LobbyMetadata,
	},
	ClaimRole
	{
		#[serde(rename = "sender")]
		username: String,

		role: Role,
	},
	ClaimAi
	{
		#[serde(default, skip_serializing_if = "is_zero", rename = "sender")]
		slot: Option<Botslot>,

		#[serde(rename = "content")]
		ai_name: String,
	},
	ClaimDifficulty
	{
		#[serde(default, skip_serializing_if = "is_zero", rename = "sender")]
		slot: Option<Botslot>,

		difficulty: Difficulty,
	},
	PickMap
	{
		#[serde(rename = "content")]
		map_name: String,
	},
	PickTimer
	{
		#[serde(rename = "time")]
		seconds: u32,
	},
	PickChallenge
	{
		#[serde(rename = "content")]
		challenge_key: String,
	},
	PickRuleset
	{
		#[serde(rename = "content")]
		ruleset_name: String,
	},
	AddBot
	{
		#[serde(default, skip_serializing_if = "is_zero", rename = "content")]
		slot: Option<Botslot>,
	},
	RemoveBot
	{
		#[serde(rename = "content")]
		slot: Botslot,
	},
	ListChallenge
	{
		#[serde(rename = "content")]
		key: String,

		metadata: challenge::Metadata,
	},
	ListAi
	{
		#[serde(rename = "content")]
		ai_name: String,
	},
	ListMap
	{
		#[serde(rename = "content")]
		map_name: String,

		metadata: map::Metadata,
	},
	ListRuleset
	{
		#[serde(rename = "content")]
		ruleset_name: String,
	},
	Challenge,
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

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug, Enum)]
#[serde(rename_all = "lowercase")]
pub enum Role
{
	Player,
	Observer,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug, Enum)]
#[serde(rename_all = "lowercase")]
pub enum VisionType
{
	Normal,
	Global,
}

#[derive(PartialEq, Eq, Copy, Clone, Serialize, Deserialize, Default, Debug)]
pub struct JoinMetadata
{
	#[serde(default, skip_serializing_if = "is_zero")]
	pub dev: bool,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub guest: bool,
}

#[derive(PartialEq, Eq, Copy, Clone, Serialize, Deserialize, Default, Debug)]
pub struct LobbyMetadata
{
	#[serde(default, skip_serializing_if = "is_zero")]
	pub max_players: i32,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub num_players: i32,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub num_bot_players: i32,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub is_public: bool,
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
