/* Server */

extern crate epicinium;

use epicinium::*;


fn main()
{
	let logname = String::from("rust");
	let currentversion = Version::current();
	println!("[ Epicinium Server ] ({} v{})",
		logname,
		currentversion.to_string());
}
