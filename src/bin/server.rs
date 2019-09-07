/* Server */

extern crate epicinium;

use epicinium::*;

fn main()
{
	let mut logname = "rust".to_string();
	let currentversion = Version::current();

	let mut settings = match Settings::create("settings-server.json")
	{
		Ok(settings) => settings,
		Err(e) =>
		{
			eprintln!("Error while loading the settings: {}", e);
			return;
		}
	};

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

	match run_server(&settings)
	{
		Ok(()) =>
		{
			println!("");
			println!("[ Done ]");
		}
		Err(e) =>
		{
			eprintln!("Error while running the server: {}", e);
			return;
		}
	}
}
