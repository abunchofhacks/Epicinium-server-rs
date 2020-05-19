/* Map */

use crate::logic::epicinium;

use std::io;

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

async fn load_metadata(_mapname: &str) -> Result<Metadata, io::Error>
{
	// TODO implement
	Ok(Metadata(serde_json::Value::Null))
}
