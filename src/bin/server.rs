/* Server */

use epicinium::Settings;
use epicinium::Version;
use epicinium::{run_server, setup_server};

use log::{error, info};

use docopt::Docopt;

use serde::Deserialize;

const USAGE: &'static str = "
Usage: server [options]

Options:
	--logname=NAME               The name used in the filenames of logs.
	--loglevel=LEVEL             The level to filter on when writing logs.
	--server=IPADDRESS           The IP address to bind to.
	--port=PORT                  The port to bind to.
	--login-server=URL           The login server to connect to.
	--allow-discord-login=BOOL   Whether to allow clients to log in using only
	                             their Discord username as credentials.
	--steam-web-key=FILENAME     The location of the Steam Web API Key.
	--slackurl=URL               The Slack callback url to post to.
	--slackname=NAME             The name with which to post to Slack.
	--discordurl=URL             The Discord callback url to post to.
	--settings=FILENAME          Filename to load additional settings from.
";

#[derive(Deserialize)]
struct Args
{
	flag_settings: Option<String>,

	flag_logname: Option<String>,
	flag_loglevel: Option<epicinium::common::log::Level>,

	flag_server: Option<String>,
	flag_port: Option<u16>,

	flag_login_server: Option<String>,
	flag_allow_discord_login: Option<bool>,
	flag_steam_web_key: Option<String>,

	flag_slackname: Option<String>,
	flag_slackurl: Option<String>,

	flag_discordurl: Option<String>,
}

fn main() -> std::result::Result<(), anyhow::Error>
{
	let args: Args = Docopt::new(USAGE)
		.unwrap()
		.deserialize()
		.unwrap_or_else(|error| error.exit());

	let settings_filename = args
		.flag_settings
		.clone()
		.unwrap_or("settings-server.json".to_string());
	let mut settings = Settings::load(&settings_filename)?;

	settings.logname = args.flag_logname.or(settings.logname);
	settings.loglevel = args.flag_loglevel.or(settings.loglevel);
	settings.server = args.flag_server.or(settings.server);
	settings.port = args.flag_port.or(settings.port);
	settings.login_server = args.flag_login_server.or(settings.login_server);
	settings.allow_discord_login = args
		.flag_allow_discord_login
		.or(settings.allow_discord_login);
	settings.steam_web_key = args.flag_steam_web_key.or(settings.steam_web_key);
	settings.slackurl = args.flag_slackurl.or(settings.slackurl);
	settings.slackname = args.flag_slackname.or(settings.slackname);
	settings.discordurl = args.flag_discordurl.or(settings.discordurl);

	let logname = settings.logname.clone().unwrap_or("rust".to_string());
	let loglevel = settings.loglevel.unwrap_or(epicinium::log::Level::Verbose);
	epicinium::log::start(&logname, loglevel)?;
	let log_setup = epicinium::logrotate::setup(&logname)?;

	let currentversion = Version::current();

	println!("[ Epicinium Server ] ({} v{})", logname, currentversion);
	println!("");

	info!("Server started.");

	let server = match setup_server(&settings, log_setup)
	{
		Ok(server) => server,
		Err(error) =>
		{
			error!("Error running server: {}", error);
			error!("{:#?}", error);
			println!("Error running server: {}", error);
			return Err(error);
		}
	};

	run_server(server);

	info!("Server stopped.");

	println!("");
	println!("[ Done ]");
	Ok(())
}
