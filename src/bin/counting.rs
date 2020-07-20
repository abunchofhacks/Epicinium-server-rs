/* Counting Stress Test */

extern crate epicinium;

use epicinium::*;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>>
{
	let mut logname = "counting".to_string();
	let currentversion = Version::current();

	let mut settings = Settings::create("settings-counting.json")?;

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
	log::start(&logname, loglevel)?;

	println!(
		"[ Epicinium Counting Stress Test ] ({} v{})",
		logname, currentversion
	);
	println!("");

	server::countingtest::run(&settings)?;

	println!("");
	println!("[ Done ]");
	Ok(())
}
