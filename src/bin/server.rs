/* Server */

extern crate epicinium;
extern crate log;

use epicinium::*;

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

	epicinium::log::start(&logname)?;
	epicinium::logic::log_initialize();

	println!("[ Epicinium Server ] ({} v{})", logname, currentversion);
	println!("");

	info!("Server started.");

	run_server(&settings)?;

	info!("Server stopped.");

	println!("");
	println!("[ Done ]");
	Ok(())
}
