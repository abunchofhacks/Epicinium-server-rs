/* Map */

use crate::logic::epicinium;

use std::io;

use tokio::fs::File;
use tokio::io::AsyncBufReadExt;
use tokio::io::BufReader;

use serde_json;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metadata(serde_json::Value);

async fn load_metadata(mapname: &str) -> Result<Metadata, io::Error>
{
	let filename = format!("maps/{}.map", mapname);
	let file = File::open(filename).await?;
	let mut reader = BufReader::new(file);
	let mut buffer = String::new();
	reader.read_line(&mut buffer).await?;
	let metadata: Metadata = serde_json::from_str(&buffer)?;
	Ok(metadata)
}
