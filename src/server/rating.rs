/* Server::Rating */

use crate::common::keycode::Keycode;
use crate::common::platform::*;
use crate::common::version::*;
use crate::server::game;
use crate::server::settings::*;

use std::collections::HashMap;
use std::error;

use tokio::sync::mpsc;

use reqwest as http;

#[derive(Debug)]
pub enum Update
{
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
			Update::GameResult { result } =>
			{
				database.update_rating(result).await?
			}
		}
	}

	println!("Ratings have been pushed.");
	Ok(())
}

struct Database
{
	connection: Option<Connection>,
	cache: HashMap<Keycode, Entry>,
}

struct Entry
{
	client_username: String,
	rating: f32,
	stars: i32,
	recent_stars: i32,
}

impl Database
{
	async fn update_rating(
		&mut self,
		result: game::PlayerResult,
	) -> Result<(), Error>
	{
		// TODO update cache
		// TODO update connection
		println!("Result: {:?}.", result);
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
}

impl Connection
{
	async fn connect(
		settings: &Settings,
	) -> Result<Connection, Box<dyn error::Error>>
	{
		let url = settings.get_login_server()?;
		let base_url = http::Url::parse(url)?;

		let platform = Platform::current();
		let platformstring = serde_plain::to_string(&platform)?;
		let user_agent = format!(
			"epicinium-server/{} ({}; rust)",
			Version::current().to_string(),
			platformstring,
		);

		let http = http::Client::builder().user_agent(user_agent).build()?;

		Ok(Connection { http })
	}
}
