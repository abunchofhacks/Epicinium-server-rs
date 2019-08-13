/* Settings */

use common::header::*;

use std::fs;
use std::io;

pub struct Settings
{
	filename: String,

	overrides: SettingContents,
	contents: SettingContents,
	defaults: SettingContents,
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
	pub fn logname(&self) -> Option<&String>
	{
		self.overrides
			.logname
			.as_ref()
			.or(self.contents.logname.as_ref())
			.or(self.defaults.logname.as_ref())
	}
	pub fn port(&self) -> Option<i32>
	{
		self.overrides
			.port
			.or(self.contents.port)
			.or(self.defaults.port)
	}
	pub fn login_server(&self) -> Option<&String>
	{
		self.overrides
			.login_server
			.as_ref()
			.or(self.contents.login_server.as_ref())
			.or(self.defaults.login_server.as_ref())
	}
	pub fn allow_discord_login(&self) -> Option<bool>
	{
		self.overrides
			.allow_discord_login
			.or(self.contents.allow_discord_login)
			.or(self.defaults.allow_discord_login)
	}
	pub fn slackname(&self) -> Option<&String>
	{
		self.overrides
			.slackname
			.as_ref()
			.or(self.contents.slackname.as_ref())
			.or(self.defaults.slackname.as_ref())
	}
	pub fn slackurl(&self) -> Option<&String>
	{
		self.overrides
			.slackurl
			.as_ref()
			.or(self.contents.slackurl.as_ref())
			.or(self.defaults.slackurl.as_ref())
	}
	pub fn discordurl(&self) -> Option<&String>
	{
		self.overrides
			.discordurl
			.as_ref()
			.or(self.contents.discordurl.as_ref())
			.or(self.defaults.discordurl.as_ref())
	}

	pub fn override_logname(&mut self, value: String)
	{
		self.overrides.logname = Some(value);
	}
	pub fn override_port(&mut self, value: i32)
	{
		self.overrides.port = Some(value);
	}
	pub fn override_login_server(&mut self, value: String)
	{
		self.overrides.login_server = Some(value);
	}
	pub fn override_allow_discord_login(&mut self, value: bool)
	{
		self.overrides.allow_discord_login = Some(value);
	}
	pub fn override_slackname(&mut self, value: String)
	{
		self.overrides.slackname = Some(value);
	}
	pub fn override_slackurl(&mut self, value: String)
	{
		self.overrides.slackurl = Some(value);
	}
	pub fn override_discordurl(&mut self, value: String)
	{
		self.overrides.discordurl = Some(value);
	}

	pub fn create(filename: &str) -> io::Result<Settings>
	{
		let mut settings = Settings {
			filename: filename.to_string(),
			overrides: Default::default(),
			contents: Default::default(),
			defaults: Default::default(),
		};

		if cfg!(debug_assertions)
		{
			if cfg!(feature = "candidate")
			{
				settings.defaults.port = Some(9976);
				settings.defaults.login_server =
					Some("https://test.epicinium.nl".to_string());
			}
			else
			{
				settings.defaults.port = Some(9999);
			}
		}
		else
		{
			settings.defaults.port = Some(9975);
			settings.defaults.login_server =
				Some("https://login.epicinium.nl".to_string());
		}
		settings.defaults.allow_discord_login = Some(false);

		settings.load()?;

		Ok(settings)
	}

	pub fn load(&mut self) -> io::Result<()>
	{
		let raw = fs::read_to_string(&self.filename)?;
		self.contents = serde_json::from_str(&raw)?;

		// TODO store recursive settings into defaults

		Ok(())
	}

	pub fn save(&self) -> io::Result<()>
	{
		unimplemented!();
	}
}
