/* Main */

#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;

#[allow(dead_code)]
mod logic;

use logic::*;


fn main()
{
	println!("Size of Board: {}", std::mem::size_of::<Board>());
	println!("Size of Space: {}", std::mem::size_of::<Space>());
	println!("Size of Change: {}", std::mem::size_of::<Change>());
	println!("Size of Order: {}", std::mem::size_of::<Order>());
	println!("Size of Move: {}", std::mem::size_of::<Move>());
	println!("Size of Vec<Move>: {}", std::mem::size_of::<Vec<Move>>());
	let x = Order::MOVE	{
		subject: Descriptor {
			typ: descriptor::Type::GROUND,
			position: Position {
				row: 0,
				col: 0,
			}
		},
		target: Descriptor {
			typ: descriptor::Type::GROUND,
			position: Position {
				row: 2,
				col: 3,
			}
		},
		moves: vec![
			order::Move::EAST,
			order::Move::SOUTH,
			order::Move::SOUTH,
			order::Move::EAST,
			order::Move::EAST
		],
	};
	let txt = serde_json::to_string(& x).unwrap();
	println!("{:?}: {}", x, txt);
	let y : Order = serde_json::from_str(& txt).unwrap();
	println!("{} => {:?}", txt, y);
}
