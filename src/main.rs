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

use std::fs::File;
use std::io::prelude::*;


fn main()
{
	println!("Size of Automaton: {}", std::mem::size_of::<Automaton>());
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
	let y : Bible = serde_json::from_str(& txt).unwrap();
	{
		let mut file = File::create("x.out").unwrap();
		let _result = write!(file, "{:?}", x);
	}
	{
		let mut file = File::create("v0.23.0.out").unwrap();
		let _result = write!(file, "{}", txt);
	}
	{
		let mut file = File::create("y.out").unwrap();
		let _result = write!(file, "{:?}", y);
	}
	assert!(x == y);
}
