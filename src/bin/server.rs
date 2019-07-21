/* Server */

extern crate epicinium;

use epicinium::*;

fn main() -> std::result::Result<(), std::io::Error>
{
	let logname = String::from("rust");
	let currentversion = Version::current();

	println!(
		"[ Epicinium Server ] ({} v{})",
		logname,
		currentversion.to_string()
	);
	println!("");

	{
		let mut server = ServerCluster::create();
		server.run()?;
	}

	println!("");
	println!("[ Done ]");

	Ok(())
}
