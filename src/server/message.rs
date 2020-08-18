/* Message */

use crate::common::header::*;
use crate::common::keycode::*;
use crate::common::version::*;
use crate::logic::challenge;
use crate::logic::change::Change;
use crate::logic::difficulty::Difficulty;
use crate::logic::map;
use crate::logic::order::Order;
use crate::logic::player::PlayerColor;
use crate::logic::ruleset;
use crate::server::botslot::Botslot;
use crate::server::lobby;
use crate::server::lobby::LobbyType;

use serde_derive::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

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

		#[serde(default, skip_serializing_if = "Option::is_none")]
		#[serde(rename = "metadata")]
		invite: Option<lobby::Invite>,
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
		#[serde(default, skip_serializing_if = "is_zero")]
		metadata: Option<LobbyMetadata>,
	},
	SaveLobby,
	DisbandLobby
	{
		#[serde(rename = "content")]
		lobby_id: Keycode,
	},
	LockLobby,
	UnlockLobby,
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
	ClaimColor
	{
		#[serde(rename = "sender")]
		username_or_slot: UsernameOrSlot,

		#[serde(rename = "player")]
		color: PlayerColor,
	},
	#[serde(rename = "claim_visiontype")] // The capital T would cause "_type".
	ClaimVisionType
	{
		#[serde(rename = "sender")]
		username_or_slot: UsernameOrSlot,

		visiontype: VisionType,
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
	EnableCustomMaps,
	AssignColor
	{
		#[serde(rename = "sender")]
		name: String,

		#[serde(rename = "player")]
		color: PlayerColor,
	},
	RulesetRequest
	{
		#[serde(rename = "content")]
		ruleset_name: String,
	},
	RulesetData
	{
		#[serde(rename = "content")]
		ruleset_name: String,

		data: ruleset::Data,
	},
	RulesetUnknown
	{
		#[serde(rename = "content")]
		ruleset_name: String,
	},
	Secrets
	{
		#[serde(rename = "metadata")]
		secrets: lobby::Secrets,
	},
	Skins
	{
		metadata: map::Metadata,
	},
	InGame
	{
		#[serde(rename = "content")]
		lobby_id: Keycode,

		#[serde(rename = "sender")]
		username: String,

		role: Role,
	},
	Start,
	Game
	{
		#[serde(default, skip_serializing_if = "is_zero")]
		role: Option<Role>,

		#[serde(default, skip_serializing_if = "is_zero")]
		player: Option<PlayerColor>,

		#[serde(default, skip_serializing_if = "is_zero", rename = "content")]
		ruleset_name: Option<String>,

		#[serde(default, skip_serializing_if = "is_zero", rename = "time")]
		timer_in_seconds: Option<u32>,
	},
	Tutorial
	{
		#[serde(default, skip_serializing_if = "is_zero")]
		role: Option<Role>,

		#[serde(default, skip_serializing_if = "is_zero")]
		player: Option<PlayerColor>,

		#[serde(default, skip_serializing_if = "is_zero", rename = "content")]
		ruleset_name: Option<String>,

		#[serde(default, skip_serializing_if = "is_zero", rename = "time")]
		timer_in_seconds: Option<u32>,
	},
	Challenge,
	Briefing
	{
		#[serde(rename = "metadata")]
		briefing: challenge::MissionBriefing,
	},
	#[serde(rename = "replay")]
	ReplayWithAnimations
	{
		#[serde(rename = "time")]
		on_or_off: OnOrOff,
	},
	Resign
	{
		#[serde(default, skip_serializing_if = "is_zero", rename = "content")]
		username: Option<String>,
	},
	#[serde(rename = "change")]
	Changes
	{
		changes: Vec<Change>,
	},
	#[serde(rename = "order_old")]
	OrdersOld
	{
		orders: Vec<Order>,
	},
	#[serde(rename = "order_new")]
	OrdersNew
	{
		orders: Vec<Order>,
	},
	Sync
	{
		#[serde(default, skip_serializing_if = "is_zero", rename = "time")]
		time_remaining_in_seconds: Option<u32>,
	},
	Init,
	RatingAndStars
	{
		#[serde(rename = "content")]
		username: String,

		#[serde(default, skip_serializing_if = "is_zero")]
		rating: f64,

		#[serde(default, skip_serializing_if = "is_zero", rename = "time")]
		stars: i32,
	},
	#[serde(rename = "rating")]
	UpdatedRating
	{
		#[serde(default, skip_serializing_if = "is_zero")]
		rating: f64,
	},
	RecentStars
	{
		#[serde(default, skip_serializing_if = "is_zero", rename = "time")]
		stars: i32,
	},
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
	Debug
	{
		content: String,
	},
}

#[derive(PartialEq, Eq, Copy, Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ChatTarget
{
	General,
	Lobby,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Role
{
	Player,
	Observer,
}

impl Role
{
	pub fn vision_level(&self) -> PlayerColor
	{
		match self
		{
			Role::Observer => PlayerColor::Observer,
			Role::Player => PlayerColor::Blind,
		}
	}
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum VisionType
{
	Normal,
	Global,
}

// Botslot strings always start with % and usernames cannot contain %, so we
// can try to deserialize as a Botslot and if that fails it is a username.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum UsernameOrSlot
{
	Slot(Botslot),
	Username(String),
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
	pub lobby_type: LobbyType,

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
	EmailUnverified = 9,
	UsernameRequired = 10,

	DatabaseError = 94,
	MethodInvalid = 95,
	RequestMalformed = 96,
	ResponseMalformed = 97,
	ConnectionFailed = 98,
	UnknownError = 99,
}

#[derive(Debug, Copy, Clone, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum OnOrOff
{
	Off = 0,
	On = 1,
}
