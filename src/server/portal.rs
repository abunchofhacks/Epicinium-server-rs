/* Server::Login */

use server::settings::*;

use std::error;
use std::fmt;
use std::io;

use futures::future;
use futures::future::Either;
use futures::{Future, IntoFuture};

use reqwest as http;

pub struct Binding
{
	connection: Option<Connection>,

	pub port: u16,
}

#[derive(Serialize, Deserialize, Debug)]
struct ServerConfirmation
{
	online: bool,
}

struct Connection
{
	http: http::async::Client,
	registered_url: http::Url,
}

pub fn bind(
	settings: &Settings,
) -> impl Future<Item = Binding, Error = ()> + Send
{
	if settings.login_server().is_some()
		|| (!cfg!(feature = "version-is-dev")
			&& (!cfg!(debug_assertions) || cfg!(feature = "candidate")))
	{
		Either::A(Connection::bind(settings))
	}
	else
	{
		Either::B(dev_bind(settings).into_future())
	}
}

fn dev_bind(settings: &Settings) -> Result<Binding, ()>
{
	match settings.port()
	{
		Some(port) => Ok(Binding {
			connection: None,
			port,
		}),
		None => Err(eprintln!("Cannot bind in dev mode: no port defined")),
	}
}

impl Binding
{
	pub fn confirm(&self) -> impl Future<Item = (), Error = ()> + Send
	{
		match &self.connection
		{
			Some(connection) => Either::A(connection.confirm()),
			None => Either::B(future::ok(())),
		}
	}

	pub fn unbind(self) -> impl Future<Item = (), Error = ()> + Send
	{
		match self.connection
		{
			Some(connection) => Either::A(connection.deregister()),
			None => Either::B(future::ok(())),
		}
	}
}

fn build_registration_url(
	settings: &Settings,
) -> Result<http::Url, Box<dyn error::Error>>
{
	let url = settings.get_login_server()?;
	let base_url = http::Url::parse(url)?;

	let mut registration_url = base_url;
	registration_url.set_path("api/v1/servers");

	Ok(registration_url)
}

impl Connection
{
	fn bind(
		settings: &Settings,
	) -> impl Future<Item = Binding, Error = ()> + Send
	{
		build_registration_url(settings)
			.map_err(|error| eprintln!("Failed to build url: {}", error))
			.into_future()
			.and_then(|url| Connection::resolve(url))
	}

	fn resolve(
		registration_url: http::Url,
	) -> impl Future<Item = Binding, Error = ()> + Send
	{
		let http = http::async::Client::new();
		Connection::register(&http, registration_url).map(
			move |(port, registered_url)| Binding {
				connection: Some(Connection {
					http,
					registered_url,
				}),
				port,
			},
		)
	}

	fn register(
		http: &http::async::Client,
		registration_url: http::Url,
	) -> impl Future<Item = (u16, http::Url), Error = ()> + Send
	{
		http.request(http::Method::POST, registration_url.clone())
			.send()
			.map_err(|error| error.into())
			.and_then(|response| {
				response.error_for_status().map_err(|error| error.into())
			})
			.and_then(|mut response| {
				response.json().map_err(|error| error.into())
			})
			.map(move |response: RegistrationResponse| {
				let port = response.port;
				let path = format!("{}/{}", registration_url.path(), port);
				let mut url = registration_url;
				url.set_path(&path);
				(port, url)
			})
			.map_err(|error: ApiError| {
				eprintln!("Failed to register server: {}", error)
			})
	}

	fn deregister(self) -> impl Future<Item = (), Error = ()> + Send
	{
		self.http
			.request(http::Method::DELETE, self.registered_url.clone())
			.send()
			.map_err(|error| error.into())
			.and_then(|response| {
				response.error_for_status().map_err(|e| e.into())
			})
			.map(|_| ())
			.map_err(|error: ApiError| {
				eprintln!("Failed to deregister server: {}", error)
			})
	}

	fn confirm(&self) -> impl Future<Item = (), Error = ()> + Send
	{
		let info = ServerConfirmation { online: true };
		match serde_json::to_string(&info)
		{
			Ok(payload) => Either::A(self.update(payload)),
			Err(error) =>
			{
				eprintln!("Failed to prepare update payload: {}", error);
				Either::B(future::err(()))
			}
		}
	}

	fn update(
		&self,
		payload: String,
	) -> impl Future<Item = (), Error = ()> + Send
	{
		self.http
			.request(http::Method::PATCH, self.registered_url.clone())
			.body(payload)
			.send()
			.map_err(|error| error.into())
			.and_then(|response| {
				response.error_for_status().map_err(|e| e.into())
			})
			.map(|_| ())
			.map_err(|error: ApiError| {
				eprintln!("Failed to send update to portal: {}", error)
			})
	}
}

#[derive(Clone, Deserialize, Debug)]
struct RegistrationResponse
{
	port: u16,
}

#[derive(Debug)]
enum ApiError
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
