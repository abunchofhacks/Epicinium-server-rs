/* Main */

#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;

mod logic;

use logic::*;

fn main()
{
	println!("Size of UnitToken: {}", std::mem::size_of::<UnitToken>());
	let x = UnitToken {
		typ: UnitType::RIFLEMAN,
		owner: Player::TEAL,
		stacks: 3,
		.. UnitToken::default()};
	let txt = serde_json::to_string(&x).unwrap();
	println!("{:?}: {}", x, txt);
	let y : Player = serde_json::from_str("\"teal\"").unwrap();
	println!("{} => {:?}", txt, y);
}
