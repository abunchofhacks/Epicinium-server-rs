/* Server::Login */

use common::keycode::*;
use server::message::*;
use server::settings::*;

use std::error;
use std::fmt;
use std::io;

use futures::future;
use futures::future::Either;
use futures::{Future, IntoFuture};

use reqwest as http;

#[derive(Debug)]
pub struct Request
{
	pub token: String,
	pub account_id: String,
}

#[derive(Clone, Deserialize, Debug)]
pub struct RegistrationResponse
{
	pub port: u16,
}

pub struct Server
{
	connection: ConnectionImpl,
}

enum ConnectionImpl
{
	Http(Connection),
	Dummy(Dummy),
}

pub fn connect(settings: &Settings) -> Result<Server, Box<dyn error::Error>>
{
	if settings.login_server().is_some()
		|| (!cfg!(feature = "version-is-dev")
			&& (!cfg!(debug_assertions) || cfg!(feature = "candidate")))
	{
		let connection = Connection::open(settings)?;
		Ok(Server {
			connection: ConnectionImpl::Http(connection),
		})
	}
	else
	{
		let dummy = Dummy::open(settings)?;
		Ok(Server {
			connection: ConnectionImpl::Dummy(dummy),
		})
	}
}

impl Server
{
	pub fn login(
		&self,
		request: Request,
	) -> impl Future<Item = LoginData, Error = ResponseStatus> + Send
	{
		match self.connection
		{
			ConnectionImpl::Http(ref connection) =>
			{
				Either::A(connection.login(request))
			}
			ConnectionImpl::Dummy(ref connection) =>
			{
				Either::B(connection.login(request))
			}
		}
	}

	pub fn register_server(
		&self,
	) -> impl Future<Item = RegistrationResponse, Error = ApiError> + Send
	{
		match self.connection
		{
			ConnectionImpl::Http(ref connection) =>
			{
				Either::A(connection.register_server())
			}
			ConnectionImpl::Dummy(ref connection) =>
			{
				Either::B(connection.register_server().into_future())
			}
		}
	}
}

struct Dummy
{
	port: u16,
}

impl Dummy
{
	fn open(settings: &Settings) -> Result<Dummy, Box<dyn error::Error>>
	{
		let port = settings.get_port()?;

		Ok(Dummy { port })
	}

	fn login(
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

	fn register_server(&self) -> Result<RegistrationResponse, ApiError>
	{
		Ok(RegistrationResponse { port: self.port })
	}
}

struct Connection
{
	http: http::async::Client,
	register_server_url: http::Url,
	validate_session_url: http::Url,
}

impl Connection
{
	fn open(settings: &Settings) -> Result<Connection, Box<dyn error::Error>>
	{
		let url = settings.get_login_server()?;
		let base_url = http::Url::parse(url)?;

		let mut register_server_url = base_url.clone();
		register_server_url.set_path("api/v1/servers");

		let mut validate_session_url = base_url;
		validate_session_url.set_path("validate_session.php");

		Ok(Connection {
			http: http::async::Client::new(),
			register_server_url,
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

	fn register_server(
		&self,
	) -> impl Future<Item = RegistrationResponse, Error = ApiError> + Send
	{
		self.http
			.post(self.register_server_url.clone())
			.send()
			.map_err(|error| {
				eprintln!("Failed to register server: {}", error);
				error.into()
			})
			.and_then(|response| {
				response.error_for_status().map_err(|e| e.into())
			})
			.and_then(|mut response| {
				response.json().map_err(|error| {
					eprintln!(
						"Received malformed response from login server: {}",
						error
					);
					error.into()
				})
			})
	}
}

#[derive(Debug)]
pub enum ApiError
{
	Request
	{
		error: http::Error
	},
	Response
	{
		error: io::Error
	},
}

impl From<http::Error> for ApiError
{
	fn from(error: http::Error) -> Self
	{
		ApiError::Request { error }
	}
}

impl From<io::Error> for ApiError
{
	fn from(error: io::Error) -> Self
	{
		ApiError::Response { error }
	}
}

impl fmt::Display for ApiError
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
	{
		match self
		{
			ApiError::Request { error } => error.fmt(f),
			ApiError::Response { error } => error.fmt(f),
		}
	}
}

impl error::Error for ApiError
{
	fn source(&self) -> Option<&(dyn error::Error + 'static)>
	{
		match self
		{
			ApiError::Request { error } => error.source(),
			ApiError::Response { error } => error.source(),
		}
	}
}
