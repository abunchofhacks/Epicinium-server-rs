/* Settings */

use crate::common::log;

use std::path::Path;

use serde_derive::Deserialize;

use anyhow::Context;

#[derive(Clone, Deserialize, Debug, Default)]
#[serde(rename_all = "kebab-case")]
pub struct Settings
{
	#[serde(default)]
	pub logname: Option<String>,
	#[serde(default)]
	pub loglevel: Option<log::Level>,

	#[serde(default)]
	pub server: Option<String>,
	#[serde(default)]
	pub port: Option<u16>,

	#[serde(default)]
	pub login_server: Option<String>,
	#[serde(default)]
	pub allow_discord_login: Option<bool>,
	#[serde(default)]
	pub steam_web_key: Option<String>,

	#[serde(default)]
	pub slackname: Option<String>,
	#[serde(default)]
	pub slackurl: Option<String>,

	#[serde(default)]
	pub discordurl: Option<String>,
}

impl Settings
{
	pub fn load(filename: &str) -> Result<Settings, anyhow::Error>
	{
		let filename = Path::new(filename);
		let raw = std::fs::read_to_string(filename)?;
		let settings = serde_json::from_str(&raw).with_context(|| {
			format!("parsing settings from '{}'", filename.display())
		})?;
		Ok(settings)
	}
}
