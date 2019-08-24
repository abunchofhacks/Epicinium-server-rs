/* LoginServer */

use common::keycode::*;
use server::message::*;
use server::settings::*;

use std::error;

use futures::future;
use futures::future::{Either, Future};

use reqwest as http;

#[derive(Debug)]
pub struct LoginRequest
{
	pub token: String,
	pub account_id: String,
}

pub struct LoginServer
{
	connection: Option<Connection>,
}

impl LoginServer
{
	pub fn connect(
		settings: &Settings,
	) -> Result<LoginServer, Box<dyn error::Error>>
	{
		let connection = Connection::open(settings)?;

		Ok(LoginServer {
			connection: connection,
		})
	}

	pub fn login(
		&self,
		request: LoginRequest,
	) -> impl Future<Item = LoginData, Error = ResponseStatus> + Send
	{
		match &self.connection
		{
			Some(connection) => Either::A(connection.login(request)),
			None => Either::B(self.dev_login(request)),
		}
	}

	fn dev_login(
		&self,
		request: LoginRequest,
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
	http: http::async::Client,
	validate_session_url: http::Url,
}

impl Connection
{
	fn open(
		settings: &Settings,
	) -> Result<Option<Connection>, Box<dyn error::Error>>
	{
		let url = if !cfg!(debug_assertions) || cfg!(feature = "candidate")
		{
			settings.get_login_server()?
		}
		else
		{
			match settings.login_server()
			{
				Some(x) => x,
				None =>
				{
					return Ok(None);
				}
			}
		};

		let base_url = http::Url::parse(url)?;
		let mut validate_session_url = base_url;
		validate_session_url.set_path("validate_session.php");

		Ok(Some(Connection {
			http: http::async::Client::new(),
			validate_session_url: validate_session_url,
		}))
	}

	fn login(
		&self,
		request: LoginRequest,
	) -> impl Future<Item = LoginData, Error = ResponseStatus> + Send
	{
		let payload = json!({
			"id": request.account_id,
			"token": request.token,
			// TODO "challenge_key": challenge_key,
		});

		self.http
			.post(self.validate_session_url.clone())
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
