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

use anyhow::{anyhow, Context};

use reqwest as http;

use enumset::*;

#[derive(Debug)]
pub struct Request
{
	pub account_identifier: String,
	pub token: String,
	pub metadata: JoinMetadata,
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
	Supporter,
	Guest,
	Bot,
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
			Ok(x) if (2..=8).contains(&x) =>
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
			username,
			unlocks,
			rating_data: rating::Data {
				rating: 0.0,
				stars: 0,
				recent_stars: 0,
			},
		};

		Ok(data)
	}
}

struct Connection
{
	http: http::Client,

	validate_session_url: http::Url,
	confirm_steam_user_url: http::Url,
	current_challenge_key: String,

	steam_api_config: SteamApiConfig,
	steam_ticket_url: http::Url,
	steam_player_summaries_url: http::Url,
	steam_app_ownership_url: http::Url,
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

		let validate_session_url = {
			let mut url = base_url.clone();
			url.set_path("validate_session.php");
			url
		};

		let confirm_steam_user_url = {
			let mut url = base_url.clone();
			url.set_path("api/v1/confirm_steam_user");
			url
		};

		let platform = Platform::current();
		let platformstring = serde_plain::to_string(&platform)?;
		let user_agent = format!(
			"epicinium-server/{} ({}; rust)",
			Version::current(),
			platformstring,
		);
		let http = http::Client::builder().user_agent(user_agent).build()?;

		let filename = settings
			.steam_web_key
			.as_ref()
			.ok_or_else(|| anyhow!("missing 'steam_web_key'"))?;
		let filename = std::path::Path::new(filename);
		let raw = std::fs::read_to_string(filename)?;
		let steam_api_config: SteamApiConfig = toml::from_str(&raw)
			.with_context(|| {
				format!("parsing steam web key from '{}'", filename.display())
			})?;

		let steam_base_url = match steam_api_config.api_type
		{
			SteamApiType::Publisher => "https://partner.steam-api.com",
			SteamApiType::User => "https://api.steampowered.com",
		};
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
		let steam_app_ownership_url = {
			let mut url = steam_base_url.clone();
			url.set_path("ISteamUser/CheckAppOwnership/v2/");
			url
		};

		Ok(Connection {
			http,
			validate_session_url,
			confirm_steam_user_url,
			current_challenge_key: challenge::get_current_key(),
			steam_api_config,
			steam_ticket_url,
			steam_player_summaries_url,
			steam_app_ownership_url,
		})
	}

	async fn login(&self, request: Request)
		-> Result<LoginData, ResponseStatus>
	{
		if request.account_identifier == "!steam"
		{
			self.login_with_steam(request.token, request.metadata).await
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
		if !request
			.account_identifier
			.chars()
			.all(|x| x.is_ascii_digit())
		{
			error!(
				"Login failed: refusing account id with non-digits '{}'",
				request.account_identifier
			);
			return Err(ResponseStatus::RequestMalformed);
		}

		let payload = ValidateSessionPayload {
			session_token: request.token,
			account_id_as_string: request.account_identifier,
			challenge_key: self.current_challenge_key.clone(),
		};

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
		ticket: String,
		metadata: JoinMetadata,
	) -> Result<LoginData, ResponseStatus>
	{
		let steam_id = self.get_steam_id(ticket).await?;

		let mut data = self.login_steam_user(steam_id, metadata).await?;

		match self.steam_api_config.api_type
		{
			SteamApiType::Publisher =>
			{
				self.check_app_ownership(steam_id).await?;
			}
			SteamApiType::User =>
			{
				warn!("Granting automatic beta access to everyone!");
			}
		}
		data.unlocks.insert(Unlock::BetaAccess);

		Ok(data)
	}

	async fn login_steam_user(
		&self,
		steam_id: SteamId,
		metadata: JoinMetadata,
	) -> Result<LoginData, ResponseStatus>
	{
		if let Some(merge_token) = metadata.merge_token
		{
			info!("Confirming steam user with merge token...");
			return self
				.confirm_steam_user(steam_id, None, Some(merge_token))
				.await;
		}
		else if let Some(username) = metadata.desired_username
		{
			if !is_valid_username(&username)
			{
				warn!("Rejecting invalid desired username '{}'.", username);
				return Err(ResponseStatus::UsernameRequiredInvalid);
			}
			info!("Confirming steam user with desired username...");
			return self
				.confirm_steam_user(steam_id, Some(username), None)
				.await;
		}

		// First see if this user already exists.
		info!("Confirming steam user (if exists)...");
		let result = self.confirm_steam_user(steam_id, None, None).await;
		match result
		{
			Ok(data) => Ok(data),
			Err(ResponseStatus::UsernameRequiredNoUser) =>
			{
				// If not, get the current Steam persona name and use that
				// as the username for the new account.
				let username = self.get_steam_persona_name(steam_id).await?;
				if !is_valid_username(&username)
				{
					warn!("Rejecting persona name '{}' as username.", username);
					// We use a "milder" response status here because it is not
					// the user's fault that their Steam persona name is not
					// a valid Epicinium username.
					return Err(ResponseStatus::UsernameRequiredNoUser);
				}

				info!("Confirming steam user with personaname as username...");
				self.confirm_steam_user(steam_id, Some(username), None)
					.await
			}
			Err(err) => Err(err),
		}
	}

	async fn get_steam_id(
		&self,
		ticket: String,
	) -> Result<SteamId, ResponseStatus>
	{
		let payload = SteamAuthenticateUserTicketParameters {
			app_id: self.steam_api_config.app_id,
			ticket,
		};

		let response: SteamAuthenticateUserTicketResponse = self
			.http
			.get(self.steam_ticket_url.clone())
			.header("x-webapi-key", self.steam_api_config.web_key.clone())
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
			.header("x-webapi-key", self.steam_api_config.web_key.clone())
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

	async fn check_app_ownership(
		&self,
		steam_id: SteamId,
	) -> Result<(), ResponseStatus>
	{
		let payload = SteamCheckAppOwnershipParameters {
			steam_id,
			app_id: self.steam_api_config.app_id,
		};

		let response: SteamCheckAppOwnershipResponse = self
			.http
			.get(self.steam_app_ownership_url.clone())
			.header("x-webapi-key", self.steam_api_config.web_key.clone())
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

		if response.inner.result == SteamResult::Ok && response.inner.owns_app
		{
			Ok(())
		}
		else
		{
			Err(ResponseStatus::KeyRequired)
		}
	}

	async fn confirm_steam_user(
		&self,
		steam_id: SteamId,
		desired_username: Option<String>,
		merge_token: Option<String>,
	) -> Result<LoginData, ResponseStatus>
	{
		let payload = ConfirmSteamUserPayload {
			steam_id,
			desired_username,
			merge_token,
			challenge_key: self.current_challenge_key.clone(),
		};

		let response: LoginResponse = self
			.http
			.post(self.confirm_steam_user_url.clone())
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
}

#[derive(Debug, Serialize)]
struct ValidateSessionPayload
{
	#[serde(rename = "token")]
	session_token: String,

	#[serde(rename = "id")]
	account_id_as_string: String,

	challenge_key: String,
}

#[derive(Debug, Serialize)]
struct ConfirmSteamUserPayload
{
	#[serde(rename = "steam_id_as_string")]
	steam_id: SteamId,

	desired_username: Option<String>,

	merge_token: Option<String>,

	challenge_key: String,
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

#[derive(Debug, Serialize)]
struct SteamCheckAppOwnershipParameters
{
	#[serde(rename = "appid")]
	app_id: u32,

	#[serde(rename = "steamid")]
	steam_id: SteamId,
}

// Omitting certain extra fields.
#[derive(Debug, Deserialize)]
struct SteamCheckAppOwnershipResponse
{
	#[serde(rename = "appownership")]
	inner: SteamCheckAppOwnershipResponseInner,
}

// Omitting certain extra fields.
#[derive(Debug, Deserialize)]
struct SteamCheckAppOwnershipResponseInner
{
	result: SteamResult,

	#[serde(rename = "ownsapp")]
	owns_app: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
enum SteamResult
{
	#[serde(rename = "OK")]
	Ok,
}

fn is_valid_username(username: &str) -> bool
{
	username.len() >= 3
		&& username.len() <= 36
		&& username.is_ascii()
		&& username.chars().all(is_valid_username_char)
}

fn is_valid_username_char(x: char) -> bool
{
	match x
	{
		// not space
		// not !"#$%'()*+,
		'-' | '.' => true,
		// not /
		'0'..='9' => true,
		// not :;<=>?@
		'a'..='z' => true,
		// not [\]^
		'_' => true,
		// not `
		'A'..='Z' => true,
		// not {|}
		'~' => true,
		_ => false,
	}
}

#[derive(Debug, Deserialize)]
struct SteamApiConfig
{
	app_id: u32,
	api_type: SteamApiType,
	web_key: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
enum SteamApiType
{
	User,
	Publisher,
}
