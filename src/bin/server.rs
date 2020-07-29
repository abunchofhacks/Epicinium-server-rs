/* Server */

use epicinium::run_server;
use epicinium::Settings;
use epicinium::Version;

use log::info;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>>
{
	let mut logname = "rust".to_string();
	let currentversion = Version::current();

	let mut settings = Settings::create("settings-server.json")?;

	match settings.logname()
	{
		Some(name) =>
		{
			logname = name.to_string();
		}
		None =>
		{
			settings.override_logname(logname.clone());
		}
	}

	let loglevel = settings
		.loglevel()
		.unwrap_or(epicinium::log::Level::Verbose);
	epicinium::log::start(&logname, loglevel)?;
	epicinium::logic::log_initialize(loglevel);
	let log_setup = epicinium::logrotate::setup(&logname)?;

	println!("[ Epicinium Server ] ({} v{})", logname, currentversion);
	println!("");

	info!("Server started.");

	run_server(&settings, log_setup)?;

	info!("Server stopped.");

	println!("");
	println!("[ Done ]");
	Ok(())
}
