/* Server::Test::Counting */

use common::coredump;
use common::version::*;
use server::settings::*;

use std::error;

use rand::seq::SliceRandom;
use tokio::prelude::*;

pub fn run(settings: &Settings) -> Result<(), Box<dyn error::Error>>
{
	coredump::enable_coredumps()?;

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

	let server = settings.get_server()?;
	let port = settings.get_port()?;

	println!(
		"ntests = {}, fakeversion = v{}, server = {}, port = {}",
		ntests,
		fakeversion.to_string(),
		server,
		port,
	);

	// TODO seed

	let mut numbers: Vec<usize> = (0..ntests).collect();
	let mut rng = rand::thread_rng();
	numbers.shuffle(&mut rng);

	let tests = numbers
		.iter()
		.map(|&number| start_test(number, fakeversion, server, port));
	let all_tests = stream::futures_unordered(tests).fold((), |(), ()| Ok(()));

	tokio::run(all_tests);
	Ok(())
}

fn start_test(
	number: usize,
	fakeversion: Version,
	server: &str,
	port: u16,
) -> impl Future<Item = (), Error = ()> + Send
{
	println!("[{}] Connecting to {}:{}...", number, server, port);
	println!(
		"[{}] Pretending to be version {}...",
		number,
		fakeversion.to_string()
	);

	// TODO
	future::ok(())
}
