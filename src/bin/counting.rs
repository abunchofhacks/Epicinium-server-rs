/* Counting Stress Test */

use epicinium::server;
use epicinium::Settings;
use epicinium::Version;

use serde::Deserialize;

use anyhow::anyhow;

use docopt::Docopt;

const USAGE: &'static str = "
Usage: counting <num-tests> [<fake-version>] [options]

Options:
	--logname=NAME               The name used in the filenames of logs.
	--loglevel=LEVEL             The level to filter on when writing logs.
	--server=IPADDRESS           The IP address of the server to connect to.
	--port=PORT                  The port to connect to.
	--settings=FILENAME          Filename to load additional settings from.
";

#[derive(Deserialize)]
struct Args
{
	arg_num_tests: usize,
	arg_fake_version: Option<Version>,
	flag_logname: Option<String>,
	flag_loglevel: Option<epicinium::common::log::Level>,
	flag_server: Option<String>,
	flag_port: Option<u16>,
	flag_settings: Option<String>,
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
		.unwrap_or("settings-counting.json".to_string());
	let mut settings = Settings::load(&settings_filename)?;

	settings.logname = args.flag_logname.or(settings.logname);
	settings.loglevel = args.flag_loglevel.or(settings.loglevel);

	let logname = settings.logname.clone().unwrap_or("counting".to_string());
	let loglevel = settings.loglevel.unwrap_or(epicinium::log::Level::Verbose);
	epicinium::log::start(&logname, loglevel)?;

	let currentversion = Version::current();

	println!(
		"[ Epicinium Counting Stress Test ] ({} v{})",
		logname, currentversion
	);
	println!("");

	let num_tests = args.arg_num_tests;
	let fake_version = args.arg_fake_version.unwrap_or(currentversion);
	let server = args
		.flag_server
		.or(settings.server)
		.ok_or_else(|| anyhow!("Missing 'server'"))?;
	let port = args
		.flag_port
		.or(settings.port)
		.ok_or_else(|| anyhow!("Missing 'port'"))?;

	server::countingtest::run(num_tests, fake_version, server, port)?;

	println!("");
	println!("[ Done ]");
	Ok(())
}
