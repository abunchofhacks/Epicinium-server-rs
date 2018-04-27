/* Main */

#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;

mod logic;

use logic::player::Player;

fn main()
{
    println!("Size of Player: {}", std::mem::size_of::<Player>()) ;
    let x = Player::TEAL;
    println!("{:?}: {}", x, serde_json::to_string(&x).unwrap());
    let txt = "\"teal\"";
    let y : Player = serde_json::from_str("\"teal\"").unwrap();
    println!("{} => {:?}", txt, y);
}
