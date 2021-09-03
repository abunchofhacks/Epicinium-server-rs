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

pub use epicinium_lib::error::InitializationError;

use tokio::fs::File;
use tokio::io::AsyncReadExt;
use tokio::io::BufReader;

use serde_derive::{Deserialize, Serialize};
use serde_json;

pub fn initialize_collection() -> Result<(), InitializationError>
{
	epicinium_lib::initialize_ruleset_collection()
}

pub fn current() -> String
{
	epicinium_lib::name_current_ruleset()
}

pub fn exists(name: &str) -> bool
{
	epicinium_lib::ruleset_exists(name)
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
