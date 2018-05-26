/* Bible */

use enum_map::EnumMap;
use std::ops::Index;
use std::ops::IndexMut;
use serde::Serialize;
use serde::Serializer;
use serde::ser::SerializeMap;
use serde::Deserialize;
use serde::Deserializer;
use serde::de::Visitor;
use serde::de::MapAccess;
use std::marker::PhantomData;
use std::fmt;

use logic::unit::UnitType;
use logic::tile::TileType;
use logic::cycle::Season;
use common::version::Version;


#[derive(Default, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct Bible
{
	pub version : Version,

	/* TILES */
	pub tileAccessible : TileMap<bool>,
	pub tileWalkable : TileMap<bool>,
	pub tileBuildable : TileMap<bool>,
	pub tileDestructible : TileMap<bool>,
	pub tileGrassy : TileMap<bool>,
	pub tileNatural : TileMap<bool>,
	pub tileLaboring : TileMap<bool>,
	pub tileEnergizing : TileMap<bool>,
	pub tilePowered : TileMap<bool>,
	pub tileOwnable : TileMap<bool>,
	pub tileControllable : TileMap<bool>,
	pub tileAutoCultivates : TileMap<bool>,
	pub tilePlane : TileMap<bool>,

	pub tileStacksBuilt : TileMap<i8>,
	pub tileStacksMax : TileMap<i8>,
	pub tilePowerBuilt : TileMap<i8>,
	pub tilePowerMax : TileMap<i8>,
	pub tileVision : TileMap<i8>,
	pub tileHitpoints : TileMap<i8>,
	pub tileIncome : TileMap<i8>,
	pub tileLeakGas : TileMap<i8>,
	pub tileLeakRads : TileMap<i8>,
	pub tileEmitChaos : TileMap<i8>,

	pub tileProduces : TileMap<Vec<UnitBuild>>,
	pub tileExpands : TileMap<Vec<TileBuild>>,
	pub tileUpgrades : TileMap<Vec<TileBuild>>,
	pub tileCultivates : TileMap<Vec<TileBuild>>,

	pub tileScoreBase : TileMap<i16>,
	pub tileScoreStack : TileMap<i16>,

	pub tileDestroyed : TileMap<TileType>,

	pub tileExpandRangeMin : i8,
	pub tileExpandRangeMax : i8,
	pub tileProduceRangeMin : i8,
	pub tileProduceRangeMax : i8,

	/* UNITS */
	pub unitAir : UnitMap<bool>,
	pub unitInfantry : UnitMap<bool>,
	pub unitMechanical : UnitMap<bool>,
	pub unitCanMove : UnitMap<bool>,
	pub unitCanAttack : UnitMap<bool>,
	pub unitCanGuard : UnitMap<bool>,
	pub unitCanFocus : UnitMap<bool>,
	pub unitCanShell : UnitMap<bool>,
	pub unitCanBombard : UnitMap<bool>,
	pub unitCanBomb : UnitMap<bool>,
	pub unitCanCapture : UnitMap<bool>,
	pub unitCanOccupy : UnitMap<bool>,

	pub unitStacksMax : UnitMap<i8>,
	pub unitSpeed : UnitMap<i8>,
	pub unitVision : UnitMap<i8>,
	pub unitHitpoints : UnitMap<i8>,
	pub unitAttackShots : UnitMap<i8>,
	pub unitAttackDamage : UnitMap<i8>,
	pub unitTrampleShots : UnitMap<i8>,
	pub unitTrampleDamage : UnitMap<i8>,
	pub unitAbilityShots : UnitMap<i8>,
	pub unitAbilityVolleys : UnitMap<i8>,
	pub unitAbilityDamage : UnitMap<i8>,
	pub unitAbilityGas : UnitMap<i8>,
	pub unitAbilityRads : UnitMap<i8>,
	pub unitAbilityRadius : UnitMap<i8>,
	pub unitRangeMin : UnitMap<i8>,
	pub unitRangeMax : UnitMap<i8>,
	pub unitLeakGas : UnitMap<i8>,
	pub unitLeakRads : UnitMap<i8>,

	pub unitShapes : UnitMap<Vec<TileType>>,
	pub unitSettles : UnitMap<Vec<TileType>>,

	pub unitSizeMax : i8,
	pub unitVisionMax : i8,

	/* COMBAT */
	pub missCountGround : i8,
	pub missCountAir : i8,
	pub missCountTrenches : i8,
	pub missHitpointsGround : i8,
	pub missHitpointsAir : i8,
	pub missHitpointsTrenches : i8,

	/* WEATHER */
	pub seasonTemperatureSwing : SeasonMap<i8>,
	pub seasonGlobalWarmingFactor : SeasonMap<i8>,

	pub emissionDivisor : i8,
	pub forestGrowthProbabilityDivisor : i8,
	pub forestRegrowthProbabilityDivisor : i8,
	pub grassRegrowthProbabilityDivisor : i8,
	pub cropsRegrowthProbabilityDivisor : i8,

	pub groundPollutionAmount : i8,
	pub gasPollutionAmount : i8,
	pub aridificationAmountHumid : i8,
	pub aridificationAmountDegraded : i8,
	pub forestGrowthAmount : i8,
	pub forestRegrowthAmount : i8,

	pub aridificationRange : i8,

	pub aridificationCount : i8,
	pub firestormCount : i8,
	pub deathCount : i8,

	pub temperatureMax : i8,
	pub temperatureMin : i8,
	pub humidityMax : i8,
	pub humidityMin : i8,
	pub chaosMax : i8,
	pub chaosMin : i8,

	pub chaosThreshold : i8,

	pub temperatureMinHotDeath : SeasonMap<i8>,
	pub temperatureMinFirestorm : SeasonMap<i8>,
	pub temperatureMinAridification : SeasonMap<i8>,
	pub temperatureMaxComfortable : SeasonMap<i8>,
	pub temperatureMinComfortable : SeasonMap<i8>,
	pub temperatureMaxSnow : SeasonMap<i8>,
	pub temperatureMaxFrostbite : SeasonMap<i8>,
	pub temperatureMaxColdDeath : SeasonMap<i8>,

	pub humidityMinWet : SeasonMap<i8>,
	pub humidityMaxDegradation : SeasonMap<i8>,
	pub humidityMaxDesertification : SeasonMap<i8>,
	pub humidityMinSnow : SeasonMap<i8>,
	pub humidityMinFrostbite : SeasonMap<i8>,
	pub humidityMaxFirestorm : SeasonMap<i8>,
	pub humidityMaxBonedrought : SeasonMap<i8>,
	pub humidityMaxStonedrought : SeasonMap<i8>,
	pub humidityMaxDeath : SeasonMap<i8>,

	pub chaosMinDegradation : SeasonMap<i8>,
	pub chaosMinDesertification : SeasonMap<i8>,
	pub chaosMinAridification : SeasonMap<i8>,
	pub chaosMinSnow : SeasonMap<i8>,
	pub chaosMinFrostbite : SeasonMap<i8>,
	pub chaosMinFirestorm : SeasonMap<i8>,
	pub chaosMinBonedrought : SeasonMap<i8>,
	pub chaosMinStonedrought : SeasonMap<i8>,
	pub chaosMinDeath : SeasonMap<i8>,

	pub frostbiteShots : i8,
	pub frostbiteDamage : i8,
	pub frostbiteThresholdDamage : i8,
	pub frostbiteThresholdVulnerability : i8,
	pub firestormShots : i8,
	pub firestormDamage : i8,
	pub gasShots : i8,
	pub gasDamage : i8,
	pub gasThresholdDamage : i8,
	pub gasThresholdVulnerability : i8,
	pub radiationShots : i8,
	pub radiationDamage : i8,
	pub radiationThresholdDamage : i8,
	pub radiationThresholdVulnerability : i8,
	pub radiationThresholdDeath : i8,

	pub tempGenDefault : i8,
	pub humGenDefault : i8,
	pub tempGenGainRange : i8,
	pub humGenGainRange : i8,

	pub tempGenMountainGain : Vec<i8>,
	pub tempGenOceanGain : Vec<i8>,
	pub humGenLakeGain : Vec<i8>,
	pub humGenOceanGain : Vec<i8>,
	pub humGenDesertGain : Vec<i8>,
	pub humGenMountainGain : Vec<i8>,

	/* MECHANICS */
	pub separatePowerStages : bool,
	pub industryNicenessQuantitative : bool,
	pub reactorNicenessQuantitative : bool,
	pub grassOnlyRegrowsInSpring : bool,
	pub treesOnlyGrowInSpring : bool,
	pub cropsOnlyGrowInSpring : bool,
	pub cropsConsumedAtNight : bool,
	pub forestChaosProtectionPermanent : bool,
	pub collateralDamageKillsTiles : bool,
	pub gasOnlyTargetsGroundUnits : bool,
	pub frostbiteOnlyTargetsGroundUnits : bool,
	pub trenchesForceOccupy : bool,
	pub trenchesHideBypassedUnit : bool,
	pub captureStrengthCheck : bool,
	pub powerDrainScaled : bool,
	pub groundPollutionOnlyInAutumn : bool,
	pub counterBasedWeather : bool,
	pub quantitativeChaos : bool,
	pub stackBasedFrostbite : bool,
	pub emptyBasedFrostbite : bool,
	pub planeBasedFrostbite : bool,
	pub randomizedFirestorm : bool,
	pub randomizedAridification : bool,
	pub cumulativeDeath : bool,
	pub vulnerabilitiesStack : bool,
	pub markersChangeAtNight : bool,
	pub cityManualStackGrowth : bool,
	pub industryManualStackGrowth : bool,
	pub reactorManualStackGrowth : bool,
	pub publicInitiative : bool,

	pub snowSlowAmount : i8,
	pub snowSlowMaximum : i8,
	pub trenchesSlowAmount : i8,
	pub trenchesSlowMaximum : i8,

	pub startingIncome : i16,
	pub newOrderLimit : i16,
}
// TODO ensure that no out of bounds are generated when accessing
// TODO compile time bounds checking on these maps?

impl Bible
{
	pub fn current() -> Bible
	{
		let mut bible = Bible::default();

		bible.tileAccessible[TileType::GRASS] = true;
		bible.tileWalkable[TileType::GRASS] = true;
		bible.tileBuildable[TileType::GRASS] = true;
		bible.tileDestructible[TileType::GRASS] = true;
		bible.tileGrassy[TileType::GRASS] = true;
		bible.tileNatural[TileType::GRASS] = true;
		bible.tilePlane[TileType::GRASS] = true;
		bible.tileDestroyed[TileType::GRASS] = TileType::DIRT;
		bible.tileScoreBase[TileType::GRASS] = 1;

		bible
	}
}

/*
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

#[derive(Default, PartialEq, Eq, Debug)]
pub struct TileMap<T>(EnumMap<TileType, T>);

impl<T> Index<TileType> for TileMap<T>
{
	type Output = T;

	fn index(&self, key : TileType) -> &T
	{
		self.0.index(key)
	}
}

impl<T> IndexMut<TileType> for TileMap<T>
{
	fn index_mut(&mut self, key : TileType) -> &mut T
	{
		self.0.index_mut(key)
	}
}

impl <T> Serialize for TileMap<T>
	where T: Serialize
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut map = serializer.serialize_map(Some(self.0.len()))?;
		for (k, v) in self.0.iter()
		{
			map.serialize_entry(&k, &v)?;
		}
		map.end()
	}
}

impl <'de, T> Deserialize<'de> for TileMap<T>
	where T: Deserialize<'de> + Default
{
	fn deserialize<D>(deserializer : D) -> Result<Self, D::Error>
		where D: Deserializer<'de>
	{
		deserializer.deserialize_map(TileMapVisitor::new())
	}
}

struct TileMapVisitor<T> {
	marker: PhantomData<fn() -> TileMap<T>>
}

impl<T> TileMapVisitor<T> {
	fn new() -> Self {
		TileMapVisitor {
			marker: PhantomData
		}
	}
}

impl<'de, T> Visitor<'de> for TileMapVisitor<T>
where
	T: Deserialize<'de> + Default,
{
	type Value = TileMap<T>;

	fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		formatter.write_str("TileMap")
	}

	fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
	where
		M: MapAccess<'de>,
	{
		let mut map = TileMap::default();

		while let Some((key, value)) = access.next_entry()? {
			*(map.0.index_mut(key)) = value;
		}

		Ok(map)
	}
}

#[derive(Default, PartialEq, Eq, Debug)]
pub struct UnitMap<T>(EnumMap<UnitType, T>);

impl<T> Index<UnitType> for UnitMap<T>
{
	type Output = T;

	fn index(&self, key : UnitType) -> &T
	{
		self.0.index(key)
	}
}

impl<T> IndexMut<UnitType> for UnitMap<T>
{
	fn index_mut(&mut self, key : UnitType) -> &mut T
	{
		self.0.index_mut(key)
	}
}

impl <T> Serialize for UnitMap<T>
	where T: Serialize
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut map = serializer.serialize_map(Some(self.0.len()))?;
		for (k, v) in self.0.iter()
		{
			map.serialize_entry(&k, &v)?;
		}
		map.end()
	}
}

impl <'de, T> Deserialize<'de> for UnitMap<T>
	where T: Deserialize<'de> + Default
{
	fn deserialize<D>(deserializer : D) -> Result<Self, D::Error>
		where D: Deserializer<'de>
	{
		deserializer.deserialize_map(UnitMapVisitor::new())
	}
}

struct UnitMapVisitor<T> {
	marker: PhantomData<fn() -> UnitMap<T>>
}

impl<T> UnitMapVisitor<T> {
	fn new() -> Self {
		UnitMapVisitor {
			marker: PhantomData
		}
	}
}

impl<'de, T> Visitor<'de> for UnitMapVisitor<T>
where
	T: Deserialize<'de> + Default,
{
	type Value = UnitMap<T>;

	fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		formatter.write_str("UnitMap")
	}

	fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
	where
		M: MapAccess<'de>,
	{
		let mut map = UnitMap::default();

		while let Some((key, value)) = access.next_entry()? {
			*(map.0.index_mut(key)) = value;
		}

		Ok(map)
	}
}

#[derive(Default, PartialEq, Eq, Debug)]
pub struct SeasonMap<T>(EnumMap<Season, T>);

impl<T> Index<Season> for SeasonMap<T>
{
	type Output = T;

	fn index(&self, key : Season) -> &T
	{
		self.0.index(key)
	}
}

impl<T> IndexMut<Season> for SeasonMap<T>
{
	fn index_mut(&mut self, key : Season) -> &mut T
	{
		self.0.index_mut(key)
	}
}

impl <T> Serialize for SeasonMap<T>
	where T: Serialize
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut map = serializer.serialize_map(Some(self.0.len()))?;
		for (k, v) in self.0.iter()
		{
			map.serialize_entry(&k, &v)?;
		}
		map.end()
	}
}

impl <'de, T> Deserialize<'de> for SeasonMap<T>
	where T: Deserialize<'de> + Default
{
	fn deserialize<D>(deserializer : D) -> Result<Self, D::Error>
		where D: Deserializer<'de>
	{
		deserializer.deserialize_map(SeasonMapVisitor::new())
	}
}

struct SeasonMapVisitor<T> {
	marker: PhantomData<fn() -> SeasonMap<T>>
}

impl<T> SeasonMapVisitor<T> {
	fn new() -> Self {
		SeasonMapVisitor {
			marker: PhantomData
		}
	}
}

impl<'de, T> Visitor<'de> for SeasonMapVisitor<T>
where
	T: Deserialize<'de> + Default,
{
	type Value = SeasonMap<T>;

	fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		formatter.write_str("SeasonMap")
	}

	fn visit_map<M>(self, mut access: M) -> Result<Self::Value, M::Error>
	where
		M: MapAccess<'de>,
	{
		let mut map = SeasonMap::default();

		while let Some((key, value)) = access.next_entry()? {
			*(map.0.index_mut(key)) = value;
		}

		Ok(map)
	}
}
