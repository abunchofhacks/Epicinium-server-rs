/* Server */

extern crate epicinium;

use epicinium::*;

use std::io;

fn main() -> io::Result<()>
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
			// TODO settings.override_logname(logname);
		}
	}

	println!(
		"[ Epicinium Server ] ({} v{})",
		logname,
		currentversion.to_string()
	);
	println!("");

	{
		run_server()?;
	}

	println!("");
	println!("[ Done ]");

	Ok(())
}
