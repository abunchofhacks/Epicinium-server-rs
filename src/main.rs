/* Main */

extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

extern crate enum_map;

#[macro_use]
extern crate enum_map_derive;

mod common;

#[allow(dead_code)]
mod logic;

use logic::*;
use common::*;


fn main()
{
	println!("Size of Bible: {}", std::mem::size_of::<Bible>());
	println!("Size of Version: {}", std::mem::size_of::<Version>());
	println!("Size of Board: {}", std::mem::size_of::<Board>());
	println!("Size of Space: {}", std::mem::size_of::<Space>());
	println!("Size of Change: {}", std::mem::size_of::<Change>());
	println!("Size of Order: {}", std::mem::size_of::<Order>());
	println!("Size of Move: {}", std::mem::size_of::<Move>());
	println!("Size of Vec<Move>: {}", std::mem::size_of::<Vec<Move>>());
	let x = Bible::current();
	let txt = serde_json::to_string(& x).unwrap();
	println!("{:?}: {}", x, txt);
	let y : Bible = serde_json::from_str(& txt).unwrap();
	println!("{} => {:?}", txt, y);
	assert!(x == y);
}
