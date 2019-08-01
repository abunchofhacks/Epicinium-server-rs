/* Settings */

use common::header::*;

use std::fs;
use std::io;

pub struct Settings
{
	// TODO fallback
	filename: String,

	contents: SettingContents,
}

#[derive(Clone, Serialize, Deserialize, Debug, Default)]
#[serde(rename_all = "kebab-case")]
struct SettingContents
{
	#[serde(default, skip_serializing_if = "is_zero")]
	logname: Option<String>,
	// TODO loglevel
	// TODO logrollback
	// TODO perflog
	// TODO datafolder
	// TODO seed
	#[serde(default, skip_serializing_if = "is_zero")]
	port: Option<i32>,
	#[serde(default, skip_serializing_if = "is_zero")]
	login_server: Option<String>,
	#[serde(default, skip_serializing_if = "is_zero")]
	allow_discord_login: Option<bool>,
	#[serde(default, skip_serializing_if = "is_zero")]
	slackname: Option<String>,
	#[serde(default, skip_serializing_if = "is_zero")]
	slackurl: Option<String>,
	#[serde(default, skip_serializing_if = "is_zero")]
	discordurl: Option<String>,
}

impl Settings
{
	pub fn logname(&self) -> &Option<String>
	{
		&self.contents.logname
	}
	pub fn port(&self) -> &Option<i32>
	{
		&self.contents.port
	}
	pub fn login_server(&self) -> &Option<String>
	{
		&self.contents.login_server
	}
	pub fn allow_discord_login(&self) -> &Option<bool>
	{
		&self.contents.allow_discord_login
	}
	pub fn slackname(&self) -> &Option<String>
	{
		&self.contents.slackname
	}
	pub fn slackurl(&self) -> &Option<String>
	{
		&self.contents.slackurl
	}
	pub fn discordurl(&self) -> &Option<String>
	{
		&self.contents.discordurl
	}

	pub fn create(filename: &str) -> io::Result<Settings>
	{
		let mut settings = Settings {
			filename: filename.to_string(),
			contents: Default::default(),
		};

		settings.load()?;

		Ok(settings)
	}

	pub fn load(&mut self) -> io::Result<()>
	{
		if self.filename.is_empty()
		{
			unimplemented!();
		}
		else
		{
			let raw = fs::read_to_string(&self.filename)?;
			self.contents = serde_json::from_str(&raw)?;

			Ok(())
		}
	}

	pub fn save(&self) -> io::Result<()>
	{
		if self.filename.is_empty()
		{
			unimplemented!();
		}
		else
		{
			unimplemented!();
		}
	}
}
