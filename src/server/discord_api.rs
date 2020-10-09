/* Server::DiscordApi */

use crate::common::platform::Platform;
use crate::common::version::Version;
use crate::server::settings::Settings;

use log::*;

use serde_derive::{Deserialize, Serialize};
use serde_json::json;

use anyhow::anyhow;

use tokio::sync::mpsc;
use tokio::time::Duration;

use reqwest as http;

#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Post
{
	GameStarted
	{
		#[serde(rename = "rated")]
		is_rated: bool,

		#[serde(rename = "player1")]
		first_player_username: String,

		#[serde(rename = "player2")]
		second_player_username: String,

		#[serde(rename = "map")]
		map_name: String,

		#[serde(rename = "ruleset")]
		ruleset_name: String,

		#[serde(rename = "time")]
		planning_time_in_seconds_or_zero: u32,
	},
	GameEnded
	{
		#[serde(rename = "rated")]
		is_rated: bool,

		#[serde(rename = "player1")]
		first_player_username: String,

		#[serde(rename = "player1_defeated")]
		is_first_player_defeated: bool,

		#[serde(rename = "player1_score")]
		first_player_score: i32,

		#[serde(rename = "player2")]
		second_player_username: String,

		#[serde(rename = "player2_defeated")]
		is_second_player_defeated: bool,

		#[serde(rename = "player2_score")]
		second_player_score: i32,
	},
	Link
	{
		discord_id: String,
		username: String,
	},
}

pub struct Setup
{
	connection: Option<Connection>,
}

pub fn setup(settings: &Settings) -> Result<Setup, anyhow::Error>
{
	if settings.discordurl.is_some()
	{
		let connection = Connection::start(settings)?;
		Ok(Setup {
			connection: Some(connection),
		})
	}
	else
	{
		Ok(Setup { connection: None })
	}
}

pub async fn run(setup: Setup, mut posts: mpsc::Receiver<Post>)
{
	match setup
	{
		Setup {
			connection: Some(connection),
		} =>
		{
			info!("Connected.");
			while let Some(post) = posts.recv().await
			{
				connection.send(post).await;
			}
			info!("Finished sending posts to Discord.");
		}
		Setup { connection: None } =>
		{
			while let Some(post) = posts.recv().await
			{
				let message = match serde_json::to_string(&post)
				{
					Ok(message) => message,
					Err(error) =>
					{
						error!("Error while jsonifying: {:?}", error);
						debug!("Original post: {:?}", post);
						continue;
					}
				};
				debug!("{}", message);
			}
		}
	}
}

struct Connection
{
	http: http::Client,
	url: http::Url,
}

impl Connection
{
	fn start(settings: &Settings) -> Result<Connection, anyhow::Error>
	{
		let url = settings
			.discordurl
			.as_ref()
			.ok_or_else(|| anyhow!("missing 'discordurl'"))?;
		let mut url = http::Url::parse(url)?;
		url.query_pairs_mut().append_pair("wait", "true");

		let platform = Platform::current();
		let platformstring = serde_plain::to_string(&platform)?;
		let user_agent = format!(
			"epicinium-server/{} ({}; rust)",
			Version::current(),
			platformstring,
		);

		let http = http::Client::builder().user_agent(user_agent).build()?;

		let connection = Connection { http, url };

		Ok(connection)
	}

	async fn send(&self, post: Post)
	{
		let message = match serde_json::to_string(&post)
		{
			Ok(message) => message,
			Err(error) =>
			{
				error!("Error while jsonifying: {:?}", error);
				error!("Original post: {:?}", post);
				return;
			}
		};

		trace!("Sending: {}", message);

		let payload = json!({
			"content": message,
		});

		loop
		{
			match self.try_send(&payload).await
			{
				Ok(Status::Ok) => break,
				Ok(Status::RateLimited { retry_after }) =>
				{
					warn!(
						"We are being rate limited, retrying after {}ms...",
						retry_after.as_millis()
					);
					tokio::time::delay_for(retry_after).await;
				}
				Err(error) =>
				{
					error!("Error: {:#?}", error);
					break;
				}
			}
		}
	}

	async fn try_send(
		&self,
		payload: &serde_json::Value,
	) -> Result<Status, Error>
	{
		let response = self
			.http
			.request(http::Method::POST, self.url.clone())
			.json(payload)
			.send()
			.await?;
		let status = response.status();
		if status == http::StatusCode::TOO_MANY_REQUESTS
		{
			let text = response.text().await?;
			let response: Response = serde_json::from_str(&text)?;
			let retry_after = Duration::from_millis(response.retry_after);
			Ok(Status::RateLimited { retry_after })
		}
		else
		{
			let _response = response.error_for_status()?;
			Ok(Status::Ok)
		}
	}
}

#[derive(Debug)]
enum Status
{
	Ok,
	RateLimited
	{
		retry_after: Duration,
	},
}

#[derive(Debug, Deserialize)]
struct Response
{
	retry_after: u64,
}

#[derive(Debug)]
enum Error
{
	Http(http::Error),
	Json(serde_json::Error),
}

impl From<http::Error> for Error
{
	fn from(error: http::Error) -> Error
	{
		Error::Http(error)
	}
}

impl From<serde_json::Error> for Error
{
	fn from(error: serde_json::Error) -> Error
	{
		Error::Json(error)
	}
}

impl std::fmt::Display for Error
{
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
	{
		match self
		{
			Error::Http(error) => error.fmt(f),
			Error::Json(error) => error.fmt(f),
		}
	}
}
