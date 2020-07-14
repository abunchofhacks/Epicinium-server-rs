/* Server::Rating */

use crate::common::platform::*;
use crate::common::version::*;
use crate::logic::challenge;
use crate::server::game;
use crate::server::game::MatchType;
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
	pub rating: f64,
	pub stars: i32,
	pub recent_stars: i32,
}

#[derive(Debug)]
pub enum Update
{
	Fresh
	{
		user_id: UserId,
		data: Data,
	},
	GameResult(game::PlayerResult),
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
			Update::GameResult(result) =>
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
			let result = adjust(entry.rating, result.score, result.match_type);
			if let Some(new_rating) = result
			{
				entry.rating = new_rating;
				if let Some(connection) = &mut self.connection
				{
					connection.update_rating(user_id, entry.rating).await?;
				}
			}
		}

		if result.challenge == Some(challenge::current_id())
			&& result.awarded_stars > entry.recent_stars
		{
			let diff = result.awarded_stars - entry.recent_stars;
			entry.stars += diff;
			entry.recent_stars = result.awarded_stars;

			if let Some(connection) = &mut self.connection
			{
				connection.award_stars(user_id, entry.recent_stars).await?;
			}
		}
		Ok(())
	}
}

fn adjust(rating: f64, score: i32, match_type: MatchType) -> Option<f64>
{
	let (mut gain_percentage, loss_percentage) = match match_type
	{
		MatchType::Competitive => (10, 10),
		MatchType::FreeForAll {
			num_non_bot_players: num,
		} => (num as i32, 1),
		MatchType::VersusAi => (1, 1),
		MatchType::Unrated => return None,
	};

	// Represent 12.3f as 123 tenths.
	let ratingtenths = (10.0 * rating + 0.5) as i32;
	let scoretenths = 10 * score;

	// For players with a rating below 9.0f, the gain percentage is increased.
	if ratingtenths < 90
	{
		// At least 10% for a rating of 0.0f; at least 2% for a rating of 8.9f.
		let minimum = 10 - (ratingtenths / 10);
		gain_percentage = std::cmp::max(gain_percentage, minimum);
	}

	// Should the rating increase...
	let ratingtenths = if scoretenths > ratingtenths
	{
		// Get the absolute difference.
		let difference = scoretenths - ratingtenths;
		// Rating gain is a percentage of the difference,
		// rounded down to the nearest tenth,
		// but at least a tenth.
		let gaintenths = std::cmp::max(1, (gain_percentage * difference) / 100);
		// Increase the rating by the gain.
		std::cmp::max(0, std::cmp::min(ratingtenths + gaintenths, 1000))
	}
	// ... or decrease...
	else if scoretenths < ratingtenths
	{
		// Get the absolute difference.
		let difference = ratingtenths - scoretenths;
		// Rating loss is a percentage of the difference,
		// rounded down to the nearest tenth,
		// but at least a tenth.
		let losstenths = std::cmp::max(1, (loss_percentage * difference) / 100);
		// Lower the rating by the loss.
		std::cmp::max(0, std::cmp::min(ratingtenths - losstenths, 1000))
	}
	// ... or stay the same?
	else
	{
		ratingtenths
	};

	// Convert back to real rating.
	Some(0.1 * (ratingtenths as f64))
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
		rating: f64,
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
