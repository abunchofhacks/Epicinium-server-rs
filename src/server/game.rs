/* ServerGame */

use crate::common::keycode::*;
use crate::logic::ai;
use crate::logic::automaton;
use crate::logic::automaton::Automaton;
use crate::logic::challenge;
use crate::logic::challenge::ChallengeId;
use crate::logic::change::ChangeSet;
use crate::logic::difficulty::Difficulty;
use crate::logic::map;
use crate::logic::order::Order;
use crate::logic::player::PlayerColor;
use crate::server::botslot::Botslot;
use crate::server::chat;
use crate::server::client;
use crate::server::discord_api;
use crate::server::lobby;
use crate::server::lobby::LobbyType;
use crate::server::lobby::Update;
use crate::server::login::UserId;
use crate::server::message::*;
use crate::server::rating;

use std::fmt;

use log::*;

use tokio::sync::mpsc;
use tokio::time as timer;
use tokio::time::{Duration, Instant};

#[derive(Debug)]
pub struct PlayerClient
{
	pub id: Keycode,
	pub user_id: UserId,
	pub username: String,
	pub handle: client::Handle,
	pub rating_callback: Option<mpsc::Sender<rating::Update>>,

	pub color: PlayerColor,
	pub vision: VisionType,

	pub is_defeated: bool,
	pub has_synced: bool,
	pub submitted_orders: Option<Vec<Order>>,
}

impl PlayerClient
{
	fn is_connected(&self) -> bool
	{
		!self.handle.is_disconnected()
	}

	fn is_retired(&self) -> bool
	{
		self.rating_callback.is_none()
	}
}

#[derive(Debug)]
pub struct BotClient
{
	pub slot: Botslot,
	pub difficulty: Difficulty,
	pub descriptive_name: String,
	pub ai_metadata: ai::Metadata,
	pub connected_bot_metadata: ConnectedBotMetadata,

	pub id: Keycode,
	pub user_id: UserId,
	pub handle: client::Handle,

	pub color: PlayerColor,
	pub vision: VisionType,

	pub is_defeated: bool,
	pub submitted_orders: Option<Vec<Order>>,
}

impl BotClient
{
	fn is_connected(&self) -> bool
	{
		!self.handle.is_disconnected()
	}

	fn is_retired(&self) -> bool
	{
		false
	}
}

#[derive(Debug)]
pub struct LocalBot
{
	pub slot: Botslot,
	pub ai: ai::Commander,

	pub color: PlayerColor,
	pub vision: VisionType,

	pub is_defeated: bool,
}

#[derive(Debug)]
pub struct WatcherClient
{
	pub id: Keycode,
	pub user_id: UserId,
	pub username: String,
	pub handle: client::Handle,

	pub role: Role,
	pub vision_level: PlayerColor,

	pub has_synced: bool,
}

impl WatcherClient
{
	fn is_connected(&self) -> bool
	{
		!self.handle.is_disconnected()
	}
}

pub struct Setup
{
	pub lobby_id: Keycode,
	pub lobby_name: String,
	pub lobby_description_metadata: LobbyMetadata,

	pub players: Vec<PlayerClient>,
	pub connected_bots: Vec<BotClient>,
	pub local_bots: Vec<LocalBot>,
	pub watchers: Vec<WatcherClient>,
	pub map_name: String,
	pub map_metadata: map::Metadata,
	pub ruleset_name: String,
	pub planning_time_in_seconds: Option<u32>,
	pub lobby_type: LobbyType,
	pub challenge: Option<ChallengeId>,
	pub is_public: bool,
}

pub async fn run(
	setup: Setup,
	mut discord_api: mpsc::Sender<discord_api::Post>,
	mut updates: mpsc::Receiver<Update>,
) -> Result<(), Error>
{
	let Setup {
		lobby_id,
		lobby_name,
		lobby_description_metadata,
		mut players,
		mut connected_bots,
		mut local_bots,
		mut watchers,
		map_name,
		map_metadata,
		ruleset_name,
		planning_time_in_seconds,
		lobby_type,
		challenge,
		is_public,
	} = setup;

	let mut playercolors = Vec::new();
	for player in &players
	{
		playercolors.push(player.color);
	}
	for bot in &connected_bots
	{
		playercolors.push(bot.color);
	}
	for bot in &local_bots
	{
		playercolors.push(bot.color);
	}

	let mut automaton = Automaton::create(playercolors, &ruleset_name)?;

	// Certain players might have global vision.
	for player in &players
	{
		match player.vision
		{
			VisionType::Normal => (),
			VisionType::Global => automaton.grant_global_vision(player.color),
		}
	}
	for bot in &connected_bots
	{
		match bot.vision
		{
			VisionType::Normal => (),
			VisionType::Global => automaton.grant_global_vision(bot.color),
		}
	}
	for bot in &local_bots
	{
		match bot.vision
		{
			VisionType::Normal => (),
			VisionType::Global => automaton.grant_global_vision(bot.color),
		}
	}

	// Tell everyone that the game is starting.
	for client in &mut players
	{
		if let LobbyType::Tutorial = lobby_type
		{
			client.handle.send(Message::Tutorial {
				role: Some(Role::Player),
				player: Some(client.color),
				ruleset_name: Some(ruleset_name.clone()),
				timer_in_seconds: planning_time_in_seconds,
			});
		}
		else
		{
			client.handle.send(Message::Game {
				role: Some(Role::Player),
				player: Some(client.color),
				ruleset_name: Some(ruleset_name.clone()),
				timer_in_seconds: planning_time_in_seconds,
				difficulty: None,
				connected_bot: None,
			});
		}
	}

	for client in &mut connected_bots
	{
		client.handle.send(Message::Game {
			role: Some(Role::Player),
			player: Some(client.color),
			ruleset_name: Some(ruleset_name.clone()),
			timer_in_seconds: planning_time_in_seconds,
			difficulty: Some(client.difficulty),
			connected_bot: Some(client.connected_bot_metadata),
		});
	}

	for client in &mut watchers
	{
		client.handle.send(Message::Game {
			role: Some(Role::Observer),
			player: None,
			ruleset_name: Some(ruleset_name.clone()),
			timer_in_seconds: planning_time_in_seconds,
			difficulty: None,
			connected_bot: None,
		});
	}

	// Tell everyone who is playing as which color.
	let mut initial_messages = Vec::new();
	for player in &players
	{
		initial_messages.push(Message::AssignColor {
			color: player.color,
			name: player.username.clone(),
		});
	}
	for bot in &mut connected_bots
	{
		let descriptive_name = bot.descriptive_name.clone();
		initial_messages.push(Message::AssignColor {
			color: bot.color,
			name: descriptive_name,
		});
	}
	for bot in &mut local_bots
	{
		let descriptive_name = bot.ai.descriptive_name()?;
		initial_messages.push(Message::AssignColor {
			color: bot.color,
			name: descriptive_name,
		});
	}

	// Tell everyone which skins are being used.
	initial_messages.push(Message::Skins {
		metadata: map_metadata.clone(),
	});

	// A challenge might be set.
	if lobby_type == LobbyType::Challenge
	{
		let challenge_id = match challenge
		{
			Some(challenge_id) => challenge_id,
			None => return Err(Error::MissingChallengeId),
		};
		automaton.set_challenge(challenge_id)?;

		let briefing = challenge::load_briefing(challenge_id)?;
		initial_messages.push(Message::Briefing { briefing });
	}

	// Send the initial messages.
	for client in &mut players
	{
		for message in &initial_messages
		{
			client.handle.send(message.clone());
		}
	}
	for client in &mut watchers
	{
		for message in &initial_messages
		{
			client.handle.send(message.clone());
		}
	}

	// Prepare metadata for the recording.
	let mut metadata_players = Vec::new();
	for player in &players
	{
		metadata_players.push(automaton::PlayerMetadata {
			color: player.color,
			username: player.username.clone(),
		});
	}
	let mut metadata_watchers = Vec::new();
	for watcher in &watchers
	{
		metadata_watchers.push(automaton::WatcherMetadata {
			username: watcher.username.clone(),
		});
	}
	let mut metadata_bots = Vec::new();
	for bot in &mut connected_bots
	{
		let ai_metadata = bot.ai_metadata.clone();
		metadata_bots.push(automaton::BotMetadata {
			color: bot.color,
			ai_metadata,
		});
	}
	for bot in &mut local_bots
	{
		let ai_metadata = bot.ai.metadata()?;
		metadata_bots.push(automaton::BotMetadata {
			color: bot.color,
			ai_metadata,
		});
	}
	let metadata = automaton::Metadata {
		map_name: map_name.clone(),
		map_metadata: map_metadata.clone(),
		is_online: true,
		planning_time_in_seconds_or_zero: planning_time_in_seconds.unwrap_or(0),
		players: metadata_players,
		watchers: metadata_watchers,
		bots: metadata_bots,
	};

	// Load the map.
	let shuffleplayers = match lobby_type
	{
		LobbyType::Generic => true,
		LobbyType::OneVsOne => true,
		LobbyType::Custom => true,
		LobbyType::Tutorial => false,
		LobbyType::Challenge => false,
		LobbyType::Replay => false,
	};
	automaton.load(map_name.clone(), shuffleplayers)?;
	automaton.start_recording(metadata, lobby_id.to_string())?;

	// Games on custom maps are unrated because the map might not be balanced.
	// Challenges are unrated because you cannot get 100 points.
	let is_rated = match lobby_type
	{
		LobbyType::Generic => true,
		LobbyType::OneVsOne => true,
		LobbyType::Custom => false,
		LobbyType::Tutorial => true,
		LobbyType::Challenge => false,
		LobbyType::Replay => false,
	};

	// Is this game rated?
	let match_type = if !is_rated
	{
		MatchType::Unrated
	}
	// Is this a competitive 1v1 match with two humans?
	else if lobby_type == LobbyType::OneVsOne
	{
		MatchType::Competitive
	}
	// Is this a friendly 1v1 match with two humans?
	else if players.len() == 2
		&& connected_bots.len() == 0
		&& local_bots.len() == 0
	{
		MatchType::FriendlyOneVsOne
	}
	// Is this a free for all match with at least two humans?
	else if players.len() >= 2
	{
		MatchType::FreeForAll {
			num_non_bot_players: players.len(),
		}
	}
	// Is this a versus AI match?
	else if players.len() == 1
	{
		MatchType::VersusAi
	}
	// Otherwise this match contains only bots.
	else
	{
		MatchType::Unrated
	};

	// Is this a (rated or unrated) 1v1 match between two humans?
	let mentioned_on_discord = if players.len() == 2
		&& connected_bots.len() == 0
		&& local_bots.len() == 0
	{
		let post = discord_api::Post::GameStarted {
			is_rated,
			first_player_username: players[0].username.clone(),
			second_player_username: players[1].username.clone(),
			map_name: map_name.clone(),
			ruleset_name: ruleset_name.clone(),
			planning_time_in_seconds_or_zero: planning_time_in_seconds
				.unwrap_or(0),
		};
		discord_api.send(post).await?;

		Some(MentionedOnDiscord {
			first_color: players[0].color,
			second_color: players[1].color,
			first_player_username: players[0].username.clone(),
			second_player_username: players[1].username.clone(),
		})
	}
	else
	{
		None
	};

	let lobby_info = LobbyInfo {
		id: lobby_id,
		name: lobby_name,
		description_metadata: lobby_description_metadata,
		is_public,
		match_type,
		challenge,
		num_bots: connected_bots.len() + local_bots.len(),
		map_name,
		map_metadata,
		ruleset_name,
		planning_time_in_seconds,
		initial_messages,
	};

	loop
	{
		let state = iterate(
			&lobby_info,
			&mut automaton,
			&mut players,
			&mut connected_bots,
			&mut local_bots,
			&mut watchers,
			&mut updates,
			planning_time_in_seconds,
		)
		.await?;

		match state
		{
			State::InProgress => (),
			State::Finished => break,
			State::Abandoned => break,
		}
	}

	// Did we send a gameStarted post?
	if let Some(MentionedOnDiscord {
		first_color,
		second_color,
		first_player_username,
		second_player_username,
	}) = mentioned_on_discord
	{
		let post = discord_api::Post::GameEnded {
			is_rated,
			first_player_username,
			is_first_player_defeated: automaton.is_defeated(first_color),
			first_player_score: automaton.score(first_color),
			second_player_username,
			is_second_player_defeated: automaton.is_defeated(second_color),
			second_player_score: automaton.score(second_color),
		};
		discord_api.send(post).await?;
	}

	// If there are non-bot non-observer participants, adjust their ratings.
	for client in players.iter_mut()
	{
		retire(&lobby_info, &mut automaton, client).await?;
	}

	debug!("Game has finished in lobby {}; lingering...", lobby_id);
	linger(
		&lobby_info,
		&mut players,
		&mut connected_bots,
		&mut watchers,
		&mut updates,
	)
	.await?;

	Ok(())
}

#[derive(Debug)]
pub enum Sub
{
	Orders
	{
		client_id: Keycode,
		orders: Vec<Order>,
	},
	Sync
	{
		client_id: Keycode
	},
	Resign
	{
		client_id: Keycode
	},
	BotOrders
	{
		client_id: Keycode,
		slot: Botslot,
		orders: Vec<Order>,
	},
}

#[derive(Debug)]
struct LobbyInfo
{
	id: Keycode,
	name: String,
	is_public: bool,
	match_type: MatchType,
	challenge: Option<ChallengeId>,
	num_bots: usize,
	map_name: String,
	map_metadata: map::Metadata,
	ruleset_name: String,
	planning_time_in_seconds: Option<u32>,
	description_metadata: LobbyMetadata,
	initial_messages: Vec<Message>,
}

struct MentionedOnDiscord
{
	first_color: PlayerColor,
	second_color: PlayerColor,
	first_player_username: String,
	second_player_username: String,
}

#[derive(Debug, PartialEq, Eq)]
enum State
{
	InProgress,
	Finished,
	Abandoned,
}

async fn iterate(
	lobby: &LobbyInfo,
	automaton: &mut Automaton,
	players: &mut Vec<PlayerClient>,
	connected_bots: &mut Vec<BotClient>,
	local_bots: &mut Vec<LocalBot>,
	watchers: &mut Vec<WatcherClient>,
	updates: &mut mpsc::Receiver<Update>,
	planning_time_in_seconds: Option<u32>,
) -> Result<State, Error>
{
	while automaton.is_active()
	{
		let cset = automaton.act()?;
		broadcast(players, connected_bots, local_bots, watchers, cset)?;
	}

	// If players or bots are defeated, we no longer wait for them in the
	// planning phase.
	for player in players.into_iter()
	{
		if automaton.is_defeated(player.color)
		{
			player.is_defeated = true;
		}
	}
	for bot in connected_bots.into_iter()
	{
		if automaton.is_defeated(bot.color)
		{
			bot.is_defeated = true;
		}
	}
	for bot in local_bots.into_iter()
	{
		if automaton.is_defeated(bot.color)
		{
			bot.is_defeated = true;
		}
	}

	rest(lobby, automaton, players, connected_bots, watchers, updates).await?;

	// If the game has ended, we are done.
	// We waited with this check until all players have finished animating,
	// to avoid spoiling the outcome by changing ratings or posting to Discord.
	if automaton.is_gameover()
	{
		return Ok(State::Finished);
	}
	else if all_players_and_watchers_have_disconnected(players, watchers)
	{
		debug!("Abandoning game in lobby {} without clients...", lobby.id);
		return Ok(State::Abandoned);
	}

	// If all live players are disconnected during the resting phase,
	// the game cannot continue until at least one player reconnects
	// and has finished rejoining.
	ensure_live_players(
		lobby,
		automaton,
		players,
		connected_bots,
		watchers,
		updates,
	)
	.await?;

	let message = Message::Sync {
		time_remaining_in_seconds: planning_time_in_seconds,
	};
	for client in players.into_iter()
	{
		client.has_synced = false;
		client.handle.send(message.clone());
	}
	for client in watchers.into_iter()
	{
		client.has_synced = false;
		client.handle.send(message.clone());
	}

	let cset = automaton.hibernate()?;
	broadcast(players, connected_bots, local_bots, watchers, cset)?;

	// Allow the bots to calculate their next move.
	// FUTURE bots could prepare orders asynchronously
	// FUTURE start planning phase timer before bots prepare orders
	for bot in local_bots.into_iter()
	{
		bot.ai.prepare_orders();
	}

	sleep(lobby, automaton, players, connected_bots, watchers, updates).await?;

	let cset = automaton.awake()?;
	broadcast(players, connected_bots, local_bots, watchers, cset)?;

	stage(lobby, automaton, players, connected_bots, watchers, updates).await?;

	// Get submitted or calculated orders.
	for player in players.into_iter()
	{
		if let Some(orders) = player.submitted_orders.take()
		{
			automaton.receive(player.color, orders)?;
		}
	}
	for bot in connected_bots.into_iter()
	{
		if let Some(orders) = bot.submitted_orders.take()
		{
			automaton.receive(bot.color, orders)?;
		}
	}
	for bot in local_bots.into_iter()
	{
		if !bot.is_defeated
		{
			let orders = bot.ai.retrieve_orders()?;
			automaton.receive(bot.color, orders)?;
		}
	}

	let cset = automaton.prepare()?;
	broadcast(players, connected_bots, local_bots, watchers, cset)?;

	Ok(State::InProgress)
}

fn broadcast(
	players: &mut Vec<PlayerClient>,
	connected_bots: &mut Vec<BotClient>,
	local_bots: &mut Vec<LocalBot>,
	watchers: &mut Vec<WatcherClient>,
	cset: ChangeSet,
) -> Result<(), Error>
{
	for client in players
	{
		let changes = cset.get(client.color);
		let message = Message::Changes {
			changes,
			connected_bot: None,
		};
		client.handle.send(message);
	}

	for client in connected_bots
	{
		let changes = cset.get(client.color);
		let message = Message::Changes {
			changes,
			connected_bot: Some(client.connected_bot_metadata.clone()),
		};
		client.handle.send(message);
	}

	for bot in local_bots
	{
		let changes = cset.get(bot.color);
		bot.ai.receive(changes)?;
	}

	for client in watchers
	{
		let changes = cset.get(client.vision_level);
		let message = Message::Changes {
			changes,
			connected_bot: None,
		};
		client.handle.send(message);
	}

	Ok(())
}

fn all_players_and_watchers_have_disconnected(
	players: &mut Vec<PlayerClient>,
	watchers: &mut Vec<WatcherClient>,
) -> bool
{
	players.iter().find(|x| x.is_connected()).is_none()
		&& watchers.iter().find(|x| x.is_connected()).is_none()
}

async fn rest(
	lobby: &LobbyInfo,
	automaton: &mut Automaton,
	players: &mut Vec<PlayerClient>,
	bots: &mut Vec<BotClient>,
	watchers: &mut Vec<WatcherClient>,
	updates: &mut mpsc::Receiver<Update>,
) -> Result<(), Error>
{
	// Start the planning phase when all players (or all watchers if
	// if this is a replay lobby) are ready. Players and watchers can
	// reconnect in the resting phase while others are animating.
	// Players will wait until other players have fully rejoined,
	// and watchers wait until other watchers have rejoined.
	// Players will not wait for bots to sync, because bots do not animate
	// their changes.
	while !all_players_or_watchers_have_synced(players, watchers)
	{
		trace!("Waiting until all players/watchers have synced...");

		let update = match updates.recv().await
		{
			Some(update) => update,
			None => return Err(Error::Abandoned),
		};

		match update
		{
			Update::ForGame(Sub::Orders { client_id, .. }) =>
			{
				debug!("Ignoring orders from {} while resting", client_id);
			}
			Update::ForGame(Sub::BotOrders { client_id, .. }) =>
			{
				debug!("Ignoring orders from {} while resting", client_id);
			}
			Update::ForGame(Sub::Sync { client_id }) =>
			{
				for client in players.iter_mut()
				{
					if client.id == client_id
					{
						client.has_synced = true;
					}
				}
				for client in watchers.iter_mut()
				{
					if client.id == client_id
					{
						client.has_synced = true;
					}
				}
			}
			Update::ForGame(Sub::Resign { client_id }) =>
			{
				handle_resign(lobby, automaton, players, client_id).await?;
			}
			Update::Leave {
				client_id,
				mut general_chat,
			} =>
			{
				handle_leave(
					lobby.id,
					players,
					bots,
					watchers,
					client_id,
					&mut general_chat,
				)
				.await?;
			}
			Update::Join {
				client_id,
				client_user_id,
				client_username,
				client_handle,
				lobby_sendbuffer,
				mut general_chat,
				desired_metadata: _,
				invite,
			} =>
			{
				handle_join(
					lobby,
					automaton,
					players,
					watchers,
					RejoinPhase::Other,
					client_id,
					client_user_id,
					client_username,
					client_handle,
					lobby_sendbuffer,
					&mut general_chat,
					invite,
				)
				.await?;
			}
			Update::ForSetup(..) =>
			{}
			Update::Msg(message) =>
			{
				for client in players.iter_mut()
				{
					client.handle.send(message.clone());
				}
				for client in watchers.iter_mut()
				{
					client.handle.send(message.clone());
				}
			}
			Update::Pulse =>
			{}
		}
	}

	Ok(())
}

fn all_players_or_watchers_have_synced(
	players: &mut Vec<PlayerClient>,
	watchers: &mut Vec<WatcherClient>,
) -> bool
{
	if players.iter().find(|x| x.is_connected()).is_some()
	{
		players
			.iter()
			.find(|x| x.is_connected() && !x.has_synced)
			.is_none()
	}
	else
	{
		watchers
			.iter()
			.find(|x| x.is_connected() && !x.has_synced)
			.is_none()
	}
}

async fn ensure_live_players(
	lobby: &LobbyInfo,
	automaton: &mut Automaton,
	players: &mut Vec<PlayerClient>,
	bots: &mut Vec<BotClient>,
	watchers: &mut Vec<WatcherClient>,
	updates: &mut mpsc::Receiver<Update>,
) -> Result<(), Error>
{
	if players.is_empty()
	{
		return Ok(());
	}

	while !at_least_one_live_player(players)
	{
		trace!("Waiting for at least one live player...");

		let update = match updates.recv().await
		{
			Some(update) => update,
			None => return Err(Error::Abandoned),
		};

		match update
		{
			Update::ForGame(Sub::Orders { client_id, .. }) =>
			{
				debug!("Ignoring orders from {} after resting", client_id);
			}
			Update::ForGame(Sub::BotOrders { client_id, .. }) =>
			{
				debug!("Ignoring orders from {} after resting", client_id);
			}
			Update::ForGame(Sub::Sync { client_id }) =>
			{
				debug!("Ignoring sync from {} after resting", client_id);
			}
			Update::ForGame(Sub::Resign { client_id }) =>
			{
				handle_resign(lobby, automaton, players, client_id).await?;
			}
			Update::Leave {
				client_id,
				mut general_chat,
			} =>
			{
				handle_leave(
					lobby.id,
					players,
					bots,
					watchers,
					client_id,
					&mut general_chat,
				)
				.await?;
			}
			Update::Join {
				client_id,
				client_user_id,
				client_username,
				client_handle,
				lobby_sendbuffer,
				mut general_chat,
				desired_metadata: _,
				invite,
			} =>
			{
				handle_join(
					lobby,
					automaton,
					players,
					watchers,
					RejoinPhase::Other,
					client_id,
					client_user_id,
					client_username,
					client_handle,
					lobby_sendbuffer,
					&mut general_chat,
					invite,
				)
				.await?;
			}
			Update::ForSetup(..) =>
			{}
			Update::Msg(message) =>
			{
				for client in players.iter_mut()
				{
					client.handle.send(message.clone());
				}
				for client in watchers.iter_mut()
				{
					client.handle.send(message.clone());
				}
			}
			Update::Pulse =>
			{}
		}
	}

	Ok(())
}

fn at_least_one_live_player(players: &mut Vec<PlayerClient>) -> bool
{
	players
		.iter()
		.find(|x| !x.is_defeated && !x.is_retired() && x.is_connected())
		.is_some()
}

async fn sleep(
	lobby: &LobbyInfo,
	automaton: &mut Automaton,
	players: &mut Vec<PlayerClient>,
	connected_bots: &mut Vec<BotClient>,
	watchers: &mut Vec<WatcherClient>,
	updates: &mut mpsc::Receiver<Update>,
) -> Result<(), Error>
{
	let num_bots = lobby.num_bots;
	if too_few_potential_winners(players, num_bots)
	{
		trace!("Ending sleep early");
		return Ok(());
	}

	let start = Instant::now();
	let duration = match lobby.planning_time_in_seconds
	{
		Some(value) => Duration::from_secs(value as u64),
		None => Duration::from_secs(24 * 60 * 60),
	};
	let end = start + duration;

	// Wait for 1 second at the start of the planning phase, to prevent
	// blasting through the game in the time between the last human client
	// disconnecting and the lobby being detected as abandoned.
	// This does not affect the planning timer because `end` is already set
	// and we use `timeout_at` later on.
	timer::delay_for(Duration::from_secs(1)).await;

	// Start staging when either the timer runs out or all non-defeated
	// players are ready, or when all players except one are defeated
	// or have retired. Players and watchers can reconnect in the
	// planning phase if there is still time.
	while !all_players_have_submitted_orders(players, connected_bots)
	{
		trace!("Waiting until all players have submitted orders...");

		let update = match timer::timeout_at(end, updates.recv()).await
		{
			Ok(Some(update)) => update,
			Ok(None) => return Err(Error::Abandoned),
			Err(timer::Elapsed { .. }) =>
			{
				trace!("Planning phase ending...");
				break;
			}
		};

		match update
		{
			Update::ForGame(Sub::Orders { client_id, orders }) =>
			{
				if let Some(client) =
					players.iter_mut().find(|client| client.id == client_id)
				{
					client.submitted_orders = Some(orders);
				}
				else
				{
					warn!("Missing client {}", client_id);
				}
			}
			Update::ForGame(Sub::BotOrders {
				client_id,
				slot,
				orders,
			}) =>
			{
				if let Some(client) = connected_bots.iter_mut().find(|client| {
					client.id == client_id && client.slot == slot
				})
				{
					client.submitted_orders = Some(orders);
				}
				else
				{
					warn!("Missing bot client {}, slot {}", client_id, slot);
				}
			}
			Update::ForGame(Sub::Sync { client_id }) =>
			{
				debug!("Ignoring sync from {} while sleeping", client_id);
			}
			Update::ForGame(Sub::Resign { client_id }) =>
			{
				handle_resign(lobby, automaton, players, client_id).await?;

				if too_few_potential_winners(players, num_bots)
				{
					trace!("Ending sleep early");
					return Ok(());
				}
			}
			Update::Leave {
				client_id,
				mut general_chat,
			} =>
			{
				handle_leave(
					lobby.id,
					players,
					connected_bots,
					watchers,
					client_id,
					&mut general_chat,
				)
				.await?;
			}
			Update::Join {
				client_id,
				client_user_id,
				client_username,
				client_handle,
				lobby_sendbuffer,
				mut general_chat,
				desired_metadata: _,
				invite,
			} =>
			{
				let time_remaining_in_seconds = lobby
					.planning_time_in_seconds
					.map(|timer| timer - start.elapsed().as_secs() as u32);
				handle_join(
					lobby,
					automaton,
					players,
					watchers,
					RejoinPhase::Planning {
						time_remaining_in_seconds,
					},
					client_id,
					client_user_id,
					client_username,
					client_handle,
					lobby_sendbuffer,
					&mut general_chat,
					invite,
				)
				.await?;
			}
			Update::ForSetup(..) =>
			{}
			Update::Msg(message) =>
			{
				for client in players.iter_mut()
				{
					client.handle.send(message.clone());
				}
				for client in watchers.iter_mut()
				{
					client.handle.send(message.clone());
				}
			}
			Update::Pulse =>
			{}
		}
	}

	Ok(())
}

fn too_few_potential_winners(
	players: &mut Vec<PlayerClient>,
	num_bots: usize,
) -> bool
{
	let potentialwinners = players
		.iter()
		.filter(|x| !x.is_defeated && !x.is_retired())
		.count();

	potentialwinners + num_bots < 2
}

fn all_players_have_submitted_orders(
	players: &mut Vec<PlayerClient>,
	bots: &mut Vec<BotClient>,
) -> bool
{
	players
		.iter()
		.filter(|x| !x.is_defeated && !x.is_retired() && x.is_connected())
		.all(|x| x.submitted_orders.is_some())
		&& bots
			.iter()
			.filter(|x| !x.is_defeated && !x.is_retired() && x.is_connected())
			.all(|x| x.submitted_orders.is_some())
}

async fn stage(
	lobby: &LobbyInfo,
	automaton: &mut Automaton,
	players: &mut Vec<PlayerClient>,
	connected_bots: &mut Vec<BotClient>,
	watchers: &mut Vec<WatcherClient>,
	updates: &mut mpsc::Receiver<Update>,
) -> Result<(), Error>
{
	let start = Instant::now();
	let end = start + Duration::from_secs(10);

	// There is a 10 second grace period for anyone whose orders we
	// haven't received; they might have sent their orders before
	// receiving the staging announcement.
	// In particular this includes connected bots.
	while !all_players_have_submitted_orders(players, connected_bots)
	{
		trace!("Waiting until all players have staged orders...");

		let update = match timer::timeout_at(end, updates.recv()).await
		{
			Ok(Some(update)) => update,
			Ok(None) => return Err(Error::Abandoned),
			Err(timer::Elapsed { .. }) =>
			{
				trace!("Staging phase ending...");
				break;
			}
		};

		match update
		{
			Update::ForGame(Sub::Orders { client_id, orders }) =>
			{
				for client in players.iter_mut()
				{
					if client.id == client_id
					{
						client.submitted_orders = Some(orders);
						break;
					}
				}
			}
			Update::ForGame(Sub::BotOrders {
				client_id,
				slot,
				orders,
			}) =>
			{
				for client in connected_bots.iter_mut()
				{
					if client.id == client_id && client.slot == slot
					{
						client.submitted_orders = Some(orders);
						break;
					}
				}
			}

			Update::ForGame(Sub::Sync { client_id }) =>
			{
				debug!("Ignoring sync from {} while staging", client_id);
			}
			Update::ForGame(Sub::Resign { client_id }) =>
			{
				handle_resign(lobby, automaton, players, client_id).await?;
			}
			Update::Leave {
				client_id,
				mut general_chat,
			} =>
			{
				handle_leave(
					lobby.id,
					players,
					connected_bots,
					watchers,
					client_id,
					&mut general_chat,
				)
				.await?;
			}
			Update::Join {
				client_id,
				client_user_id,
				client_username,
				client_handle,
				lobby_sendbuffer,
				mut general_chat,
				desired_metadata: _,
				invite,
			} =>
			{
				handle_join(
					lobby,
					automaton,
					players,
					watchers,
					RejoinPhase::Other,
					client_id,
					client_user_id,
					client_username,
					client_handle,
					lobby_sendbuffer,
					&mut general_chat,
					invite,
				)
				.await?;
			}
			Update::ForSetup(..) =>
			{}
			Update::Msg(message) =>
			{
				for client in players.iter_mut()
				{
					client.handle.send(message.clone());
				}
				for client in watchers.iter_mut()
				{
					client.handle.send(message.clone());
				}
			}
			Update::Pulse =>
			{}
		}
	}

	Ok(())
}

async fn linger(
	lobby: &LobbyInfo,
	players: &mut Vec<PlayerClient>,
	bots: &mut Vec<BotClient>,
	watchers: &mut Vec<WatcherClient>,
	updates: &mut mpsc::Receiver<Update>,
) -> Result<(), Error>
{
	while let Some(update) = updates.recv().await
	{
		match update
		{
			Update::Leave {
				client_id,
				mut general_chat,
			} =>
			{
				handle_leave(
					lobby.id,
					players,
					bots,
					watchers,
					client_id,
					&mut general_chat,
				)
				.await?;
			}
			Update::Join { .. } =>
			{
				// The game has ended, no longer accept joins.
			}
			Update::ForSetup(..) =>
			{}
			Update::ForGame(..) =>
			{}
			Update::Msg(message) =>
			{
				for client in players.iter_mut()
				{
					client.handle.send(message.clone());
				}
				for client in watchers.iter_mut()
				{
					client.handle.send(message.clone());
				}
			}
			Update::Pulse =>
			{}
		}
	}

	Ok(())
}

async fn handle_join(
	lobby: &LobbyInfo,
	automaton: &mut Automaton,
	players: &mut Vec<PlayerClient>,
	watchers: &mut Vec<WatcherClient>,
	rejoin_phase: RejoinPhase,
	client_id: Keycode,
	client_user_id: UserId,
	client_username: String,
	client_handle: client::Handle,
	lobby_sendbuffer: mpsc::Sender<Update>,
	general_chat: &mut mpsc::Sender<chat::Update>,
	invite: Option<lobby::Invite>,
) -> Result<(), Error>
{
	match do_join(
		lobby,
		automaton,
		players,
		watchers,
		rejoin_phase,
		client_id,
		client_user_id,
		client_username.clone(),
		client_handle,
		lobby_sendbuffer,
		invite,
	)
	{
		Ok(RejoinResult::Joined) => (),
		Ok(RejoinResult::AccessDenied) => return Ok(()),
		Err(error) => return Err(error),
	}

	let update = chat::Update::JoinedLobby {
		client_id,
		lobby_id: lobby.id,
	};
	general_chat.send(update).await?;

	Ok(())
}

fn do_join(
	lobby: &LobbyInfo,
	automaton: &mut Automaton,
	players: &mut Vec<PlayerClient>,
	watchers: &mut Vec<WatcherClient>,
	rejoin_phase: RejoinPhase,
	client_id: Keycode,
	client_user_id: UserId,
	client_username: String,
	mut client_handle: client::Handle,
	lobby_sendbuffer: mpsc::Sender<Update>,
	invite: Option<lobby::Invite>,
) -> Result<RejoinResult, Error>
{
	let (disconnected_role, disconnected_player_color) = {
		if let Some(player) =
			players.iter_mut().find(|x| x.user_id == client_user_id)
		{
			if !player.is_defeated && !player.is_retired()
			{
				(Some(Role::Player), Some(player.color))
			}
			else
			{
				(Some(Role::Observer), None)
			}
		}
		else if let Some(watcher) =
			watchers.iter_mut().find(|x| x.user_id == client_user_id)
		{
			(Some(watcher.role), None)
		}
		else
		{
			(None, None)
		}
	};

	let is_invited = if let Some(invite) = invite
	{
		if invite.secret().lobby_id != lobby.id
		{
			return Ok(RejoinResult::AccessDenied);
		}
		players.iter().any(|x| x.handle.verify_invite(&invite))
			|| watchers.iter().any(|x| x.handle.verify_invite(&invite))
	}
	else
	{
		false
	};

	if disconnected_role.is_none() && !lobby.is_public && !is_invited
	{
		// TODO send joinlobby{}?
		return Ok(RejoinResult::AccessDenied);
	}

	// Tell the newcomer which users are already in the lobby.
	for other in players.iter().filter(|x| x.is_connected())
	{
		client_handle.send(Message::JoinLobby {
			lobby_id: Some(lobby.id),
			username: Some(other.username.clone()),
			invite: None,
		});
	}
	for other in watchers.iter().filter(|x| x.is_connected())
	{
		client_handle.send(Message::JoinLobby {
			lobby_id: Some(lobby.id),
			username: Some(other.username.clone()),
			invite: None,
		});
	}

	// Tell everyone who the newcomer is.
	let message = Message::JoinLobby {
		lobby_id: Some(lobby.id),
		username: Some(client_username.clone()),
		invite: None,
	};
	for other in players.iter_mut()
	{
		other.handle.send(message.clone());
	}
	for other in watchers.iter_mut()
	{
		other.handle.send(message.clone());
	}
	client_handle.send(message);

	// Tell the client certain pieces of lobby configuration so that the
	// Discord presence has the right info.
	client_handle.send(Message::ListMap {
		map_name: lobby.map_name.clone(),
		metadata: lobby.map_metadata.clone(),
	});
	client_handle.send(Message::PickMap {
		map_name: lobby.map_name.clone(),
	});
	// TODO tell them the recording if this is a replay lobby
	if let Some(_id) = lobby.challenge
	{
		// FUTURE this should be the challenge key of _id
		let challenge_key = challenge::get_current_key();
		client_handle.send(Message::PickChallenge { challenge_key });
	}

	// Describe the lobby to the client so that Discord presence is updated.
	let message = Message::ListLobby {
		lobby_id: lobby.id,
		lobby_name: lobby.name.clone(),
		metadata: lobby.description_metadata.clone(),
	};
	client_handle.send(message);

	// Tell the newcomer that the game has started.
	let role = disconnected_role.unwrap_or(Role::Observer);
	if let Some(color) = disconnected_player_color
	{
		client_handle.send(Message::Game {
			role: Some(role),
			player: Some(color),
			ruleset_name: Some(lobby.ruleset_name.clone()),
			timer_in_seconds: lobby.planning_time_in_seconds,
			difficulty: None,
			connected_bot: None,
		});
	}
	else
	{
		client_handle.send(Message::Game {
			role: Some(role),
			player: None,
			ruleset_name: Some(lobby.ruleset_name.clone()),
			timer_in_seconds: lobby.planning_time_in_seconds,
			difficulty: None,
			connected_bot: None,
		});
	}

	// Tell the newcomer the player colors, skins and if there is a challenge.
	for message in &lobby.initial_messages
	{
		client_handle.send(message.clone());
	}

	let vision = match disconnected_player_color
	{
		Some(color) => color,
		None => role.vision_level(),
	};
	let cset = automaton.rejoin(vision)?;
	let changes = cset.get(vision);

	client_handle.send(Message::ReplayWithAnimations {
		on_or_off: OnOrOff::Off,
	});
	client_handle.send(Message::Changes {
		changes,
		connected_bot: None,
	});
	client_handle.send(Message::ReplayWithAnimations {
		on_or_off: OnOrOff::On,
	});
	match rejoin_phase
	{
		RejoinPhase::Planning {
			time_remaining_in_seconds,
		} =>
		{
			client_handle.send(Message::Sync {
				time_remaining_in_seconds,
			});
		}
		RejoinPhase::Other =>
		{}
	}

	let update = client::Update::JoinedLobby {
		lobby_id: lobby.id,
		lobby: lobby_sendbuffer,
	};
	client_handle.notify(update);

	// Send secrets for Discord invites and Ask-to-Join.
	client_handle.generate_and_send_secrets(lobby.id);

	// Reconnect the player or observer, or add them as a new observer.
	if let Some(player) =
		players.iter_mut().find(|x| x.user_id == client_user_id)
	{
		player.id = client_id;
		debug_assert!(player.username == client_username);
		player.has_synced = false;
		player.handle = client_handle;
	}
	else if let Some(watcher) =
		watchers.iter_mut().find(|x| x.user_id == client_user_id)
	{
		watcher.id = client_id;
		debug_assert!(watcher.username == client_username);
		watcher.has_synced = false;
		watcher.handle = client_handle;
	}
	else
	{
		let newcomer = WatcherClient {
			id: client_id,
			user_id: client_user_id,
			username: client_username,
			handle: client_handle,

			role,
			vision_level: role.vision_level(),

			has_synced: false,
		};
		watchers.push(newcomer);
	}

	Ok(RejoinResult::Joined)
}

enum RejoinPhase
{
	Planning
	{
		time_remaining_in_seconds: Option<u32>,
	},
	Other,
}

enum RejoinResult
{
	Joined,
	AccessDenied,
}

async fn handle_resign(
	lobby: &LobbyInfo,
	automaton: &mut Automaton,
	players: &mut Vec<PlayerClient>,
	client_id: Keycode,
) -> Result<(), Error>
{
	let client = match players.iter_mut().find(|x| x.id == client_id)
	{
		Some(client) => client,
		None => return Err(Error::ClientGone { client_id }),
	};

	automaton.resign(client.color);

	retire(lobby, automaton, client).await
}

async fn retire(
	lobby: &LobbyInfo,
	automaton: &mut Automaton,
	client: &mut PlayerClient,
) -> Result<(), Error>
{
	let mut callback = match client.rating_callback.take()
	{
		Some(callback) => callback,
		None => return Ok(()),
	};

	// Because players may resign immediately after starting the game,
	// e.g. due to lobby mishaps or deciding not to play on a certain map,
	// the game is not rated until the game reaches the third action phase,
	// which is when the Automaton updates its _round variable.
	// Note that this means it is possible for someone to resign while unrated
	// even though their opponent keeps playing a rated game.
	let is_rated = automaton.current_round() >= 3;
	let result = PlayerResult {
		user_id: client.user_id,
		username: client.username.clone(),
		is_rated,
		is_victorious: !automaton.is_defeated(client.color),
		score: automaton.score(client.color),
		awarded_stars: automaton.award(client.color),
		match_type: lobby.match_type,
		challenge: lobby.challenge,
	};
	let update = rating::Update::GameResult(result);
	callback.send(update).await?;
	Ok(())
}

async fn handle_leave(
	lobby_id: Keycode,
	players: &mut Vec<PlayerClient>,
	bots: &mut Vec<BotClient>,
	watchers: &mut Vec<WatcherClient>,
	client_id: Keycode,
	general_chat: &mut mpsc::Sender<chat::Update>,
) -> Result<(), Error>
{
	let mut was_bot = false;
	for bot in bots
	{
		if bot.id == client_id
		{
			bot.handle.take();
			was_bot = true;
			// Do not break, as a bot client may play in multiple bot slots,
			// and all of them are now disconnected.
		}
	}
	if was_bot
	{
		return Ok(());
	}

	let (username, mut handle) = {
		if let Some(client) = players.iter_mut().find(|x| x.id == client_id)
		{
			(client.username.clone(), client.handle.take())
		}
		else if let Some(client) =
			watchers.iter_mut().find(|x| x.id == client_id)
		{
			(client.username.clone(), client.handle.take())
		}
		else
		{
			return Err(Error::ClientGone { client_id });
		}
	};

	let message = Message::LeaveLobby {
		lobby_id: Some(lobby_id),
		username: Some(username),
	};

	for client in players.iter_mut()
	{
		client.handle.send(message.clone());
	}
	for client in watchers.iter_mut()
	{
		client.handle.send(message.clone());
	}

	handle.send(message);

	let update = chat::Update::LeftLobby {
		lobby_id,
		client_id,
	};
	general_chat.send(update).await?;

	if players.iter().all(|x| x.handle.is_disconnected())
		&& watchers.iter().all(|x| x.handle.is_disconnected())
	{
		let update = chat::Update::DisbandLobby { lobby_id: lobby_id };
		general_chat.send(update).await?;
	}

	Ok(())
}

#[derive(Debug)]
pub enum Error
{
	Abandoned,
	MissingChallengeId,
	ClientGone
	{
		client_id: Keycode,
	},
	ResultDropped
	{
		error: mpsc::error::SendError<rating::Update>,
	},
	DiscordApiPostDropped
	{
		error: mpsc::error::SendError<discord_api::Post>,
	},
	GeneralChat
	{
		error: mpsc::error::SendError<chat::Update>,
	},
	Interface(automaton::InterfaceError),
}

impl From<mpsc::error::SendError<rating::Update>> for Error
{
	fn from(error: mpsc::error::SendError<rating::Update>) -> Self
	{
		Error::ResultDropped { error }
	}
}

impl From<mpsc::error::SendError<discord_api::Post>> for Error
{
	fn from(error: mpsc::error::SendError<discord_api::Post>) -> Self
	{
		Error::DiscordApiPostDropped { error }
	}
}

impl From<mpsc::error::SendError<chat::Update>> for Error
{
	fn from(error: mpsc::error::SendError<chat::Update>) -> Self
	{
		Error::GeneralChat { error }
	}
}

impl From<automaton::InterfaceError> for Error
{
	fn from(error: automaton::InterfaceError) -> Self
	{
		Error::Interface(error)
	}
}

impl fmt::Display for Error
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
	{
		match self
		{
			Error::Abandoned => write!(f, "{:#?}", &self),
			Error::MissingChallengeId => write!(f, "{:#?}", &self),
			Error::ClientGone { .. } => write!(f, "{:#?}", &self),
			Error::ResultDropped { .. } => write!(f, "{:#?}", &self),
			Error::DiscordApiPostDropped { .. } => write!(f, "{:#?}", &self),
			Error::GeneralChat { .. } => write!(f, "{:#?}", &self),
			Error::Interface(error) => error.fmt(f),
		}
	}
}

impl std::error::Error for Error {}

#[derive(Debug)]
pub struct PlayerResult
{
	pub user_id: UserId,
	pub username: String,
	pub is_rated: bool,
	pub is_victorious: bool,
	pub score: i32,
	pub awarded_stars: i32,

	pub match_type: MatchType,
	pub challenge: Option<ChallengeId>,
}

#[derive(Debug, Clone, Copy)]
pub enum MatchType
{
	Competitive,
	FriendlyOneVsOne,
	FreeForAll
	{
		num_non_bot_players: usize,
	},
	VersusAi,
	Unrated,
}
