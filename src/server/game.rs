/* ServerGame */

use crate::common::keycode::*;
use crate::logic::ai;
use crate::logic::automaton;
use crate::logic::automaton as metadata;
use crate::logic::automaton::Automaton;
use crate::logic::challenge;
use crate::logic::challenge::ChallengeId;
use crate::logic::change::ChangeSet;
use crate::logic::map;
use crate::logic::order::Order;
use crate::logic::player::PlayerColor;
use crate::server::botslot::Botslot;
use crate::server::chat;
use crate::server::client;
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
pub struct Bot
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
	pub bots: Vec<Bot>,
	pub watchers: Vec<WatcherClient>,
	pub map_name: String,
	pub map_metadata: map::Metadata,
	pub ruleset_name: String,
	pub planning_time_in_seconds: Option<u32>,
	pub challenge: Option<ChallengeId>,
	pub is_tutorial: bool,
	pub is_rated: bool,
	pub is_public: bool,
}

pub async fn run(
	setup: Setup,
	mut updates: mpsc::Receiver<Update>,
) -> Result<(), Error>
{
	let Setup {
		lobby_id,
		lobby_name,
		lobby_description_metadata,
		mut players,
		mut bots,
		mut watchers,
		map_name,
		map_metadata,
		ruleset_name,
		planning_time_in_seconds,
		challenge,
		is_tutorial,
		is_rated,
		is_public,
	} = setup;

	let mut playercolors = Vec::new();
	for player in &players
	{
		playercolors.push(player.color);
	}
	for bot in &bots
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
	for bot in &bots
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
		if is_tutorial
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
			});
		}
	}

	for client in &mut watchers
	{
		client.handle.send(Message::Game {
			role: Some(Role::Observer),
			player: None,
			ruleset_name: Some(ruleset_name.clone()),
			timer_in_seconds: planning_time_in_seconds,
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
	for bot in &mut bots
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
	if let Some(challenge_id) = challenge
	{
		automaton.set_challenge(challenge_id)?;

		initial_messages.push(Message::Briefing {
			briefing: challenge::load_briefing(challenge_id),
		});
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
		metadata_players.push(metadata::Player {
			color: player.color,
			username: player.username.clone(),
		});
	}
	let mut metadata_watchers = Vec::new();
	for watcher in &watchers
	{
		metadata_watchers.push(metadata::Watcher {
			username: watcher.username.clone(),
		});
	}
	let mut metadata_bots = Vec::new();
	for bot in &mut bots
	{
		let ai_metadata = bot.ai.metadata()?;
		metadata_bots.push(metadata::Bot {
			color: bot.color,
			ai_metadata,
		});
	}
	let metadata = automaton::Metadata {
		map_name: map_name.clone(),
		map_metadata: map_metadata,
		is_online: true,
		planning_time_in_seconds_or_zero: planning_time_in_seconds.unwrap_or(0),
		players: metadata_players,
		watchers: metadata_watchers,
		bots: metadata_bots,
	};

	// Load the map.
	let shuffleplayers = challenge.is_none()
		&& !map_name.contains("demo")
		&& !map_name.contains("tutorial");
	automaton.load(map_name, shuffleplayers)?;
	automaton.start_recording(metadata, lobby_id.to_string())?;

	// Is this game rated?
	let match_type = if !is_rated
	{
		MatchType::Unrated
	}
	// Is this a competitive 1v1 match with two humans?
	else if players.len() == 2 && bots.len() == 0
	{
		MatchType::Competitive
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
	let should_mention_on_discord = players.len() == 2 && bots.len() == 0;
	if should_mention_on_discord
	{
		// TODO mention on discord
	}

	let lobby_info = LobbyInfo {
		id: lobby_id,
		name: lobby_name,
		description_metadata: lobby_description_metadata,
		is_public,
		match_type,
		challenge,
		should_mention_on_discord,
		num_bots: bots.len(),
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
			&mut bots,
			&mut watchers,
			&mut updates,
			planning_time_in_seconds,
		)
		.await?;

		match state
		{
			State::InProgress => (),
			State::Finished => break,
		}
	}

	// Did we send a gameStarted post?
	if should_mention_on_discord
	{
		// TODO mention on discord
	}

	// If there are non-bot non-observer participants, adjust their ratings.
	for client in players.iter_mut()
	{
		retire(&lobby_info, &mut automaton, client).await?;
	}

	debug!("Game has finished in lobby {}; lingering...", lobby_id);
	linger(&lobby_info, &mut players, &mut watchers, &mut updates).await?;

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
}

#[derive(Debug)]
struct LobbyInfo
{
	id: Keycode,
	name: String,
	is_public: bool,
	match_type: MatchType,
	challenge: Option<ChallengeId>,
	should_mention_on_discord: bool,
	num_bots: usize,
	ruleset_name: String,
	planning_time_in_seconds: Option<u32>,
	description_metadata: LobbyMetadata,
	initial_messages: Vec<Message>,
}

#[derive(Debug, PartialEq, Eq)]
enum State
{
	InProgress,
	Finished,
}

async fn iterate(
	lobby: &LobbyInfo,
	automaton: &mut Automaton,
	players: &mut Vec<PlayerClient>,
	bots: &mut Vec<Bot>,
	watchers: &mut Vec<WatcherClient>,
	updates: &mut mpsc::Receiver<Update>,
	planning_time_in_seconds: Option<u32>,
) -> Result<State, Error>
{
	while automaton.is_active()
	{
		let cset = automaton.act()?;
		broadcast(players, bots, watchers, cset)?;
	}

	// If players are defeated, we no longer wait for them in the
	// planning phase.
	for player in players.into_iter()
	{
		if automaton.is_defeated(player.color)
		{
			player.is_defeated = true;
		}
	}
	// If bots are defeated, we no longer ask them for orders in the
	// action phase.
	for bot in bots.into_iter()
	{
		if automaton.is_defeated(bot.color)
		{
			bot.is_defeated = true;
		}
	}

	rest(lobby, automaton, players, watchers, updates).await?;

	// If the game has ended, we are done.
	// We waited with this check until all players have finished animating,
	// to avoid spoiling the outcome by changing ratings or posting to Discord.
	if automaton.is_gameover()
	{
		return Ok(State::Finished);
	}

	// If all live players are disconnected during the resting phase,
	// the game cannot continue until at least one player reconnects
	// and has finished rejoining.
	ensure_live_players(lobby, automaton, players, watchers, updates).await?;

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
	broadcast(players, bots, watchers, cset)?;

	// Allow the bots to calculate their next move.
	// FUTURE bots could prepare orders asynchronously
	// FUTURE start planning phase timer before bots prepare orders
	for bot in bots.into_iter()
	{
		bot.ai.prepare_orders();
	}

	sleep(lobby, automaton, players, watchers, updates).await?;

	let cset = automaton.awake()?;
	broadcast(players, bots, watchers, cset)?;

	stage(lobby, automaton, players, watchers, updates).await?;

	for player in players.into_iter()
	{
		if let Some(orders) = player.submitted_orders.take()
		{
			automaton.receive(player.color, orders)?;
		}
	}

	for bot in bots.into_iter()
	{
		if !bot.is_defeated
		{
			let orders = bot.ai.retrieve_orders()?;
			automaton.receive(bot.color, orders)?;
		}
	}

	let cset = automaton.prepare()?;
	broadcast(players, bots, watchers, cset)?;

	Ok(State::InProgress)
}

fn broadcast(
	players: &mut Vec<PlayerClient>,
	bots: &mut Vec<Bot>,
	watchers: &mut Vec<WatcherClient>,
	cset: ChangeSet,
) -> Result<(), Error>
{
	for client in players
	{
		let changes = cset.get(client.color);
		let message = Message::Changes { changes };
		client.handle.send(message);
	}

	for bot in bots
	{
		let changes = cset.get(bot.color);
		bot.ai.receive(changes)?;
	}

	for client in watchers
	{
		let changes = cset.get(client.vision_level);
		let message = Message::Changes { changes };
		client.handle.send(message);
	}

	Ok(())
}

async fn rest(
	lobby: &LobbyInfo,
	automaton: &mut Automaton,
	players: &mut Vec<PlayerClient>,
	watchers: &mut Vec<WatcherClient>,
	updates: &mut mpsc::Receiver<Update>,
) -> Result<(), Error>
{
	// Start the planning phase when all players (or all watchers if
	// if this is a replay lobby) are ready. Players and watchers can
	// reconnect in the resting phase while others are animating.
	// Players will wait until other players have fully rejoined,
	// and watchers wait until other watchers have rejoined.
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

	// Start staging when either the timer runs out or all non-defeated
	// players are ready, or when all players except one are defeated
	// or have retired. Players and watchers can reconnect in the
	// planning phase if there is still time.
	while !all_players_have_submitted_orders(players)
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
				for client in players.iter_mut()
				{
					if client.id == client_id
					{
						client.submitted_orders = Some(orders);
						break;
					}
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

fn all_players_have_submitted_orders(players: &mut Vec<PlayerClient>) -> bool
{
	players
		.iter()
		.filter(|x| !x.is_defeated && !x.is_retired() && x.is_connected())
		.all(|x| x.submitted_orders.is_some())
}

async fn stage(
	lobby: &LobbyInfo,
	automaton: &mut Automaton,
	players: &mut Vec<PlayerClient>,
	watchers: &mut Vec<WatcherClient>,
	updates: &mut mpsc::Receiver<Update>,
) -> Result<(), Error>
{
	let start = Instant::now();
	let end = start + Duration::from_secs(10);

	// There is a 10 second grace period for anyone whose orders we
	// haven't received; they might have sent their orders before
	// receiving the staging announcement.
	while !all_players_have_submitted_orders(players)
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
					watchers,
					client_id,
					&mut general_chat,
				)
				.await?;
			}
			Update::Join { .. } =>
			{
				// The game has ended, no longer accept joins.
				// TODO send joinlobby{}?
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
	)
	{
		Ok(RejoinResult::Joined) => (),
		Ok(RejoinResult::AccessDenied) => return Ok(()),
		Err(error) => return Err(error),
	}

	let message = Message::JoinLobby {
		lobby_id: Some(lobby.id),
		username: Some(client_username.clone()),
		metadata: None,
	};
	let update = chat::Update::Msg(message);
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
) -> Result<RejoinResult, Error>
{
	let (disconnected_role, disconnected_player_color) = {
		if let Some(player) =
			players.iter_mut().find(|x| x.user_id == client_user_id)
		{
			(Some(Role::Player), Some(player.color))
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

	// TODO check invitation
	let is_invited = false;

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
			metadata: None,
		});
	}
	for other in watchers.iter().filter(|x| x.is_connected())
	{
		client_handle.send(Message::JoinLobby {
			lobby_id: Some(lobby.id),
			username: Some(other.username.clone()),
			metadata: None,
		});
	}

	// Tell everyone who the newcomer is.
	let message = Message::JoinLobby {
		lobby_id: Some(lobby.id),
		username: Some(client_username.clone()),
		metadata: None,
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
		});
	}
	else
	{
		client_handle.send(Message::Game {
			role: Some(role),
			player: None,
			ruleset_name: Some(lobby.ruleset_name.clone()),
			timer_in_seconds: lobby.planning_time_in_seconds,
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
	client_handle.send(Message::Changes { changes });
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
		lobby: lobby_sendbuffer,
	};
	client_handle.notify(update);

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

	// Resigning while not yet being defeated is not rated
	// as a defeat until they reach the third action phase,
	// which is when the Automaton updates its _round variable.
	// Note that this means it is possible for someone to resign while unrated
	// even though their opponent keeps playing a rated game.
	let is_rated =
		automaton.is_defeated(client.color) || automaton.current_round() >= 3;
	let result = PlayerResult {
		user_id: client.user_id,
		username: client.username.clone(),
		is_rated,
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
	watchers: &mut Vec<WatcherClient>,
	client_id: Keycode,
	general_chat: &mut mpsc::Sender<chat::Update>,
) -> Result<(), Error>
{
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
	ClientGone
	{
		client_id: Keycode,
	},
	ResultDropped
	{
		error: mpsc::error::SendError<rating::Update>,
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
			Error::ClientGone { .. } => write!(f, "{:#?}", &self),
			Error::ResultDropped { .. } => write!(f, "{:#?}", &self),
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
	pub score: i32,
	pub awarded_stars: i32,

	pub match_type: MatchType,
	pub challenge: Option<ChallengeId>,
}

#[derive(Debug, Clone, Copy)]
pub enum MatchType
{
	Competitive,
	FreeForAll
	{
		num_non_bot_players: usize,
	},
	VersusAi,
	Unrated,
}
