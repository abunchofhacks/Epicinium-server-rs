/* Server::Portal */

use crate::common::platform::*;
use crate::common::version::*;
use crate::server::settings::*;

use std::error;

use serde_derive::{Deserialize, Serialize};

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
	http: http::Client,
	registered_url: http::Url,
}

pub async fn bind(settings: &Settings)
	-> Result<Binding, Box<dyn error::Error>>
{
	if settings.login_server().is_some()
		|| (!cfg!(feature = "version-is-dev")
			&& (!cfg!(debug_assertions) || cfg!(feature = "candidate")))
	{
		Connection::bind(settings).await
	}
	else
	{
		dev_bind(settings)
	}
}

fn dev_bind(settings: &Settings) -> Result<Binding, Box<dyn error::Error>>
{
	let port = settings.get_port()?;

	Ok(Binding {
		connection: None,
		port,
	})
}

impl Binding
{
	pub async fn confirm(&self) -> Result<(), Box<dyn error::Error>>
	{
		match &self.connection
		{
			Some(connection) => connection.confirm().await,
			None => Ok(()),
		}
	}

	pub async fn unbind(self) -> Result<(), Box<dyn error::Error>>
	{
		match self.connection
		{
			Some(connection) => connection.deregister().await,
			None => Ok(()),
		}
	}
}

impl Connection
{
	async fn bind(settings: &Settings)
		-> Result<Binding, Box<dyn error::Error>>
	{
		let url = settings.get_login_server()?;
		let base_url = http::Url::parse(url)?;

		let mut registration_url = base_url;
		registration_url.set_path("api/v1/servers");

		let platform = Platform::current();
		let platformstring = serde_plain::to_string(&platform)?;
		let user_agent = format!(
			"epicinium-server/{} ({}; rust)",
			Version::current(),
			platformstring,
		);

		let http = http::Client::builder().user_agent(user_agent).build()?;

		let response: RegistrationResponse = http
			.request(http::Method::POST, registration_url.clone())
			.send()
			.await?
			.error_for_status()?
			.json()
			.await?;

		let port: u16 = response.port;
		let path = format!("{}/{}", registration_url.path(), port);
		let mut registered_url = registration_url;
		registered_url.set_path(&path);

		Ok(Binding {
			connection: Some(Connection {
				http,
				registered_url,
			}),
			port,
		})
	}

	async fn deregister(self) -> Result<(), Box<dyn error::Error>>
	{
		let _: http::Response = self
			.http
			.request(http::Method::DELETE, self.registered_url.clone())
			.send()
			.await?
			.error_for_status()?;
		Ok(())
	}

	async fn confirm(&self) -> Result<(), Box<dyn error::Error>>
	{
		let info = ServerConfirmation { online: true };
		let payload = serde_json::to_string(&info)?;

		let _: http::Response = self
			.http
			.request(http::Method::PATCH, self.registered_url.clone())
			.body(payload)
			.send()
			.await?
			.error_for_status()?;
		Ok(())
	}
}

#[derive(Clone, Deserialize, Debug)]
struct RegistrationResponse
{
	port: u16,
}
