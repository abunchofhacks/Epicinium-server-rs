/* Server::Login */

use crate::common::keycode::*;
use crate::common::platform::*;
use crate::common::version::*;
use crate::logic::challenge;
use crate::server::message::*;
use crate::server::rating;
use crate::server::settings::*;

use log::*;

use serde_aux::field_attributes::deserialize_number_from_string;
use serde_derive::{Deserialize, Serialize};
use serde_json::json;

use anyhow::anyhow;

use reqwest as http;

use enumset::*;

#[derive(Debug)]
pub struct Request
{
	pub account_identifier: String,
	pub token: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(u64);

#[derive(Debug, Clone, Deserialize)]
pub struct LoginData
{
	pub user_id: UserId,
	pub username: String,

	#[serde(rename = "labeled_unlocks")]
	pub unlocks: EnumSet<Unlock>,

	#[serde(flatten)]
	pub rating_data: rating::Data,
}

#[derive(EnumSetType, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[enumset(serialize_as_list)]
pub enum Unlock
{
	Dev,
	BetaAccess,
	Guest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SteamId
{
	#[serde(deserialize_with = "deserialize_number_from_string")]
	as_u64: u64,
}

impl std::fmt::Display for SteamId
{
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
	{
		write!(f, "{}", self.as_u64)
	}
}

pub struct Server
{
	connection: Option<Connection>,
}

pub fn connect(settings: &Settings) -> Result<Server, anyhow::Error>
{
	if settings.login_server.is_some()
		|| (!cfg!(feature = "version-is-dev")
			&& (!cfg!(debug_assertions) || cfg!(feature = "candidate")))
	{
		let connection = Connection::open(settings)?;
		Ok(Server {
			connection: Some(connection),
		})
	}
	else
	{
		Ok(Server { connection: None })
	}
}

impl Server
{
	pub async fn login(
		&self,
		request: Request,
	) -> Result<LoginData, ResponseStatus>
	{
		match &self.connection
		{
			Some(ref connection) => connection.login(request).await,
			None => self.dev_login(request),
		}
	}
}

impl Server
{
	fn dev_login(&self, request: Request) -> Result<LoginData, ResponseStatus>
	{
		let user_id;
		let username;
		let unlocks: EnumSet<Unlock>;
		match request.token.parse::<u8>()
		{
			Ok(1) =>
			{
				user_id = UserId(1);
				username = "Alice".to_string();
				unlocks = enum_set!(Unlock::BetaAccess | Unlock::Dev);
			}
			Ok(x) if x >= 2 && x <= 8 =>
			{
				user_id = UserId(x as u64);
				const NAMES: [&str; 7] =
					["Bob", "Carol", "Dave", "Emma", "Frank", "Gwen", "Harold"];
				username = NAMES[(x - 2) as usize].to_string();
				unlocks = enum_set!(Unlock::BetaAccess);
			}
			_ =>
			{
				let key: u16 = rand::random();
				let serial: u64 = rand::random();
				let id = keycode(key, serial);
				user_id = UserId(id.0 | 0xF000000000000000);
				username = format!("{}", id);
				unlocks = enum_set!(Unlock::BetaAccess | Unlock::Dev);
			}
		}

		let data = LoginData {
			user_id,
			username: username,
			unlocks: unlocks,
			rating_data: rating::Data {
				rating: 0.0,
				stars: 0,
				recent_stars: 0,
			},
		};

		Ok(data)
	}
}

const STEAM_APP_ID: u32 = 1286730;

struct Connection
{
	http: http::Client,

	validate_session_url: http::Url,
	current_challenge_key: String,

	steam_web_key: String,
	steam_ticket_url: http::Url,
	steam_player_summaries_url: http::Url,
}

impl Connection
{
	fn open(settings: &Settings) -> Result<Connection, anyhow::Error>
	{
		let url = settings
			.login_server
			.as_ref()
			.ok_or_else(|| anyhow!("missing 'login_server'"))?;
		let base_url = http::Url::parse(url)?;

		let mut validate_session_url = base_url;
		validate_session_url.set_path("validate_session.php");

		let platform = Platform::current();
		let platformstring = serde_plain::to_string(&platform)?;
		let user_agent = format!(
			"epicinium-server/{} ({}; rust)",
			Version::current(),
			platformstring,
		);
		let http = http::Client::builder().user_agent(user_agent).build()?;

		// TODO read from file
		let steam_web_key = "9AEC9FF2DCCE17637BBD14B700DD54BB".to_string();

		// TODO https://partner.steam-api.com
		let steam_base_url = "https://api.steampowered.com";
		let steam_base_url = http::Url::parse(steam_base_url)?;
		let steam_ticket_url = {
			let mut url = steam_base_url.clone();
			url.set_path("ISteamUserAuth/AuthenticateUserTicket/v1/");
			url
		};
		let steam_player_summaries_url = {
			let mut url = steam_base_url.clone();
			url.set_path("ISteamUser/GetPlayerSummaries/v2/");
			url
		};

		Ok(Connection {
			http,
			validate_session_url,
			current_challenge_key: challenge::get_current_key(),
			steam_web_key,
			steam_ticket_url,
			steam_player_summaries_url,
		})
	}

	async fn login(&self, request: Request)
		-> Result<LoginData, ResponseStatus>
	{
		if request.account_identifier == "!steam"
		{
			self.login_with_steam(request).await
		}
		else
		{
			self.login_live(request).await
		}
	}

	async fn login_live(
		&self,
		request: Request,
	) -> Result<LoginData, ResponseStatus>
	{
		let payload = json!({
			"token": request.token,
			"id": request.account_identifier,
			"challenge_key": self.current_challenge_key,
		});

		let response: LoginResponse = self
			.http
			.post(self.validate_session_url.clone())
			.json(&payload)
			.send()
			.await
			.map_err(|error| {
				error!("Login failed: {:?}", error);

				ResponseStatus::ConnectionFailed
			})?
			.error_for_status()
			.map_err(|error| {
				error!("Login failed: {:?}", error);

				ResponseStatus::ConnectionFailed
			})?
			.json()
			.await
			.map_err(|error| {
				error!(
					"Received malformed response from login server: {}",
					error
				);
				ResponseStatus::ResponseMalformed
			})?;

		debug!("Got a response from login server: {:?}", response);

		if response.status == ResponseStatus::Success
		{
			response.data.ok_or(ResponseStatus::ResponseMalformed)
		}
		else
		{
			Err(response.status)
		}
	}

	async fn login_with_steam(
		&self,
		request: Request,
	) -> Result<LoginData, ResponseStatus>
	{
		let steam_id = self.get_steam_id(request).await?;

		let persona_name = self.get_steam_persona_name(steam_id).await?;

		debug!("Got {} aka '{}'.", steam_id, persona_name);

		// TODO CheckAppOwnership

		// TODO create account and epicinium_user if necessary
		Err(ResponseStatus::UnknownError)
	}

	async fn get_steam_id(
		&self,
		request: Request,
	) -> Result<SteamId, ResponseStatus>
	{
		let payload = SteamAuthenticateUserTicketParameters {
			app_id: STEAM_APP_ID,
			ticket: request.token,
		};

		let response: SteamAuthenticateUserTicketResponse = self
			.http
			.get(self.steam_ticket_url.clone())
			.header("x-webapi-key", self.steam_web_key.clone())
			.query(&payload)
			.send()
			.await
			.map_err(|error| {
				error!("Login failed: {:?}", error);

				ResponseStatus::ConnectionFailed
			})?
			.error_for_status()
			.map_err(|error| {
				error!("Login failed: {:?}", error);

				ResponseStatus::ConnectionFailed
			})?
			.json()
			.await
			.map_err(|error| {
				error!("Received malformed response from Steam: {}", error);
				ResponseStatus::ResponseMalformed
			})?;

		debug!("Got a response from Steam: {:?}", response);

		let data = response.inner.params;
		if data.is_banned_by_vac || data.is_banned_by_publisher
		{
			info!("Refusing user with banned Steam ID {}.", data.steam_id);
			// TODO notify login-server to flag this account as banned?
			return Err(ResponseStatus::AccountDisabled);
		}
		else if data.result != SteamResult::Ok
		{
			warn!("Unknown 'result' in {:?}", data);
			return Err(ResponseStatus::UnknownError);
		}

		Ok(data.steam_id)
	}

	async fn get_steam_persona_name(
		&self,
		steam_id: SteamId,
	) -> Result<String, ResponseStatus>
	{
		let payload = SteamGetPlayerSummariesParameters {
			steam_ids_as_string: format!("{}", steam_id),
		};

		let response: SteamGetPlayerSummariesResponse = self
			.http
			.get(self.steam_player_summaries_url.clone())
			.header("x-webapi-key", self.steam_web_key.clone())
			.query(&payload)
			.send()
			.await
			.map_err(|error| {
				error!("Login failed: {:?}", error);

				ResponseStatus::ConnectionFailed
			})?
			.error_for_status()
			.map_err(|error| {
				error!("Login failed: {:?}", error);

				ResponseStatus::ConnectionFailed
			})?
			.json()
			.await
			.map_err(|error| {
				error!("Received malformed response from Steam: {}", error);
				ResponseStatus::ResponseMalformed
			})?;

		debug!("Got a response from Steam: {:?}", response);

		let summary = response
			.inner
			.players
			.into_iter()
			.find(|x| x.steam_id == steam_id)
			.ok_or_else(|| {
				error!("Received malformed response from Steam: no summary");
				ResponseStatus::ResponseMalformed
			})?;

		Ok(summary.persona_name)
	}
}

#[derive(Debug, Clone, Deserialize)]
struct LoginResponse
{
	status: ResponseStatus,

	#[serde(flatten)]
	data: Option<LoginData>,
}

#[derive(Debug, Serialize)]
struct SteamAuthenticateUserTicketParameters
{
	#[serde(rename = "appid")]
	app_id: u32,

	ticket: String,
}

#[derive(Debug, Deserialize)]
struct SteamAuthenticateUserTicketResponse
{
	#[serde(rename = "response")]
	inner: SteamAuthenticateUserTicketResponseInner,
}

#[derive(Debug, Deserialize)]
struct SteamAuthenticateUserTicketResponseInner
{
	params: SteamAuthenticateUserTicketResponseInnerParams,
}

#[derive(Debug, Deserialize)]
struct SteamAuthenticateUserTicketResponseInnerParams
{
	result: SteamResult,

	#[serde(rename = "steamid")]
	steam_id: SteamId,

	#[serde(rename = "ownersteamid")]
	owner_steam_id: SteamId,

	#[serde(rename = "vacbanned")]
	is_banned_by_vac: bool,

	#[serde(rename = "publisherbanned")]
	is_banned_by_publisher: bool,
}

#[derive(Debug, Serialize)]
struct SteamGetPlayerSummariesParameters
{
	#[serde(rename = "steamids")]
	steam_ids_as_string: String,
}

#[derive(Debug, Deserialize)]
struct SteamGetPlayerSummariesResponse
{
	#[serde(rename = "response")]
	inner: SteamGetPlayerSummariesResponseInner,
}

#[derive(Debug, Deserialize)]
struct SteamGetPlayerSummariesResponseInner
{
	players: Vec<SteamPlayerSummary>,
}

// Omitting certain extra fields.
#[derive(Debug, Deserialize)]
struct SteamPlayerSummary
{
	#[serde(rename = "steamid")]
	steam_id: SteamId,

	#[serde(rename = "personaname")]
	persona_name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
enum SteamResult
{
	#[serde(rename = "OK")]
	Ok,
}
