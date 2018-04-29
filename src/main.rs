/* Main */

#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;

mod logic;

use logic::*;


fn main()
{
	println!("Size of TileToken: {}", std::mem::size_of::<TileToken>());
	let x = TileToken {
		typ: TileType::FOREST,
		stacks: 4,
		.. TileToken::default()};
	let txt = serde_json::to_string(&x).unwrap();
	println!("{:?}: {}", x, txt);
	let y : TileToken = serde_json::from_str(&txt).unwrap();
	println!("{} => {:?}", txt, y);
}
