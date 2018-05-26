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
use common::header::is_zero;


#[derive(Default, PartialEq, Eq, Serialize, Deserialize, Debug)]
#[allow(non_snake_case)]
pub struct Bible
{
	pub version : Version,

	/* TILES */
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tileAccessible : TileMap<bool>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tileWalkable : TileMap<bool>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tileBuildable : TileMap<bool>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tileDestructible : TileMap<bool>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tileGrassy : TileMap<bool>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tileNatural : TileMap<bool>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tileLaboring : TileMap<bool>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tileEnergizing : TileMap<bool>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tilePowered : TileMap<bool>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tileOwnable : TileMap<bool>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tileControllable : TileMap<bool>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tileAutoCultivates : TileMap<bool>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tilePlane : TileMap<bool>,

							#[serde(default, skip_serializing_if = "is_zero")]
	pub tileStacksBuilt : TileMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tileStacksMax : TileMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tilePowerBuilt : TileMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tilePowerMax : TileMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tileVision : TileMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tileHitpoints : TileMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tileIncome : TileMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tileLeakGas : TileMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tileLeakRads : TileMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tileEmitChaos : TileMap<i8>,

							#[serde(default, skip_serializing_if = "is_zero")]
	pub tileProduces : TileMap<Vec<UnitBuild>>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tileExpands : TileMap<Vec<TileBuild>>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tileUpgrades : TileMap<Vec<TileBuild>>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tileCultivates : TileMap<Vec<TileBuild>>,

							#[serde(default, skip_serializing_if = "is_zero")]
	pub tileScoreBase : TileMap<i16>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tileScoreStack : TileMap<i16>,

							#[serde(default, skip_serializing_if = "is_zero")]
	pub tileDestroyed : TileMap<TileType>,

							#[serde(default, skip_serializing_if = "is_zero")]
	pub tileExpandRangeMin : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tileExpandRangeMax : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tileProduceRangeMin : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tileProduceRangeMax : i8,

	/* UNITS */
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitAir : UnitMap<bool>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitInfantry : UnitMap<bool>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitMechanical : UnitMap<bool>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitCanMove : UnitMap<bool>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitCanAttack : UnitMap<bool>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitCanGuard : UnitMap<bool>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitCanFocus : UnitMap<bool>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitCanShell : UnitMap<bool>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitCanBombard : UnitMap<bool>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitCanBomb : UnitMap<bool>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitCanCapture : UnitMap<bool>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitCanOccupy : UnitMap<bool>,

							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitStacksMax : UnitMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitSpeed : UnitMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitVision : UnitMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitHitpoints : UnitMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitAttackShots : UnitMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitAttackDamage : UnitMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitTrampleShots : UnitMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitTrampleDamage : UnitMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitAbilityShots : UnitMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitAbilityVolleys : UnitMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitAbilityDamage : UnitMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitAbilityGas : UnitMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitAbilityRads : UnitMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitAbilityRadius : UnitMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitRangeMin : UnitMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitRangeMax : UnitMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitLeakGas : UnitMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitLeakRads : UnitMap<i8>,

							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitShapes : UnitMap<Vec<TileType>>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitSettles : UnitMap<Vec<TileType>>,

							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitSizeMax : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitVisionMax : i8,

	/* COMBAT */
							#[serde(default, skip_serializing_if = "is_zero")]
	pub missCountGround : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub missCountAir : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub missCountTrenches : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub missHitpointsGround : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub missHitpointsAir : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub missHitpointsTrenches : i8,

	/* WEATHER */
							#[serde(default, skip_serializing_if = "is_zero")]
	pub seasonTemperatureSwing : SeasonMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub seasonGlobalWarmingFactor : SeasonMap<i8>,

							#[serde(default, skip_serializing_if = "is_zero")]
	pub emissionDivisor : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub forestGrowthProbabilityDivisor : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub forestRegrowthProbabilityDivisor : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub grassRegrowthProbabilityDivisor : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub cropsRegrowthProbabilityDivisor : i8,

							#[serde(default, skip_serializing_if = "is_zero")]
	pub groundPollutionAmount : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub gasPollutionAmount : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub aridificationAmountHumid : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub aridificationAmountDegraded : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub forestGrowthAmount : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub forestRegrowthAmount : i8,

							#[serde(default, skip_serializing_if = "is_zero")]
	pub aridificationRange : i8,

							#[serde(default, skip_serializing_if = "is_zero")]
	pub aridificationCount : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub firestormCount : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub deathCount : i8,

							#[serde(default, skip_serializing_if = "is_zero")]
	pub temperatureMax : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub temperatureMin : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub humidityMax : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub humidityMin : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub chaosMax : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub chaosMin : i8,

							#[serde(default, skip_serializing_if = "is_zero")]
	pub chaosThreshold : i8,

							#[serde(default, skip_serializing_if = "is_zero")]
	pub temperatureMinHotDeath : SeasonMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub temperatureMinFirestorm : SeasonMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub temperatureMinAridification : SeasonMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub temperatureMaxComfortable : SeasonMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub temperatureMinComfortable : SeasonMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub temperatureMaxSnow : SeasonMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub temperatureMaxFrostbite : SeasonMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub temperatureMaxColdDeath : SeasonMap<i8>,

							#[serde(default, skip_serializing_if = "is_zero")]
	pub humidityMinWet : SeasonMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub humidityMaxDegradation : SeasonMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub humidityMaxDesertification : SeasonMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub humidityMinSnow : SeasonMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub humidityMinFrostbite : SeasonMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub humidityMaxFirestorm : SeasonMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub humidityMaxBonedrought : SeasonMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub humidityMaxStonedrought : SeasonMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub humidityMaxDeath : SeasonMap<i8>,

							#[serde(default, skip_serializing_if = "is_zero")]
	pub chaosMinDegradation : SeasonMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub chaosMinDesertification : SeasonMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub chaosMinAridification : SeasonMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub chaosMinSnow : SeasonMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub chaosMinFrostbite : SeasonMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub chaosMinFirestorm : SeasonMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub chaosMinBonedrought : SeasonMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub chaosMinStonedrought : SeasonMap<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub chaosMinDeath : SeasonMap<i8>,

							#[serde(default, skip_serializing_if = "is_zero")]
	pub frostbiteShots : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub frostbiteDamage : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub frostbiteThresholdDamage : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub frostbiteThresholdVulnerability : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub firestormShots : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub firestormDamage : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub gasShots : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub gasDamage : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub gasThresholdDamage : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub gasThresholdVulnerability : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub radiationShots : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub radiationDamage : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub radiationThresholdDamage : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub radiationThresholdVulnerability : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub radiationThresholdDeath : i8,

							#[serde(default, skip_serializing_if = "is_zero")]
	pub tempGenDefault : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub humGenDefault : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tempGenGainRange : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub humGenGainRange : i8,

							#[serde(default, skip_serializing_if = "is_zero")]
	pub tempGenMountainGain : Vec<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub tempGenOceanGain : Vec<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub humGenLakeGain : Vec<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub humGenOceanGain : Vec<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub humGenDesertGain : Vec<i8>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub humGenMountainGain : Vec<i8>,

	/* MECHANICS */
							#[serde(default, skip_serializing_if = "is_zero")]
	pub separatePowerStages : bool,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub industryNicenessQuantitative : bool,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub reactorNicenessQuantitative : bool,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub grassOnlyRegrowsInSpring : bool,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub treesOnlyGrowInSpring : bool,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub cropsOnlyGrowInSpring : bool,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub cropsConsumedAtNight : bool,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub forestChaosProtectionPermanent : bool,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub collateralDamageKillsTiles : bool,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub gasOnlyTargetsGroundUnits : bool,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub frostbiteOnlyTargetsGroundUnits : bool,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub trenchesForceOccupy : bool,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub trenchesHideBypassedUnit : bool,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub captureStrengthCheck : bool,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub powerDrainScaled : bool,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub groundPollutionOnlyInAutumn : bool,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub counterBasedWeather : bool,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub quantitativeChaos : bool,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub stackBasedFrostbite : bool,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub emptyBasedFrostbite : bool,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub planeBasedFrostbite : bool,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub randomizedFirestorm : bool,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub randomizedAridification : bool,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub cumulativeDeath : bool,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub vulnerabilitiesStack : bool,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub markersChangeAtNight : bool,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub cityManualStackGrowth : bool,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub industryManualStackGrowth : bool,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub reactorManualStackGrowth : bool,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub publicInitiative : bool,

							#[serde(default, skip_serializing_if = "is_zero")]
	pub snowSlowAmount : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub snowSlowMaximum : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub trenchesSlowAmount : i8,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub trenchesSlowMaximum : i8,

							#[serde(default, skip_serializing_if = "is_zero")]
	pub startingIncome : i16,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub newOrderLimit : i16,
}

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
	where T: Serialize + Default + PartialEq
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut map = serializer.serialize_map(Some(self.0.len()))?;
		for (k, v) in self.0.iter()
		{
			if *v != T::default()
			{
				map.serialize_entry(&k, &v)?;
			}
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
	where T: Serialize + Default + PartialEq
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut map = serializer.serialize_map(Some(self.0.len()))?;
		for (k, v) in self.0.iter()
		{
			if *v != T::default()
			{
				map.serialize_entry(&k, &v)?;
			}
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
	where T: Serialize + Default + PartialEq
{
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		let mut map = serializer.serialize_map(Some(self.0.len()))?;
		for (k, v) in self.0.iter()
		{
			if *v != T::default()
			{
				map.serialize_entry(&k, &v)?;
			}
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
