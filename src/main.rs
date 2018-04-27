mod logic;

use logic::player::Player;

fn main()
{
    println!("Size of Player: {}", std::mem::size_of::<Player>()) ;
}
