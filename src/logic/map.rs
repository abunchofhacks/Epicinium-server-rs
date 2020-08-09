/* Map */

use crate::logic::epicinium;

use std::io;
use std::path::Path;

use tokio::fs::File;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;

use serde_derive::{Deserialize, Serialize};
use serde_json;

pub fn exists(mapname: &str) -> bool
{
	let fname = filename(mapname);
	Path::new(&fname).exists()
}

pub async fn load_pool_with_metadata(
) -> Result<Vec<(String, Metadata)>, io::Error>
{
	let names = epicinium::map_pool();
	let mut pool = Vec::with_capacity(names.len());
	for name in names
	{
		let metadata = load_metadata(&name).await?;
		pool.push((name, metadata));
	}
	Ok(pool)
}

pub async fn load_custom_and_user_pool_with_metadata(
) -> Result<Vec<(String, Metadata)>, io::Error>
{
	let names =
		[epicinium::map_custom_pool(), epicinium::map_user_pool()].concat();
	let mut pool = Vec::with_capacity(names.len());
	for name in names
	{
		let metadata = load_metadata(&name).await?;
		pool.push((name, metadata));
	}
	Ok(pool)
}

fn filename(mapname: &str) -> String
{
	format!("maps/{}.map", mapname)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata
{
	pub playercount: i32,

	#[serde(rename = "pool")]
	pub pool_type: PoolType,

	#[serde(rename = "ruleset", default)]
	pub ruleset_name: Option<String>,

	#[serde(flatten)]
	other: std::collections::HashMap<String, serde_json::Value>,
}

pub async fn load_metadata(mapname: &str) -> Result<Metadata, io::Error>
{
	let fname = filename(mapname);
	let file = File::open(fname).await?;
	let mut reader = BufReader::new(file);
	let mut buffer = String::new();
	reader.read_line(&mut buffer).await?;
	let metadata: Metadata = serde_json::from_str(&buffer)?;
	Ok(metadata)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PoolType
{
	None,
	Multiplayer,
	Custom,
}
