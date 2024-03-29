/*
 * Part of epicinium_server
 * developed by A Bunch of Hacks.
 *
 * Copyright (c) 2018-2021 A Bunch of Hacks
 *
 * This library is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This library is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 * [authors:]
 * Sander in 't Veld (sander@abunchofhacks.coop)
 */

use crate::common::platform::*;
use crate::common::version::*;
use crate::server::settings::*;

use serde_derive::{Deserialize, Serialize};

use anyhow::anyhow;

use reqwest as http;

pub enum Setup
{
	Real
	{
		http: http::Client,
		registration_url: http::Url,
	},
	Dev
	{
		port: u16
	},
}

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

pub fn setup(settings: &Settings) -> Result<Setup, anyhow::Error>
{
	if settings.login_server.is_some()
		|| (!cfg!(feature = "version-is-dev")
			&& (!cfg!(debug_assertions) || cfg!(feature = "candidate")))
	{
		Connection::setup(settings)
	}
	else
	{
		let port = settings.port.ok_or_else(|| anyhow!("missing 'port'"))?;

		Ok(Setup::Dev { port })
	}
}

pub async fn bind(setup: Setup) -> Result<Binding, anyhow::Error>
{
	match setup
	{
		Setup::Real {
			http,
			registration_url,
		} => Connection::bind(http, registration_url).await,
		Setup::Dev { port } =>
		{
			let binding = Binding {
				connection: None,
				port,
			};
			Ok(binding)
		}
	}
}

impl Binding
{
	pub async fn confirm(&self) -> Result<(), anyhow::Error>
	{
		match &self.connection
		{
			Some(connection) => connection.confirm().await,
			None => Ok(()),
		}
	}

	pub async fn unbind(self) -> Result<(), anyhow::Error>
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
	fn setup(settings: &Settings) -> Result<Setup, anyhow::Error>
	{
		let url = settings
			.login_server
			.as_ref()
			.ok_or_else(|| anyhow!("missing 'login_server'"))?;
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

		let setup = Setup::Real {
			http,
			registration_url,
		};
		Ok(setup)
	}

	async fn bind(
		http: http::Client,
		registration_url: http::Url,
	) -> Result<Binding, anyhow::Error>
	{
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

	async fn deregister(self) -> Result<(), anyhow::Error>
	{
		let _: http::Response = self
			.http
			.request(http::Method::DELETE, self.registered_url.clone())
			.send()
			.await?
			.error_for_status()?;
		Ok(())
	}

	async fn confirm(&self) -> Result<(), anyhow::Error>
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
