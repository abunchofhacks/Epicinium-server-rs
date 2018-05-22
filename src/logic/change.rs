/* Change */

use logic::header::*;
use logic::player::*;
use logic::position::*;
use logic::descriptor::*;
use logic::tile::*;
use logic::unit::*;
use logic::cycle::*;
use logic::order::*;
use logic::vision::*;


#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum Change
{
	// look at <subject> cell/tile/unit and see no changes
	NONE
	{
		subject: Descriptor,

		#[serde(default, skip_serializing_if = "is_zero")]
		target: Descriptor,

		#[serde(default, skip_serializing_if = "is_zero")]
		notice: Notice,
	},
	// <subject> unit starts to move to <target> cellslot
	STARTS
	{
		subject: Descriptor,
		target: Descriptor,
	},
	// <subject> unit moved to <target> cell
	MOVES
	{
		subject: Descriptor,
		target: Descriptor,
	},
	// reveal <subject> cell from fog of war and set <tile>, <snow>, <gas>, ...
	REVEAL
	{
		subject: Descriptor,
		tile: TileToken,

		#[serde(default, skip_serializing_if = "is_zero")]
		snow: bool,

		#[serde(default, skip_serializing_if = "is_zero")]
		frostbite: bool,

		#[serde(default, skip_serializing_if = "is_zero")]
		firestorm: bool,

		#[serde(default, skip_serializing_if = "is_zero")]
		bonedrought: bool,

		#[serde(default, skip_serializing_if = "is_zero")]
		death: bool,

		#[serde(default, skip_serializing_if = "is_zero")]
		gas: i8,

		#[serde(default, skip_serializing_if = "is_zero")]
		radiation: i8,

		#[serde(default, skip_serializing_if = "is_zero")]
		temperature: i8,

		#[serde(default, skip_serializing_if = "is_zero")]
		humidity: i8,

		#[serde(default, skip_serializing_if = "is_zero")]
		chaos: i8,
	},
	// cover <subject> cell in fog of war
	OBSCURE
	{
		subject: Descriptor,
	},
	// <subject> tile was naturally transformed into <tile>
	TRANSFORMED
	{
		subject: Descriptor,
		tile: TileToken,
	},
	// <subject> tile was consumed into <tile>
	CONSUMED
	{
		subject: Descriptor,
		tile: TileToken,
	},
	// <subject> unit shapes a tile
	SHAPES
	{
		subject: Descriptor,
	},
	// <subject> tile was shaped into <tile>
	SHAPED
	{
		subject: Descriptor,
		tile: TileToken,
	},
	// <subject> unit settles a tile and is removed
	SETTLES
	{
		subject: Descriptor,
	},
	// <subject> tile was settled into <tile>
	SETTLED
	{
		subject: Descriptor,
		tile: TileToken,
	},
	// <subject> tile expands a tile in <target> cell and spends <power>
	EXPANDS
	{
		subject: Descriptor,
		power: i8,
	},
	// <subject> tile was expanded into <tile>
	EXPANDED
	{
		subject: Descriptor,
		tile: TileToken,
	},
	// <subject> tile upgrades itself and spends <power>
	UPGRADES
	{
		subject: Descriptor,
		power: i8,
	},
	// <subject> tile was upgraded into <tile>
	UPGRADED
	{
		subject: Descriptor,
		tile: TileToken,
	},
	// <subject> tile cultivates tiles around itself spends <power>
	CULTIVATES
	{
		subject: Descriptor,
		power: i8,
	},
	// <subject> tile was cultivated into <tile>
	CULTIVATED
	{
		subject: Descriptor,
		tile: TileToken,
	},
	// <subject> unit captures a tile
	CAPTURES
	{
		subject: Descriptor,
	},
	// <subject> tile was captured now belongs to <player>
	CAPTURED
	{
		subject: Descriptor,
		player: Player,
	},
	// <subject> tile produces a unit and spends <power>
	PRODUCES
	{
		subject: Descriptor,
		power: i8,
	},
	// <subject> unit is a newly produced <unit>
	PRODUCED
	{
		subject: Descriptor,
		unit: UnitToken,
	},
	// <subject> unit is a new <unit> that appeared out of fog of war
	ENTERED
	{
		subject: Descriptor,
		unit: UnitToken,
	},
	// <subject> unit disappeared in the fog of war
	EXITED
	{
		subject: Descriptor,
	},
	// <subject> unit died because it lost all its figures
	DIED
	{
		subject: Descriptor,
	},
	// <subject> tile was destroyed into <tile>
	DESTROYED
	{
		subject: Descriptor,
		tile: TileToken,
	},
	// <subject> unit survived a damage step
	SURVIVED
	{
		subject: Descriptor,
	},
	// <subject> unit aims at something in <target> cell
	AIMS
	{
		subject: Descriptor,
		target: Descriptor,

		#[serde(default, skip_serializing_if = "is_zero")]
		notice: Notice,
	},
	// <subject> unit's <figure> fires upon something in <target> cell
	ATTACKS
	{
		subject: Descriptor,
		target: Descriptor,
		figure: i8,
	},
	// <subject> tile/unit's <figure> was hit by <attacker> and is <killed>
	ATTACKED
	{
		subject: Descriptor,
		attacker: Attacker,
		figure: i8,
		killed: bool,
		depowered: bool,
	},
	// <subject> unit's <figure> tramples its cell
	TRAMPLES
	{
		subject: Descriptor,
		figure: i8,
	},
	// <subject> tile/unit's <figure> was hit by <bombarder> and is <killed>
	TRAMPLED
	{
		subject: Descriptor,
		bombarder: Bombarder,
		figure: i8,
		killed: bool,
		depowered: bool,
	},
	// <subject> unit's <figure> fires upon something in <target> cell
	SHELLS
	{
		subject: Descriptor,
		target: Descriptor,
		figure: i8,
	},
	// <subject> tile/unit's <figure> was hit by <attacker> and is <killed>
	SHELLED
	{
		subject: Descriptor,
		attacker: Attacker,
		figure: i8,
		killed: bool,
		depowered: bool,
	},
	// <subject> unit's <figure> bombards some cell
	BOMBARDS
	{
		subject: Descriptor,
		figure: i8,
	},
	// <subject> tile/unit's <figure> was hit by <bombarder> and is <killed>
	BOMBARDED
	{
		subject: Descriptor,
		bombarder: Bombarder,
		figure: i8,
		killed: bool,
		depowered: bool,
	},
	// <subject> unit's <figure> bombs its cell
	BOMBS
	{
		subject: Descriptor,
		figure: i8,
	},
	// <subject> tile/unit's <figure> was hit by <bombarder> and is <killed>
	BOMBED
	{
		subject: Descriptor,
		bombarder: Bombarder,
		figure: i8,
		killed: bool,
		depowered: bool,
	},
	// <subject> tile/unit's <figure> was hit by frostbite and is <killed>
	FROSTBITTEN
	{
		subject: Descriptor,
		figure: i8,
		killed: bool,
		depowered: bool,
	},
	// <subject> tile/unit's <figure> was hit by firestorm and is <killed>
	BURNED
	{
		subject: Descriptor,
		figure: i8,
		killed: bool,
		depowered: bool,
	},
	// <subject> tile/unit's <figure> was hit by gas and is <killed>
	GASSED
	{
		subject: Descriptor,
		figure: i8,
		killed: bool,
		depowered: bool,
	},
	// <subject> tile/unit's <figure> was hit by radiation and is <killed>
	IRRADIATED
	{
		subject: Descriptor,
		figure: i8,
		killed: bool,
		depowered: bool,
	},
	// <subject> tile gains <stacks> and <power>
	GROWS
	{
		subject: Descriptor,
		stacks: i8,
		power: i8,
	},
	// set <snow> of <subject> cell
	SNOW
	{
		subject: Descriptor,
		snow: bool,
	},
	// set <frostbite> of <subject> cell
	FROSTBITE
	{
		subject: Descriptor,
		frostbite: bool,
	},
	// set <firestorm> of <subject> cell
	FIRESTORM
	{
		subject: Descriptor,
		firestorm: bool,
	},
	// set <bonedrought> of <subject> cell
	BONEDROUGHT
	{
		subject: Descriptor,
		bonedrought: bool,
	},
	// set <death> of <subject> cell
	DEATH
	{
		subject: Descriptor,
		death: bool,
	},
	// change <gas> of <subject> cell
	GAS
	{
		subject: Descriptor,
		gas: i8,
	},
	// change <radiation> of <subject> cell
	RADIATION
	{
		subject: Descriptor,
		radiation: i8,
	},
	// change <temperature> of <subject> cell
	TEMPERATURE
	{
		subject: Descriptor,
		temperature: i8,
	},
	// change <humidity> of <subject> cell
	HUMIDITY
	{
		subject: Descriptor,
		humidity: i8,
	},
	// change <chaos> of <subject> cell
	CHAOS
	{
		subject: Descriptor,
		chaos: i8,
	},
	// global warming has reached <level>
	CHAOSREPORT
	{
		subject: Descriptor,
		level: i8,
	},
	// set the year to <year>
	YEAR
	{
		subject: Descriptor,
		year: i16,
	},
	// set the season to <season>
	SEASON
	{
		subject: Descriptor,
		season: Season,
	},
	// set the daytime to <daytime>
	DAYTIME
	{
		subject: Descriptor,
		daytime: Daytime,
	},
	// set the phase to <phase>
	PHASE
	{
		subject: Descriptor,
		phase: Phase,
	},
	// <player> has <initiative> (1st, 2nd, etc)
	INITIATIVE
	{
		subject: Descriptor,

		#[serde(default)]
		player: Player,

		initiative: i8,
	},
	// you/<player> gain <money> funds
	FUNDS
	{
		#[serde(default)]
		player: Player,

		money: i16,
	},
	// you/<player> gain <money> income from <subject> tile
	INCOME
	{
		subject: Descriptor,

		#[serde(default)]
		player: Player,

		money: i16,
	},
	// you/<player> gain <money> from expenditures of <subject> tile/unit
	EXPENDITURE
	{
		subject: Descriptor,

		#[serde(default)]
		player: Player,

		money: i16,
	},
	// your orders are delayed because you gave a sleep order
	SLEEPING
	{},
	// your <subject> is currently acting out its order
	ACTING
	{
		subject: Descriptor,
		order: Order,
	},
	// your order assigned to <subject> was finished
	FINISHED
	{
		subject: Descriptor,
	},
	// your order assigned to <subject> was discarded
	DISCARDED
	{
		subject: Descriptor,
	},
	// your <subject> still has <order> because it was postponed
	POSTPONED
	{
		subject: Descriptor,
		order: Order,
	},
	// your <subject> still has <order> and will continue later
	UNFINISHED
	{
		subject: Descriptor,
		order: Order,
	},
	// update the <vision> of <subject> cell
	VISION
	{
		subject: Descriptor,
		vision: Vision,
	},
	// declare <subject> to be the bottom right corner
	CORNER
	{
		subject: Descriptor,
	},
	// declare that the entire map has been announced
	BORDER
	{},
	// <player> gained <score> from <subject>
	SCORED
	{
		subject: Descriptor,
		player: Player,
		score: i16,
	},
	// <player> was defeated and got <score>
	DEFEAT
	{
		player: Player,
		score: i16,
	},
	// <player> was victorious and got <score>
	VICTORY
	{
		player: Player,
		score: i16,
	},
	// the game has ended; the world was worth <score>
	GAMEOVER
	{
		score: i16,
	},
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Notice
{
	NONE = 0,
	DESTINATIONOCCUPIED,
	SUBJECTOCCUPIED,
	TARGETOCCUPIED,
	NOTARGET,
	LACKINGSTACKS,
	LACKINGPOWER,
	LACKINGMONEY,
	ACTIVEATTACK,
	RETALIATIONATTACK,
	FOCUSATTACK,
	TRIGGEREDFOCUSATTACK,
	OPPORTUNITYATTACK,
}

impl Default for Notice
{
	fn default() -> Notice { Notice::NONE }
}

#[derive(Default, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Attacker
{
	#[serde(rename = "type")]
	pub typ: UnitType,

	pub position: Position,
}

#[derive(Default, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Bombarder
{
	#[serde(rename = "type")]
	pub typ: UnitType,
}

