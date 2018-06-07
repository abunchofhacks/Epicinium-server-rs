/* Automaton */

use std;
use enum_map::EnumMap;
use logic::space::*;
use logic::change::*;
use logic::position::*;
use logic::unit::*;
use logic::player::*;
use logic::descriptor::*;
use logic::board::*;
use logic::order::*;
use logic::cycle::*;
use logic::bible::*;


#[derive(Debug)]
pub struct Automaton
{
	players : Vec<Player>,
	visionaries : Vec<Player>,
	money : EnumMap<Player, i16>,
	initiative : EnumMap<Player, i8>,
	orderlist : EnumMap<Player, Vec<Order>>,
	citybound : EnumMap<Player, bool>,
	defeated : EnumMap<Player, bool>,
	score : EnumMap<Player, i16>,

	bible : Bible,
	board : Board,

	gameover : bool,
	round : u32,
	year : i16,
	season : Season,
	daytime : Daytime,
	phase : Phase,
}

impl Automaton
{

}
