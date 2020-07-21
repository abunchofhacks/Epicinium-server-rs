/* Server::SlackApi */

use crate::common::platform::Platform;
use crate::common::version::Version;
use crate::server::settings::Settings;

use log::*;

use tokio::sync::mpsc;

use reqwest as http;

#[derive(Debug)]
pub struct Post
{
	pub message: String,
}

pub async fn run(
	settings: &Settings,
	mut posts: mpsc::Receiver<Post>,
) -> Result<(), Box<dyn std::error::Error>>
{
	if settings.slackurl().is_some()
	{
		let mut connection = Connection::start(settings)?;
		info!("Connected.");
		connection.run(posts).await;
		info!("Finished sending posts to Slack.");
	}
	else
	{
		while let Some(post) = posts.recv().await
		{
			debug!("{}", post.message);
		}
	}
	Ok(())
}

struct Connection
{
	http: http::Client,
	url: http::Url,
	name: String,
}

impl Connection
{
	fn start(
		settings: &Settings,
	) -> Result<Connection, Box<dyn std::error::Error>>
	{
		let servername = settings.get_slackname()?;
		let name = format!("{}-{}", servername, Version::current().to_string());

		let url = settings.get_slackurl()?;
		let url = http::Url::parse(url)?;

		let platform = Platform::current();
		let platformstring = serde_plain::to_string(&platform)?;
		let user_agent = format!(
			"epicinium-server/{} ({}; rust)",
			Version::current().to_string(),
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
