/* Settings */

use crate::common::header::*;

use std::error;
use std::fmt;
use std::fs;
use std::io;

use backtrace::Backtrace;

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
	server: Option<String>,
	#[serde(default, skip_serializing_if = "is_zero")]
	port: Option<u16>,
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

	#[serde(default, skip_serializing_if = "is_zero")]
	#[serde(rename = "defaults", alias = "settings")]
	fallback: Option<String>,
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
	pub fn server(&self) -> Option<&String>
	{
		self.overrides
			.server
			.as_ref()
			.or(self.contents.server.as_ref())
			.or(self.defaults.server.as_ref())
	}
	pub fn port(&self) -> Option<u16>
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

	pub fn get_logname(&self) -> Result<&String, Error>
	{
		self.logname().ok_or(Error::new("logname"))
	}
	pub fn get_server(&self) -> Result<&String, Error>
	{
		self.server().ok_or(Error::new("server"))
	}
	pub fn get_port(&self) -> Result<u16, Error>
	{
		self.port().ok_or(Error::new("port"))
	}
	pub fn get_login_server(&self) -> Result<&String, Error>
	{
		self.login_server().ok_or(Error::new("login-server"))
	}
	pub fn get_allow_discord_login(&self) -> Result<bool, Error>
	{
		self.allow_discord_login()
			.ok_or(Error::new("allow-discord-login"))
	}
	pub fn get_slackname(&self) -> Result<&String, Error>
	{
		self.slackname().ok_or(Error::new("slackname"))
	}
	pub fn get_slackurl(&self) -> Result<&String, Error>
	{
		self.slackurl().ok_or(Error::new("slackurl"))
	}
	pub fn get_discordurl(&self) -> Result<&String, Error>
	{
		self.discordurl().ok_or(Error::new("discordurl"))
	}

	pub fn set_logname(&mut self, value: String)
	{
		self.contents.logname = Some(value);
	}
	pub fn set_server(&mut self, value: String)
	{
		self.contents.server = Some(value);
	}
	pub fn set_port(&mut self, value: u16)
	{
		self.contents.port = Some(value);
	}
	pub fn set_login_server(&mut self, value: String)
	{
		self.contents.login_server = Some(value);
	}
	pub fn set_allow_discord_login(&mut self, value: bool)
	{
		self.contents.allow_discord_login = Some(value);
	}
	pub fn set_slackname(&mut self, value: String)
	{
		self.contents.slackname = Some(value);
	}
	pub fn set_slackurl(&mut self, value: String)
	{
		self.contents.slackurl = Some(value);
	}
	pub fn set_discordurl(&mut self, value: String)
	{
		self.contents.discordurl = Some(value);
	}

	pub fn override_logname(&mut self, value: String)
	{
		self.overrides.logname = Some(value);
	}
	pub fn override_server(&mut self, value: String)
	{
		self.overrides.server = Some(value);
	}
	pub fn override_port(&mut self, value: u16)
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
		let defaults = Settings::get_defaults();
		let arguments = Settings::parse_arguments()?;

		let fname = match arguments.fallback
		{
			Some(ref x) => x.to_string(),
			None => filename.to_string(),
		};

		let mut settings = Settings {
			filename: fname,
			overrides: arguments,
			contents: Default::default(),
			defaults: defaults,
		};

		settings.load()?;

		Ok(settings)
	}

	fn get_defaults() -> SettingContents
	{
		SettingContents {
			allow_discord_login: Some(false),
			..Default::default()
		}
	}

	fn parse_arguments() -> io::Result<SettingContents>
	{
		let mut blob: String = String::new();

		for arg in std::env::args().skip(1)
		{
			if arg.starts_with("--")
			{
				let (_, assignment) = arg.split_at(2);
				if blob.is_empty()
				{
					blob = assignment.to_string();
				}
				else
				{
					blob.push('&');
					blob.push_str(assignment);
				}
			}
			else
			{
				// Not a settings argument; will be handled by the application.
			}
		}

		// This is a bit weird, but it works.
		serde_urlencoded::from_str::<SettingContents>(&blob)
			.map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
	}

	pub fn load(&mut self) -> io::Result<()>
	{
		let raw = fs::read_to_string(&self.filename)?;
		self.contents = serde_json::from_str(&raw)?;

		let mut fallback = self.contents.fallback.clone();
		while let Some(filename) = fallback
		{
			let raw = fs::read_to_string(filename)?;
			let settings: SettingContents = serde_json::from_str(&raw)?;

			if settings.logname.is_some()
			{
				self.defaults.logname = settings.logname;
			}
			if settings.server.is_some()
			{
				self.defaults.server = settings.server;
			}
			if settings.port.is_some()
			{
				self.defaults.port = settings.port;
			}
			if settings.login_server.is_some()
			{
				self.defaults.login_server = settings.login_server;
			}
			if settings.allow_discord_login.is_some()
			{
				self.defaults.allow_discord_login =
					settings.allow_discord_login;
			}
			if settings.slackname.is_some()
			{
				self.defaults.slackname = settings.slackname;
			}
			if settings.slackurl.is_some()
			{
				self.defaults.slackurl = settings.slackurl;
			}
			if settings.discordurl.is_some()
			{
				self.defaults.discordurl = settings.discordurl;
			}

			fallback = settings.fallback;
		}

		Ok(())
	}

	pub fn save(&self) -> io::Result<()>
	{
		let jsonstr = serde_json::to_string_pretty(&self.contents)?;
		fs::write(&self.filename, jsonstr)?;

		Ok(())
	}
}

#[derive(Clone, Debug)]
pub struct Error
{
	setting_name: String,
	backtrace: Backtrace,
}

impl Error
{
	fn new(setting_name: &str) -> Self
	{
		Error {
			setting_name: setting_name.to_string(),
			backtrace: Backtrace::new(),
		}
	}
}

impl fmt::Display for Error
{
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result
	{
		write!(f, "setting '{}' undefined", self.setting_name)
	}
}

impl error::Error for Error
{
	fn source(&self) -> Option<&(dyn error::Error + 'static)>
	{
		None
	}
}
