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
use crate::server::botslot::EmptyBotslot;
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
		metadata: JoinMetadataOrTagMetadata,
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
	ClaimHost
	{
		#[serde(default, skip_serializing_if = "is_zero", rename = "sender")]
		username: Option<String>,
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
		#[serde(rename = "sender")]
		username_or_slot: UsernameOrSlot,

		#[serde(rename = "content")]
		ai_name: String,
	},
	ClaimDifficulty
	{
		#[serde(rename = "sender")]
		username_or_slot: UsernameOrSlot,

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

		#[serde(default, skip_serializing_if = "Option::is_none")]
		metadata: Option<ListAiMetadata>,
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

		#[serde(default, skip_serializing_if = "Option::is_none")]
		metadata: Option<ListRulesetMetadata>,
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

		#[serde(default, skip_serializing_if = "Option::is_none")]
		difficulty: Option<Difficulty>,

		#[serde(default, skip_serializing_if = "Option::is_none")]
		#[serde(rename = "metadata")]
		forwarding: Option<ForwardingMetadata>,
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
	HostSync
	{
		#[serde(default, skip_serializing_if = "Option::is_none")]
		metadata: Option<HostSyncMetadata>,
	},
	HostRejoinRequest
	{
		player: PlayerColor,

		#[serde(rename = "content")]
		username: String,
	},
	HostRejoinChanges
	{
		player: PlayerColor,

		#[serde(rename = "content")]
		username: String,

		changes: Vec<Change>,
	},
	#[serde(rename = "change")]
	Changes
	{
		changes: Vec<Change>,

		#[serde(default, skip_serializing_if = "Option::is_none")]
		#[serde(rename = "metadata")]
		forwarding: Option<ForwardingMetadata>,
	},
	#[serde(rename = "order_new")]
	Orders
	{
		orders: Vec<Order>,

		#[serde(default, skip_serializing_if = "Option::is_none")]
		#[serde(rename = "metadata")]
		forwarding: Option<ForwardingMetadata>,
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
		#[serde(rename = "content")]
		challenge_key: String,

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
	LinkAccounts
	{
		metadata: AccountLinkingMetadata,
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
	Empty(EmptyBotslot),
	Slot(Botslot),
	Username(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum JoinMetadataOrTagMetadata
{
	JoinMetadata(JoinMetadata),
	TagMetadata(TagMetadata),
}

impl Default for JoinMetadataOrTagMetadata
{
	fn default() -> JoinMetadataOrTagMetadata
	{
		JoinMetadataOrTagMetadata::JoinMetadata(Default::default())
	}
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct JoinMetadata
{
	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub desired_username: Option<String>,

	#[serde(default, skip_serializing_if = "Option::is_none")]
	pub merge_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct TagMetadata
{
	#[serde(default, skip_serializing_if = "is_zero")]
	pub dev: bool,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub guest: bool,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub bot: bool,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub supporter: bool,
}

#[derive(
	PartialEq, Eq, Copy, Clone, Serialize, Deserialize, Default, Debug,
)]
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

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AccountLinkingMetadata
{
	pub discord_user_id: String,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct BotAuthorsMetadata
{
	pub authors: String,
}

#[derive(PartialEq, Eq, Clone, Copy, Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum ForwardingMetadata
{
	ConnectedBot
	{
		lobby_id: Keycode, slot: Botslot
	},
	ClientHosted
	{
		player: PlayerColor
	},
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum ListAiMetadata
{
	FromHost
	{
		self_hosted: bool,
	},
	Authors(BotAuthorsMetadata),
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum ListRulesetMetadata
{
	FromHost
	{
		self_hosted: bool
	},
	Forwarding
	{
		lobby_id: Keycode
	},
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
	UsernameRequiredNoUser = 10,
	UsernameRequiredInvalid = 11,
	UsernameRequiredTaken = 12,

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

#[derive(PartialEq, Eq, Clone, Serialize, Deserialize, Debug)]
pub struct HostSyncMetadata
{
	#[serde(default, skip_serializing_if = "is_zero")]
	pub defeated_players: Vec<PlayerColor>,

	pub game_over: bool,

	#[serde(default, skip_serializing_if = "is_zero")]
	pub stars: i32,
}
