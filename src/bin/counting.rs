/* Counting Stress Test */

extern crate epicinium;

use epicinium::*;

fn main()
{
	let mut logname = "counting".to_string();
	let currentversion = Version::current();

	let mut settings = match Settings::create("settings-counting.json")
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
		"[ Epicinium Counting Stress Test ] ({} v{})",
		logname,
		currentversion.to_string()
	);
	println!("");

	match server::countingtest::run(&settings)
	{
		Ok(()) =>
		{
			println!("");
			println!("[ Done ]");
		}
		Err(e) =>
		{
			eprintln!("Error while running stress test: {}", e);
			return;
		}
	}
}
