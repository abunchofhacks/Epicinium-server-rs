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

use crate::common::platform::Platform;
use crate::common::version::Version;
use crate::server::settings::Settings;

use log::*;

use serde_json::json;

use anyhow::anyhow;

use tokio::sync::mpsc;

use reqwest as http;

#[derive(Debug)]
pub struct Post
{
	pub message: String,
}

pub struct Setup
{
	connection: Option<Connection>,
}

pub fn setup(settings: &Settings) -> Result<Setup, anyhow::Error>
{
	if settings.slackurl.is_some()
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
			connection: Some(mut connection),
		} =>
		{
			info!("Connected.");
			connection.run(posts).await;
			info!("Finished sending posts to Slack.");
		}
		Setup { connection: None } =>
		{
			while let Some(post) = posts.recv().await
			{
				debug!("{}", post.message);
			}
		}
	}
}

struct Connection
{
	http: http::Client,
	url: http::Url,
	name: String,
}

impl Connection
{
	fn start(settings: &Settings) -> Result<Connection, anyhow::Error>
	{
		let servername = settings
			.slackname
			.as_ref()
			.ok_or_else(|| anyhow!("missing 'slackname'"))?;
		let name = format!("{}-{}", servername, Version::current());

		let url = settings
			.slackurl
			.as_ref()
			.ok_or_else(|| anyhow!("missing 'slackurl'"))?;
		let url = http::Url::parse(url)?;

		let platform = Platform::current();
		let platformstring = serde_plain::to_string(&platform)?;
		let user_agent = format!(
			"epicinium-server/{} ({}; rust)",
			Version::current(),
			platformstring,
		);

		let http = http::Client::builder().user_agent(user_agent).build()?;

		let connection = Connection { http, url, name };

		Ok(connection)
	}

	async fn run(&mut self, mut posts: mpsc::Receiver<Post>)
	{
		let post = Post {
			message: "Server started.".to_string(),
		};
		self.send(post).await;

		while let Some(post) = posts.recv().await
		{
			self.send(post).await;
		}

		let post = Post {
			message: "Server stopped.".to_string(),
		};
		self.send(post).await;
	}

	async fn send(&self, post: Post)
	{
		trace!("Sending: {}", post.message);

		let payload = json!({
			"channel": "server-notifications",
			"username": self.name,
			"icon_emoji": ":signal_strength:",
			"text": post.message,
		});

		match self.try_send(&payload).await
		{
			Ok(()) =>
			{}
			Err(error) =>
			{
				error!("Error: {:#?}", error);
			}
		}
	}

	async fn try_send(
		&self,
		payload: &serde_json::Value,
	) -> Result<(), http::Error>
	{
		let response = self
			.http
			.request(http::Method::POST, self.url.clone())
			.json(payload)
			.send()
			.await?;
		let _response = response.error_for_status()?;
		Ok(())
	}
}
