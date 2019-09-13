/* Server::Test::Counting */

use common::version::*;
use server::limits;
use server::settings::*;

use std::error;

pub fn run(settings: &Settings) -> Result<(), Box<dyn error::Error>>
{
	limits::enable_coredumps()?;

	let mut ntests: usize = 2;
	let mut fakeversion: Version = Version::current();

	for arg in std::env::args().skip(1)
	{
		if arg.starts_with("-")
		{
			// Setting argument, will be handled by Settings.
		}
		else if arg.starts_with("v")
		{
			fakeversion = arg.parse()?;
		}
		else
		{
			ntests = arg.parse()?;
		}
	}

	println!(
		"ntests = {}, fakeversion = v{}, port = {}",
		ntests,
		fakeversion.to_string(),
		settings.get_port()?,
	);

	Ok(())
}
