/* Server::Login */

use crate::common::platform::*;
use crate::common::version::*;
use crate::server::settings::*;

use std::error;

use futures::future;
use futures::future::Either;
use futures::Future;

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
	user_agent: http::header::HeaderValue,
	registered_url: http::Url,
}

pub async fn bind(
	settings: &Settings,
) -> Result<Binding, Box<dyn error::Error>>
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

	Binding {
			connection: None,
			port,
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

async fn build_headers_and_urls(
	settings: &Settings,
) -> Result<(http::header::HeaderValue, http::Url), Box<dyn error::Error>>
{
	let url = settings.get_login_server()?;
	let base_url = http::Url::parse(url)?;

	let mut registration_url = base_url;
	registration_url.set_path("api/v1/servers");

	let platform = Platform::current();
	let platformstring = serde_plain::to_string(&platform)?;
	let uastring = format!(
		"epicinium-server/{} ({}; rust)",
		Version::current().to_string(),
		platformstring,
	);
	let user_agent = uastring.parse()?;

	Ok((user_agent, registration_url))
}

impl Connection
{
	async fn bind(
		settings: &Settings,
	) -> Result<Binding, Box<dyn error::Error>>
	{
		let (ua, url) = build_headers_and_urls(settings).await?;

		Connection::resolve(ua, url).await
	}

	async fn resolve(
		user_agent: http::header::HeaderValue,
		registration_url: http::Url,
	) -> Result<Binding, Box<dyn error::Error>>
	{
		let http = http::Client::new();
		let (port, registered_url) = Connection::register(&http, &user_agent, registration_url).await?;

		Ok(Binding {
				connection: Some(Connection {
					http,
					user_agent,
					registered_url,
				}),
				port,
			})
	}

	async fn register(
		http: &http::Client,
		user_agent: &http::header::HeaderValue,
		registration_url: http::Url,
	) -> Result<(u16, http::Url), Box<dyn error::Error>>
	{
		let response = http.post(registration_url.clone())
			.header(http::header::USER_AGENT, user_agent.clone())
			.send()
			.await?
			.error_for_status()?
			.json::<RegistrationResponse>()
			.await?;

		let port = response.port;
		let path = format!("{}/{}", registration_url.path(), port);
		let mut url = registration_url;
		url.set_path(&path);

		Ok((port, url))
	}

	async fn deregister(self) -> Result<(), Box<dyn error::Error>>
	{
		let _response = self.http
			.delete(self.registered_url.clone())
			.header(http::header::USER_AGENT, self.user_agent.clone())
			.send()
			.await?
			.error_for_status()?;
		Ok(())
	}

	async fn confirm(&self) -> Result<(), Box<dyn error::Error>>
	{
		let info = ServerConfirmation { online: true };
		let payload = serde_json::to_string(&info)?;
		self.update(payload).await;
		Ok(())
	}

	async fn update(
		&self,
		payload: String,
	) -> Result<(), Box<dyn error::Error>>
	{
		let _response = self.http
			.patch(self.registered_url.clone())
			.header(http::header::USER_AGENT, self.user_agent.clone())
			.body(payload)
			.send()
			.await?
			.error_for_status()?;
		Ok(())
	}
}

#[derive(Deserialize, Debug)]
struct RegistrationResponse
{
	port: u16,
}
