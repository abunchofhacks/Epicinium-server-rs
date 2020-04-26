/* Server */

extern crate epicinium;

use epicinium::*;

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

	println!(
		"[ Epicinium Server ] ({} v{})",
		logname,
		currentversion.to_string()
	);
	println!("");

	run_server(&settings)?;

	println!("");
	println!("[ Done ]");
	Ok(())
}
