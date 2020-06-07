/* Server::Login */

use crate::common::keycode::*;
use crate::common::platform::*;
use crate::common::version::*;
use crate::logic::challenge;
use crate::server::message::*;
use crate::server::settings::*;

use std::error;

use reqwest as http;

use enumset::*;

#[derive(Debug)]
pub struct Request
{
	pub token: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
pub struct UserId(u64);

#[derive(Debug, Clone, Deserialize)]
pub struct LoginData
{
	pub user_id: UserId,
	pub username: String,
	pub unlocks: EnumSet<Unlock>,
	pub rating: f32,
	pub stars: i32,
	pub recent_stars: i32,
}

pub struct Server
{
	connection: Option<Connection>,
}

pub fn connect(settings: &Settings) -> Result<Server, Box<dyn error::Error>>
{
	if settings.login_server().is_some()
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
			rating: 0.0,
			stars: 0,
			recent_stars: 0,
		};

		Ok(data)
	}
}

struct Connection
{
	http: http::Client,
	validate_session_url: http::Url,
	current_challenge_key: String,
}

impl Connection
{
	fn open(settings: &Settings) -> Result<Connection, Box<dyn error::Error>>
	{
		let url = settings.get_login_server()?;
		let base_url = http::Url::parse(url)?;

		let mut validate_session_url = base_url;
		validate_session_url.set_path("validate_session.php");

		let platform = Platform::current();
		let platformstring = serde_plain::to_string(&platform)?;
		let user_agent = format!(
			"epicinium-server/{} ({}; rust)",
			Version::current().to_string(),
			platformstring,
		);
		let http = http::Client::builder().user_agent(user_agent).build()?;

		Ok(Connection {
			http,
			validate_session_url,
			current_challenge_key: challenge::get_current_key(),
		})
	}

	async fn login(&self, request: Request)
		-> Result<LoginData, ResponseStatus>
	{
		let payload = json!({
			"token": request.token,
			"challenge_key": self.current_challenge_key,
		});

		let response: LoginResponse = self
			.http
			.post(self.validate_session_url.clone())
			.json(&payload)
			.send()
			.await
			.map_err(|error| {
				eprintln!("Login failed: {:?}", error);

				ResponseStatus::ConnectionFailed
			})?
			.error_for_status()
			.map_err(|error| {
				eprintln!("Login failed: {:?}", error);

				ResponseStatus::ConnectionFailed
			})?
			.json()
			.await
			.map_err(|error| {
				eprintln!(
					"Received malformed response from login server: {}",
					error
				);
				ResponseStatus::ResponseMalformed
			})?;

		println!("Got a response from login server: {:?}", response);

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

#[derive(Debug, Clone, Deserialize)]
struct LoginResponse
{
	status: ResponseStatus,

	#[serde(flatten)]
	data: Option<LoginData>,
}
