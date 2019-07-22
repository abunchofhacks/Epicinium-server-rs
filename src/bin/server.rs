/* Server */

extern crate epicinium;

use epicinium::*;

use std::io;

fn main() -> io::Result<()>
{
	let logname = "rust".to_string();
	let currentversion = Version::current();

	println!(
		"[ Epicinium Server ] ({} v{})",
		logname,
		currentversion.to_string()
	);
	println!("");

	{
		let mut server = ServerCluster::create()?;
		server.run()?;
	}

	println!("");
	println!("[ Done ]");

	Ok(())
}
