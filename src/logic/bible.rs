/* Bible */

use std::collections::HashMap;
use logic::unit::UnitType;
use logic::tile::TileType;
use logic::cycle::Season;
use common::version::Version;


#[derive(Default, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Bible
{
	pub version : Version,

	pub tile_accessible : HashMap<TileType, bool>,
	pub tile_walkable : HashMap<TileType, bool>,
	pub tile_buildable : HashMap<TileType, bool>,
	pub tile_destructible : HashMap<TileType, bool>,
	pub tile_grassy : HashMap<TileType, bool>,
	pub tile_natural : HashMap<TileType, bool>,
	pub tile_laboring : HashMap<TileType, bool>,
	pub tile_energizing : HashMap<TileType, bool>,
	pub tile_powered : HashMap<TileType, bool>,
	pub tile_ownable : HashMap<TileType, bool>,
	pub tile_controllable : HashMap<TileType, bool>,
	pub tile_autocultivates : HashMap<TileType, bool>,
	pub tile_plane : HashMap<TileType, bool>,

	pub tile_stacks_built : HashMap<TileType, i8>,
	pub tile_stacks_max : HashMap<TileType, i8>,
	pub tile_power_built : HashMap<TileType, i8>,
	pub tile_power_max : HashMap<TileType, i8>,
	pub tile_vision : HashMap<TileType, i8>,
	pub tile_hitpoints : HashMap<TileType, i8>,
	pub tile_income : HashMap<TileType, i8>,
	pub tile_leak_gas : HashMap<TileType, i8>,
	pub tile_leak_rads : HashMap<TileType, i8>,
	pub tile_emit_chaos : HashMap<TileType, i8>,

	pub tile_produces : HashMap<TileType, Vec<UnitBuild>>,
	pub tile_expands : HashMap<TileType, Vec<TileBuild>>,
	pub tile_upgrades : HashMap<TileType, Vec<TileBuild>>,
	pub tile_cultivates : HashMap<TileType, Vec<TileBuild>>,

	pub tile_score_base : HashMap<TileType, i16>,
	pub tile_score_stack : HashMap<TileType, i16>,

	pub tile_destroyed : HashMap<TileType, TileType>,

	pub tile_expand_range_min : i8,
	pub tile_expand_range_max : i8,
	pub tile_produce_range_min : i8,
	pub tile_produce_range_max : i8,

	pub temperature_min_hotdeath : HashMap<Season, i8>,
	pub temperature_min_firestorm : HashMap<Season, i8>,
	pub temperature_min_aridification : HashMap<Season, i8>,
	pub temperature_max_comfortable : HashMap<Season, i8>,
	pub temperature_min_comfortable : HashMap<Season, i8>,
	pub temperature_max_snow : HashMap<Season, i8>,
	pub temperature_max_frostbite : HashMap<Season, i8>,
	pub temperature_max_colddeath : HashMap<Season, i8>,
}
// TODO replace HashMap with some sort of EnumMap if possible
// TODO compile time bounds checking on these maps?
// TODO replace Vec with bit_set::BitSet once the serde pull request is merged

/*
impl Default for Bible
{
	fn default() -> Bible
	{
		_unimplemented
	}
}

impl Deserialize for Bible
{
	fn deserialize()
	{
		_unimplemented
	}
}
*/

#[derive(Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct TileBuild
{
	pub typ : TileType,
	pub cost : i16,
}

#[derive(Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct UnitBuild
{
	pub typ : UnitType,
	pub cost : i16,
}
