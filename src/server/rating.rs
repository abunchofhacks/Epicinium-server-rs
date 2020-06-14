/* Server::Rating */

use crate::common::platform::*;
use crate::common::version::*;
use crate::logic::challenge;
use crate::server::game;
use crate::server::login::UserId;
use crate::server::message::ResponseStatus;
use crate::server::settings::*;

use std::collections::HashMap;
use std::error;

use tokio::sync::mpsc;

use reqwest as http;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Data
{
	pub rating: f32,
	pub stars: i32,
	pub recent_stars: i32,
}

#[derive(Debug)]
pub enum Update
{
	Fresh
	{
		user_id: UserId, data: Data
	},
	GameResult
	{
		result: game::PlayerResult
	},
}

pub async fn run(
	settings: &Settings,
	mut updates: mpsc::Receiver<Update>,
) -> Result<(), Box<dyn error::Error>>
{
	let mut database = initialize(settings).await?;

	while let Some(update) = updates.recv().await
	{
		match update
		{
			Update::Fresh { user_id, data } =>
			{
				database.cache.insert(user_id, data);
			}
			Update::GameResult { result } =>
			{
				database.handle_result(result).await?
			}
		}
	}

	println!("Ratings have been pushed.");
	Ok(())
}

struct Database
{
	connection: Option<Connection>,
	cache: HashMap<UserId, Data>,
}

impl Database
{
	async fn handle_result(
		&mut self,
		result: game::PlayerResult,
	) -> Result<(), Box<dyn error::Error>>
	{
		let user_id = result.user_id;
		let entry = match self.cache.get_mut(&user_id)
		{
			Some(entry) => entry,
			None =>
			{
				eprintln!("Missing entry for user id {:?}!", user_id);
				// We do not want this to end the rating task.
				return Ok(());
			}
		};
		if result.is_rated
		{
			// TODO calculate rating

			if let Some(connection) = &mut self.connection
			{
				connection.update_rating(user_id, entry.rating).await?;
			}
		}
		if result.stars_for_current_challenge > entry.recent_stars
		{
			let diff = result.stars_for_current_challenge - entry.recent_stars;
			entry.stars += diff;
			entry.recent_stars = result.stars_for_current_challenge;

			if let Some(connection) = &mut self.connection
			{
				connection.award_stars(user_id, entry.recent_stars).await?;
			}
		}
		Ok(())
	}
}

async fn initialize(
	settings: &Settings,
) -> Result<Database, Box<dyn error::Error>>
{
	if settings.login_server().is_some()
		|| (!cfg!(feature = "version-is-dev")
			&& (!cfg!(debug_assertions) || cfg!(feature = "candidate")))
	{
		let connection = Connection::connect(settings).await?;
		Ok(Database {
			connection: Some(connection),
			cache: HashMap::new(),
		})
	}
	else
	{
		Ok(Database {
			connection: None,
			cache: HashMap::new(),
		})
	}
}

struct Connection
{
	http: http::Client,
	update_rating_url: http::Url,
	award_stars_url: http::Url,
	current_challenge_key: String,
}

impl Connection
{
	async fn connect(
		settings: &Settings,
	) -> Result<Connection, Box<dyn error::Error>>
	{
		let url = settings.get_login_server()?;
		let base_url = http::Url::parse(url)?;

		let mut update_rating_url = base_url.clone();
		update_rating_url.set_path("api/v1/update_rating");

		let mut award_stars_url = base_url;
		award_stars_url.set_path("api/v1/award_stars");

		let platform = Platform::current();
		let platformstring = serde_plain::to_string(&platform)?;
		let user_agent = format!(
			"epicinium-server/{} ({}; rust)",
			Version::current().to_string(),
			platformstring,
		);

		let http = http::Client::builder().user_agent(user_agent).build()?;

		Ok(Connection {
			http,
			update_rating_url,
			award_stars_url,
			current_challenge_key: challenge::get_current_key(),
		})
	}

	async fn update_rating(
		&self,
		user_id: UserId,
		rating: f32,
	) -> Result<(), Box<dyn error::Error>>
	{
		let payload = json!({
			"user_id": user_id,
			"rating": rating,
		});

		let response: Response = self
			.http
			.request(http::Method::POST, self.update_rating_url.clone())
			.json(&payload)
			.send()
			.await?
			.error_for_status()?
			.json()
			.await?;
		println!("Got a response from database: {:?}", response);
		response.verify()?;
		Ok(())
	}

	async fn award_stars(
		&self,
		user_id: UserId,
		stars_for_current_challenge: i32,
	) -> Result<(), Box<dyn error::Error>>
	{
		let payload = json!({
			"user_id": user_id,
			"key": self.current_challenge_key.clone(),
			"stars": stars_for_current_challenge,
		});

		let response: Response = self
			.http
			.request(http::Method::POST, self.award_stars_url.clone())
			.json(&payload)
			.send()
			.await?
			.error_for_status()?
			.json()
			.await?;
		println!("Got a response from database: {:?}", response);
		response.verify()?;
		Ok(())
	}
}

#[derive(Debug, Clone, Deserialize)]
struct Response
{
	status: ResponseStatus,
}

impl Response
{
	fn verify(self) -> Result<Response, ResponseError>
	{
		if self.status == ResponseStatus::Success
		{
			Ok(self)
		}
		else
		{
			Err(ResponseError { response: self })
		}
	}
}

#[derive(Debug)]
struct ResponseError
{
	response: Response,
}

impl std::fmt::Display for ResponseError
{
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
	{
		write!(f, "Unexpected response from database: {:?}", self.response)
	}
}

impl std::error::Error for ResponseError {}