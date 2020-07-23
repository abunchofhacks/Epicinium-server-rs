/* Ruleset */

use crate::logic::epicinium;

use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::io::BufReader;

use serde_json;

pub fn initialize_collection() -> Result<(), InitializationError>
{
	epicinium::initialize_ruleset_collection()
}

pub fn current() -> String
{
	epicinium::name_current_ruleset()
}

pub fn exists(name: &str) -> bool
{
	epicinium::ruleset_exists(name)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Data(serde_json::Value);

pub async fn load_data(name: &str) -> Result<Data, std::io::Error>
{
	let fname = filename(name);
	let file = File::open(fname).await?;
	let mut reader = BufReader::new(file);
	let mut buffer = String::new();
	reader.read_to_string(&mut buffer).await?;
	let data: Data = serde_json::from_str(&buffer)?;
	Ok(data)
}

fn filename(name: &str) -> String
{
	format!("rulesets/{}.json", name)
}

#[derive(Debug)]
pub enum InitializationError
{
	Failed,
}

impl std::fmt::Display for InitializationError
{
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
	{
		match self
		{
			InitializationError::Failed => write!(f, "initialization failed"),
		}
	}
}

impl std::error::Error for InitializationError {}
