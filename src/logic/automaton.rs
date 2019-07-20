/* Automaton */

use std::collections::VecDeque;
use std::collections::HashMap;
use enum_map::EnumMap;
use logic::change::*;
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

	activeplayers : VecDeque<Player>,
	activeorders : EnumMap<Player, VecDeque<Order>>,
	unfinishedorders : EnumMap<Player, VecDeque<Order>>,
	activesubjects : HashMap<u32, Descriptor>,
	unfinishedsubjects : HashMap<u32, Descriptor>,
	changesets : VecDeque<ChangeSet>

	// TODO recording
	// TODO replay
	// TODO challenge
}

impl Automaton
{
	pub fn initialize(&mut self, players : Vec<Player>, bible : Bible)
	{
		self.players = players;
		// TODO erase NONE

		self.bible = bible;

		self.gameover = false;
		self.round = 0;
		self.year = 1;
		self.season = Season::SPRING;
		self.daytime = Daytime::LATE;
		self.phase = Phase::GROWTH;
	}

	// TODO param metadata
	// TODO param enableRecording
	pub fn load(&mut self, map : & Map, shufflePlayers : bool)
	{
		self.board.load(map);

		if (shufflePlayers)
		{
			// TODO shuffle self.players

			self.board.assignPlayers(self.players);
		}
		else
		{
			self.board.setPlayers(self.players);
		}

		// TODO finish
	}
}
