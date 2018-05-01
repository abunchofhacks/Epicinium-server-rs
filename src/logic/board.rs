/* Board */


use logic::space::*;

#[derive(Debug)]
pub struct Board
{
	pub cols : i32,
	pub rows : i32,

	pub edge : Space,
	pub cells : Vec<Space>,
}
