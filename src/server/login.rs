/* Server::Login */

use crate::common::keycode::*;
use crate::common::platform::*;
use crate::common::version::*;
use crate::server::message::*;
use crate::server::settings::*;

use std::error;

use futures::future;
use futures::future::Either;
use futures::Future;

use reqwest as http;

#[derive(Debug)]
pub struct Request
{
	pub token: String,
	pub account_id: String,
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
	pub fn login(
		&self,
		request: Request,
	) -> impl Future<Item = LoginData, Error = ResponseStatus> + Send
	{
		match &self.connection
		{
			Some(ref connection) => Either::A(connection.login(request)),
			None => Either::B(self.dev_login(request)),
		}
	}
}

impl Server
{
	fn dev_login(
		&self,
		request: Request,
	) -> impl Future<Item = LoginData, Error = ResponseStatus> + Send
	{
		let username;
		let unlocks;
		match request.account_id.parse::<u8>()
		{
			Ok(1) =>
			{
				username = "Alice".to_string();
				unlocks =
					vec![unlock_id(Unlock::Access), unlock_id(Unlock::Dev)];
			}
			Ok(x) if x >= 2 && x <= 8 =>
			{
				const NAMES: [&str; 7] =
					["Bob", "Carol", "Dave", "Emma", "Frank", "Gwen", "Harold"];
				username = NAMES[(x - 2) as usize].to_string();
				unlocks = vec![unlock_id(Unlock::Access)];
			}
			_ =>
			{
				let key: u16 = rand::random();
				let serial: u64 = rand::random();
				let id = keycode(key, serial);
				username = format!("{}", id);
				unlocks =
					vec![unlock_id(Unlock::Access), unlock_id(Unlock::Dev)];
			}
		}

		let data = LoginData {
			username: username,
			unlocks: unlocks,
			rating: 0.0,
			stars: 0,
			recent_stars: 0,
		};

		future::ok(data)
	}
}

struct Connection
{
	http: http::r#async::Client,
	user_agent: http::header::HeaderValue,
	validate_session_url: http::Url,
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
		let uastring = format!(
			"epicinium-server/{} ({}; rust)",
			Version::current().to_string(),
			platformstring,
		);
		let user_agent: http::header::HeaderValue = uastring.parse()?;

		Ok(Connection {
			http: http::r#async::Client::new(),
			user_agent,
			validate_session_url,
		})
	}

	fn login(
		&self,
		request: Request,
	) -> impl Future<Item = LoginData, Error = ResponseStatus> + Send
	{
		let payload = json!({
			"id": request.account_id,
			"token": request.token,
			// TODO "challenge_key": challenge_key,
		});

		self.http
			.post(self.validate_session_url.clone())
			.header(http::header::USER_AGENT, self.user_agent.clone())
			.json(&payload)
			.send()
			.map_err(|error| {
				eprintln!("Login failed: {}", error);

				ResponseStatus::ConnectionFailed
			})
			.and_then(|response| {
				if response.status().is_success()
				{
					Ok(response)
				}
				else
				{
					Err(ResponseStatus::ConnectionFailed)
				}
			})
			.and_then(|mut response| {
				response.json().map_err(|error| {
					eprintln!(
						"Received malformed response from login server: {}",
						error
					);
					ResponseStatus::ResponseMalformed
				})
			})
			.and_then(|response: LoginResponse| {
				println!("Got a response from login server: {:?}", response);

				if response.status == ResponseStatus::Success
				{
					response.data.ok_or(ResponseStatus::ResponseMalformed)
				}
				else
				{
					Err(response.status)
				}
			})
	}
}
