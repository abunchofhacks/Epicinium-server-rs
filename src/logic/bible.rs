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
// TODO replace allow(non_snake_case) with serde(rename_all = "camelCase")?
// TODO use #[serde(default = "path")] for fields with non-zero defaults
// TODO use #[serde(deserialize_with = "path")] for fields with complex b.comp.
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

							#[serde(default, skip_serializing)]
	pub tileCost : TileMap<i16>,
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
	pub unitShapes : UnitMap<Vec<TileBuild>>,
							#[serde(default, skip_serializing_if = "is_zero")]
	pub unitSettles : UnitMap<Vec<TileBuild>>,

							#[serde(default, skip_serializing)]
	pub unitCost : UnitMap<i16>,

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

		/* TILES */
		bible.tileAccessible[TileType::GRASS] = true;
		bible.tileWalkable[TileType::GRASS] = true;
		bible.tileBuildable[TileType::GRASS] = true;
		bible.tileDestructible[TileType::GRASS] = true;
		bible.tileGrassy[TileType::GRASS] = true;
		bible.tileNatural[TileType::GRASS] = true;
		bible.tilePlane[TileType::GRASS] = true;
		bible.tileDestroyed[TileType::GRASS] = TileType::DIRT;
		bible.tileScoreBase[TileType::GRASS] = 1;

		bible.tileAccessible[TileType::DIRT] = true;
		bible.tileWalkable[TileType::DIRT] = true;
		bible.tileBuildable[TileType::DIRT] = true;
		bible.tileGrassy[TileType::DIRT] = false;
		bible.tileNatural[TileType::DIRT] = false;
		bible.tilePlane[TileType::DIRT] = true;

		bible.tileAccessible[TileType::DESERT] = true;
		bible.tileWalkable[TileType::DESERT] = true;
		bible.tileBuildable[TileType::DESERT] = false;
		bible.tileGrassy[TileType::DESERT] = false;
		bible.tileNatural[TileType::DESERT] = false;
		bible.tilePlane[TileType::DESERT] = true;

		bible.tileAccessible[TileType::STONE] = true;
		bible.tileWalkable[TileType::STONE] = true;
		bible.tileBuildable[TileType::STONE] = true;
		bible.tileGrassy[TileType::STONE] = false;
		bible.tileNatural[TileType::STONE] = false;
		bible.tilePlane[TileType::STONE] = true;

		bible.tileAccessible[TileType::RUBBLE] = true;
		bible.tileWalkable[TileType::RUBBLE] = true;
		bible.tileBuildable[TileType::RUBBLE] = false;
		bible.tileGrassy[TileType::RUBBLE] = false;
		bible.tileNatural[TileType::RUBBLE] = false;
		bible.tilePlane[TileType::RUBBLE] = true;
		bible.tileEmitChaos[TileType::RUBBLE] = 1;

		bible.tileAccessible[TileType::RIDGE] = true;
		bible.tileWalkable[TileType::RIDGE] = false;
		bible.tileGrassy[TileType::RIDGE] = false;
		bible.tileNatural[TileType::RIDGE] = false;
		bible.tileStacksMax[TileType::RIDGE] = 4;
		bible.tileStacksBuilt[TileType::RIDGE] = 4;
		bible.tileHitpoints[TileType::RIDGE] = 100;

		bible.tileAccessible[TileType::MOUNTAIN] = true;
		bible.tileWalkable[TileType::MOUNTAIN] = false;
		bible.tileGrassy[TileType::MOUNTAIN] = false;
		bible.tileNatural[TileType::MOUNTAIN] = false;
		bible.tileStacksMax[TileType::MOUNTAIN] = 5;
		bible.tileStacksBuilt[TileType::MOUNTAIN] = 5;
		bible.tileHitpoints[TileType::MOUNTAIN] = 100;

		bible.tileAccessible[TileType::WATER] = true;
		bible.tileWalkable[TileType::WATER] = false;
		bible.tileGrassy[TileType::WATER] = false;
		bible.tileNatural[TileType::WATER] = true;
		bible.tilePlane[TileType::WATER] = true;

		bible.tileAccessible[TileType::FOREST] = true;
		bible.tileWalkable[TileType::FOREST] = true;
		bible.tileBuildable[TileType::FOREST] = true;
		bible.tileDestructible[TileType::FOREST] = true;
		bible.tileGrassy[TileType::FOREST] = true;
		bible.tileNatural[TileType::FOREST] = true;
		bible.tileStacksBuilt[TileType::FOREST] = 1;
		bible.tileStacksMax[TileType::FOREST] = 5;
		bible.tileHitpoints[TileType::FOREST] = 1;
		bible.tileDestroyed[TileType::FOREST] = TileType::GRASS;
		bible.tileScoreBase[TileType::FOREST] = 1;

		bible.tileAccessible[TileType::CITY] = true;
		bible.tileWalkable[TileType::CITY] = true;
		bible.tileDestructible[TileType::CITY] = true;
		bible.tileLaboring[TileType::CITY] = true;
		bible.tilePowered[TileType::CITY] = true;
		bible.tileOwnable[TileType::CITY] = true;
		bible.tileControllable[TileType::CITY] = true;
		bible.tileStacksBuilt[TileType::CITY] = 1;
		bible.tileStacksMax[TileType::CITY] = 5;
		bible.tilePowerBuilt[TileType::CITY] = 1;
		bible.tilePowerMax[TileType::CITY] = 5;
		bible.tileVision[TileType::CITY] = 2;
		bible.tileHitpoints[TileType::CITY] = 2;
		bible.tileIncome[TileType::CITY] = 1;
		bible.tileEmitChaos[TileType::CITY] = 2;
		bible.tileProduces[TileType::CITY] = vec![
				UnitBuild {
					typ: UnitType::MILITIA,
					cost: -1,
				},
				UnitBuild {
					typ: UnitType::SETTLER,
					cost: -1,
				},
		];
		bible.tileExpands[TileType::CITY] = vec![
				TileBuild {
					typ: TileType::INDUSTRY,
					cost: -1,
				},
				TileBuild {
					typ: TileType::BARRACKS,
					cost: -1,
				},
		];
		bible.tileCost[TileType::CITY] = 20;
		bible.tileDestroyed[TileType::CITY] = TileType::RUBBLE;

		bible.tileAccessible[TileType::TOWN] = true;
		bible.tileWalkable[TileType::TOWN] = true;
		bible.tileDestructible[TileType::TOWN] = true;
		bible.tileLaboring[TileType::TOWN] = false;
		bible.tilePowered[TileType::TOWN] = true;
		bible.tileOwnable[TileType::TOWN] = true;
		bible.tileControllable[TileType::TOWN] = true;
		bible.tileStacksBuilt[TileType::TOWN] = 1;
		bible.tileStacksMax[TileType::TOWN] = 5;
		bible.tilePowerBuilt[TileType::TOWN] = 1;
		bible.tilePowerMax[TileType::TOWN] = 5;
		bible.tileVision[TileType::TOWN] = 2;
		bible.tileHitpoints[TileType::TOWN] = 2;
		bible.tileIncome[TileType::TOWN] = 1;
		bible.tileEmitChaos[TileType::TOWN] = 1;
		bible.tileProduces[TileType::TOWN] = vec![
				UnitBuild {
					typ: UnitType::SETTLER,
					cost: -1,
				},
		];
		bible.tileUpgrades[TileType::TOWN] = vec![
				TileBuild {
					typ: TileType::CITY,
					cost: 15,
				},
		];
		bible.tileCost[TileType::TOWN] = 5;
		bible.tileDestroyed[TileType::TOWN] = TileType::RUBBLE;

		bible.tileAccessible[TileType::SETTLEMENT] = true;
		bible.tileWalkable[TileType::SETTLEMENT] = true;
		bible.tileDestructible[TileType::SETTLEMENT] = true;
		bible.tileLaboring[TileType::SETTLEMENT] = true;
		bible.tilePowered[TileType::SETTLEMENT] = true;
		bible.tileOwnable[TileType::SETTLEMENT] = true;
		bible.tileControllable[TileType::SETTLEMENT] = true;
		bible.tileStacksBuilt[TileType::SETTLEMENT] = 1;
		bible.tileStacksMax[TileType::SETTLEMENT] = 3;
		bible.tilePowerBuilt[TileType::SETTLEMENT] = 1;
		bible.tilePowerMax[TileType::SETTLEMENT] = 3;
		bible.tileVision[TileType::SETTLEMENT] = 2;
		bible.tileHitpoints[TileType::SETTLEMENT] = 2;
		bible.tileIncome[TileType::SETTLEMENT] = 0;
		bible.tileEmitChaos[TileType::SETTLEMENT] = 1;
		bible.tileProduces[TileType::SETTLEMENT] = vec![
				UnitBuild {
					typ: UnitType::SETTLER,
					cost: -1,
				},
				UnitBuild {
					typ: UnitType::MILITIA,
					cost: -1,
				},
		];
		bible.tileUpgrades[TileType::SETTLEMENT] = vec![
				TileBuild {
					typ: TileType::CITY,
					cost: 18,
				},
		];
		bible.tileCost[TileType::SETTLEMENT] = 2;
		bible.tileDestroyed[TileType::SETTLEMENT] = TileType::RUBBLE;

		bible.tileAccessible[TileType::INDUSTRY] = true;
		bible.tileWalkable[TileType::INDUSTRY] = true;
		bible.tileDestructible[TileType::INDUSTRY] = true;
		bible.tileEnergizing[TileType::INDUSTRY] = true;
		bible.tilePowered[TileType::INDUSTRY] = true;
		bible.tileOwnable[TileType::INDUSTRY] = true;
		bible.tileControllable[TileType::INDUSTRY] = true;
		bible.tileStacksBuilt[TileType::INDUSTRY] = 1;
		bible.tileStacksMax[TileType::INDUSTRY] = 3;
		bible.tilePowerBuilt[TileType::INDUSTRY] = 1;
		bible.tilePowerMax[TileType::INDUSTRY] = 3;
		bible.tileVision[TileType::INDUSTRY] = 2;
		bible.tileHitpoints[TileType::INDUSTRY] = 3;
		bible.tileIncome[TileType::INDUSTRY] = 3;
		bible.tileLeakGas[TileType::INDUSTRY] = 1;
		bible.tileEmitChaos[TileType::INDUSTRY] = 5;
		bible.tileProduces[TileType::INDUSTRY] = vec![
				UnitBuild {
					typ: UnitType::TANK,
					cost: -1,
				},
		];
		bible.tileExpands[TileType::INDUSTRY] = vec![
				TileBuild {
					typ: TileType::AIRFIELD,
					cost: -1,
				},
		];
		bible.tileUpgrades[TileType::INDUSTRY] = vec![
				TileBuild {
					typ: TileType::NONE,
					cost: 30,
				},
		];
		bible.tileCost[TileType::INDUSTRY] = 3;
		bible.tileDestroyed[TileType::INDUSTRY] = TileType::RUBBLE;

		bible.tileAccessible[TileType::EMBASSY] = true;
		bible.tileWalkable[TileType::EMBASSY] = true;
		bible.tileDestructible[TileType::EMBASSY] = true;
		bible.tilePowered[TileType::EMBASSY] = true;
		bible.tileOwnable[TileType::EMBASSY] = true;
		bible.tileControllable[TileType::EMBASSY] = true;
		bible.tileStacksBuilt[TileType::EMBASSY] = 1;
		bible.tileStacksMax[TileType::EMBASSY] = 1;
		bible.tilePowerBuilt[TileType::EMBASSY] = 1;
		bible.tilePowerMax[TileType::EMBASSY] = 1;
		bible.tileVision[TileType::EMBASSY] = 2;
		bible.tileHitpoints[TileType::EMBASSY] = 3;
		bible.tileEmitChaos[TileType::EMBASSY] = 1;
		bible.tileProduces[TileType::EMBASSY] = vec![
				UnitBuild {
					typ: UnitType::SETTLER,
					cost: -1,
				},
		];
		bible.tileCost[TileType::EMBASSY] = 20;
		bible.tileDestroyed[TileType::EMBASSY] = TileType::RUBBLE;

		bible.tileAccessible[TileType::BARRACKS] = true;
		bible.tileWalkable[TileType::BARRACKS] = true;
		bible.tileDestructible[TileType::BARRACKS] = true;
		bible.tilePowered[TileType::BARRACKS] = true;
		bible.tileOwnable[TileType::BARRACKS] = true;
		bible.tileControllable[TileType::BARRACKS] = true;
		bible.tileStacksBuilt[TileType::BARRACKS] = 1;
		bible.tileStacksMax[TileType::BARRACKS] = 3;
		bible.tilePowerBuilt[TileType::BARRACKS] = 1;
		bible.tilePowerMax[TileType::BARRACKS] = 3;
		bible.tileVision[TileType::BARRACKS] = 2;
		bible.tileHitpoints[TileType::BARRACKS] = 3;
		bible.tileEmitChaos[TileType::BARRACKS] = 1;
		bible.tileProduces[TileType::BARRACKS] = vec![
				UnitBuild {
					typ: UnitType::RIFLEMAN,
					cost: -1,
				},
				UnitBuild {
					typ: UnitType::GUNNER,
					cost: -1,
				},
				UnitBuild {
					typ: UnitType::SAPPER,
					cost: -1,
				},
		];
		bible.tileUpgrades[TileType::BARRACKS] = vec![
				TileBuild {
					typ: TileType::NONE,
					cost: 30,
				},
		];
		bible.tileCost[TileType::BARRACKS] = 3;
		bible.tileDestroyed[TileType::BARRACKS] = TileType::RUBBLE;

		bible.tileAccessible[TileType::AIRFIELD] = true;
		bible.tileWalkable[TileType::AIRFIELD] = true;
		bible.tileDestructible[TileType::AIRFIELD] = true;
		bible.tilePowered[TileType::AIRFIELD] = true;
		bible.tileOwnable[TileType::AIRFIELD] = true;
		bible.tileControllable[TileType::AIRFIELD] = true;
		bible.tileStacksBuilt[TileType::AIRFIELD] = 1;
		bible.tileStacksMax[TileType::AIRFIELD] = 1;
		bible.tilePowerBuilt[TileType::AIRFIELD] = 1;
		bible.tilePowerMax[TileType::AIRFIELD] = 1;
		bible.tileVision[TileType::AIRFIELD] = 2;
		bible.tileHitpoints[TileType::AIRFIELD] = 3;
		bible.tileLeakGas[TileType::AIRFIELD] = 1;
		bible.tileEmitChaos[TileType::AIRFIELD] = 1;
		bible.tileProduces[TileType::AIRFIELD] = vec![
				UnitBuild {
					typ: UnitType::ZEPPELIN,
					cost: -1,
				},
		];
		bible.tileCost[TileType::AIRFIELD] = 5;
		bible.tileDestroyed[TileType::AIRFIELD] = TileType::RUBBLE;

		bible.tileAccessible[TileType::REACTOR] = true;
		bible.tileWalkable[TileType::REACTOR] = true;
		bible.tileDestructible[TileType::REACTOR] = true;
		bible.tilePowered[TileType::REACTOR] = true;
		bible.tileOwnable[TileType::REACTOR] = true;
		bible.tileControllable[TileType::REACTOR] = true;
		bible.tileStacksBuilt[TileType::REACTOR] = 1;
		bible.tileStacksMax[TileType::REACTOR] = 1;
		bible.tilePowerBuilt[TileType::REACTOR] = 1;
		bible.tilePowerMax[TileType::REACTOR] = 1;
		bible.tileVision[TileType::REACTOR] = 2;
		bible.tileHitpoints[TileType::REACTOR] = 3;
		bible.tileIncome[TileType::REACTOR] = 4;
		bible.tileLeakRads[TileType::REACTOR] = 2;
		bible.tileEmitChaos[TileType::REACTOR] = 1;
		bible.tileProduces[TileType::REACTOR] = vec![
				UnitBuild {
					typ: UnitType::NUKE,
					cost: -1,
				},
		];
		bible.tileCost[TileType::REACTOR] = 30;
		bible.tileDestroyed[TileType::REACTOR] = TileType::RUBBLE;

		bible.tileAccessible[TileType::FARM] = true;
		bible.tileWalkable[TileType::FARM] = true;
		bible.tileDestructible[TileType::FARM] = true;
		bible.tilePowered[TileType::FARM] = true;
		bible.tileOwnable[TileType::FARM] = true;
		bible.tileControllable[TileType::FARM] = true;
		bible.tileAutoCultivates[TileType::FARM] = true;
		bible.tileStacksBuilt[TileType::FARM] = 1;
		bible.tileStacksMax[TileType::FARM] = 2;
		bible.tilePowerBuilt[TileType::FARM] = 1;
		bible.tilePowerMax[TileType::FARM] = 2;
		bible.tileVision[TileType::FARM] = 2;
		bible.tileHitpoints[TileType::FARM] = 2;
		bible.tileIncome[TileType::FARM] = 0;
		bible.tileEmitChaos[TileType::FARM] = 1;
		bible.tileProduces[TileType::FARM] = vec![
				UnitBuild {
					typ: UnitType::SETTLER,
					cost: -1,
				},
				UnitBuild {
					typ: UnitType::MILITIA,
					cost: -1,
				},
		];
		bible.tileCultivates[TileType::FARM] = vec![
				TileBuild {
					typ: TileType::SOIL,
					cost: -1
				},
		];
		bible.tileCost[TileType::FARM] = 2;
		bible.tileDestroyed[TileType::FARM] = TileType::RUBBLE;

		bible.tileAccessible[TileType::SOIL] = true;
		bible.tileWalkable[TileType::SOIL] = true;
		bible.tileBuildable[TileType::SOIL] = true;
		bible.tileDestructible[TileType::SOIL] = true;
		bible.tilePowered[TileType::SOIL] = false;
		bible.tileOwnable[TileType::SOIL] = true;
		bible.tileControllable[TileType::SOIL] = false;
		bible.tileGrassy[TileType::SOIL] = false;
		bible.tileNatural[TileType::SOIL] = false;
		bible.tilePlane[TileType::SOIL] = true;
		bible.tileVision[TileType::SOIL] = 0;
		bible.tileIncome[TileType::SOIL] = 0;
		bible.tileCost[TileType::SOIL] = 0;
		bible.tileDestroyed[TileType::SOIL] = TileType::DIRT;

		bible.tileAccessible[TileType::CROPS] = true;
		bible.tileWalkable[TileType::CROPS] = true;
		bible.tileBuildable[TileType::CROPS] = true;
		bible.tileDestructible[TileType::CROPS] = true;
		bible.tilePowered[TileType::CROPS] = false;
		bible.tileOwnable[TileType::CROPS] = true;
		bible.tileControllable[TileType::CROPS] = false;
		bible.tileGrassy[TileType::CROPS] = false;
		bible.tileNatural[TileType::CROPS] = true;
		bible.tilePlane[TileType::CROPS] = true;
		bible.tileVision[TileType::CROPS] = 0;
		bible.tileIncome[TileType::CROPS] = 1;
		bible.tileDestroyed[TileType::CROPS] = TileType::DIRT;

		bible.tileAccessible[TileType::TRENCHES] = true;
		bible.tileWalkable[TileType::TRENCHES] = true;
		bible.tileBuildable[TileType::TRENCHES] = false;
		bible.tileGrassy[TileType::TRENCHES] = false;
		bible.tileNatural[TileType::TRENCHES] = false;
		bible.tilePowered[TileType::TRENCHES] = false;
		bible.tileOwnable[TileType::TRENCHES] = false;
		bible.tileControllable[TileType::TRENCHES] = false;
		bible.tilePlane[TileType::TRENCHES] = false;
		bible.tileCost[TileType::TRENCHES] = 0;

		bible.tileExpandRangeMin = 1;
		bible.tileExpandRangeMax = 1;
		bible.tileProduceRangeMin = 0;
		bible.tileProduceRangeMax = 1;

		/* UNITS */
		bible.unitInfantry[UnitType::RIFLEMAN] = true;
		bible.unitCanMove[UnitType::RIFLEMAN] = true;
		bible.unitCanAttack[UnitType::RIFLEMAN] = true;
		bible.unitCanFocus[UnitType::RIFLEMAN] = true;
		bible.unitCanCapture[UnitType::RIFLEMAN] = true;
		bible.unitCanOccupy[UnitType::RIFLEMAN] = true;
		bible.unitStacksMax[UnitType::RIFLEMAN] = 3;
		bible.unitSpeed[UnitType::RIFLEMAN] = 3;
		bible.unitVision[UnitType::RIFLEMAN] = 4;
		bible.unitHitpoints[UnitType::RIFLEMAN] = 2;
		bible.unitAttackShots[UnitType::RIFLEMAN] = 1;
		bible.unitAttackDamage[UnitType::RIFLEMAN] = 1;
		bible.unitCost[UnitType::RIFLEMAN] = 5;

		bible.unitInfantry[UnitType::GUNNER] = true;
		bible.unitCanMove[UnitType::GUNNER] = true;
		bible.unitCanAttack[UnitType::GUNNER] = true;
		bible.unitCanFocus[UnitType::GUNNER] = true;
		bible.unitCanOccupy[UnitType::GUNNER] = true;
		bible.unitStacksMax[UnitType::GUNNER] = 3;
		bible.unitSpeed[UnitType::GUNNER] = 2;
		bible.unitVision[UnitType::GUNNER] = 4;
		bible.unitHitpoints[UnitType::GUNNER] = 2;
		bible.unitAttackShots[UnitType::GUNNER] = 3;
		bible.unitAttackDamage[UnitType::GUNNER] = 1;
		bible.unitShapes[UnitType::GUNNER] = vec![
				TileBuild {
					typ: TileType::TRENCHES,
					cost: -1
				},
		];
		bible.unitCost[UnitType::GUNNER] = 10;

		bible.unitInfantry[UnitType::SAPPER] = true;
		bible.unitCanMove[UnitType::SAPPER] = true;
		bible.unitCanAttack[UnitType::SAPPER] = true;
		bible.unitCanFocus[UnitType::SAPPER] = true;
		bible.unitCanBombard[UnitType::SAPPER] = true;
		bible.unitCanOccupy[UnitType::SAPPER] = true;
		bible.unitStacksMax[UnitType::SAPPER] = 3;
		bible.unitSpeed[UnitType::SAPPER] = 3;
		bible.unitVision[UnitType::SAPPER] = 4;
		bible.unitHitpoints[UnitType::SAPPER] = 1;
		bible.unitAttackShots[UnitType::SAPPER] = 1;
		bible.unitAttackDamage[UnitType::SAPPER] = 1;
		bible.unitAbilityVolleys[UnitType::SAPPER] = 1;
		bible.unitAbilityShots[UnitType::SAPPER] = 1;
		bible.unitAbilityDamage[UnitType::SAPPER] = 3;
		bible.unitRangeMin[UnitType::SAPPER] = 2;
		bible.unitRangeMax[UnitType::SAPPER] = 10;
		bible.unitCost[UnitType::SAPPER] = 10;

		bible.unitMechanical[UnitType::TANK] = true;
		bible.unitCanMove[UnitType::TANK] = true;
		bible.unitCanAttack[UnitType::TANK] = false;
		bible.unitCanFocus[UnitType::TANK] = false;
		bible.unitCanShell[UnitType::TANK] = true;
		bible.unitCanOccupy[UnitType::TANK] = true;
		bible.unitStacksMax[UnitType::TANK] = 3;
		bible.unitSpeed[UnitType::TANK] = 3;
		bible.unitVision[UnitType::TANK] = 2;
		bible.unitHitpoints[UnitType::TANK] = 3;
		bible.unitAttackShots[UnitType::TANK] = 0;
		bible.unitAttackDamage[UnitType::TANK] = 0;
		bible.unitTrampleShots[UnitType::TANK] = 1;
		bible.unitTrampleDamage[UnitType::TANK] = 1;
		bible.unitAbilityVolleys[UnitType::TANK] = 2;
		bible.unitAbilityShots[UnitType::TANK] = 1;
		bible.unitAbilityDamage[UnitType::TANK] = 3;
		bible.unitRangeMin[UnitType::TANK] = 1;
		bible.unitRangeMax[UnitType::TANK] = 1;
		bible.unitCost[UnitType::TANK] = 15;

		bible.unitCanMove[UnitType::SETTLER] = true;
		bible.unitCanOccupy[UnitType::SETTLER] = true;
		bible.unitStacksMax[UnitType::SETTLER] = 1;
		bible.unitSpeed[UnitType::SETTLER] = 3;
		bible.unitVision[UnitType::SETTLER] = 2;
		bible.unitHitpoints[UnitType::SETTLER] = 1;
		bible.unitSettles[UnitType::SETTLER] = vec![
				TileBuild {
					typ: TileType::CITY,
					cost: -1,
				},
				TileBuild {
					typ: TileType::TOWN,
					cost: -1,
				},
				TileBuild {
					typ: TileType::FARM,
					cost: -1,
				},
		];
		bible.unitCost[UnitType::SETTLER] = 1;

		bible.unitInfantry[UnitType::MILITIA] = true;
		bible.unitCanMove[UnitType::MILITIA] = true;
		bible.unitCanAttack[UnitType::MILITIA] = true;
		bible.unitCanFocus[UnitType::MILITIA] = true;
		bible.unitCanCapture[UnitType::MILITIA] = false;
		bible.unitCanOccupy[UnitType::MILITIA] = true;
		bible.unitStacksMax[UnitType::MILITIA] = 5;
		bible.unitSpeed[UnitType::MILITIA] = 3;
		bible.unitVision[UnitType::MILITIA] = 4;
		bible.unitHitpoints[UnitType::MILITIA] = 1;
		bible.unitAttackShots[UnitType::MILITIA] = 1;
		bible.unitAttackDamage[UnitType::MILITIA] = 1;
		bible.unitCost[UnitType::MILITIA] = 5;

		bible.unitCanMove[UnitType::DIPLOMAT] = true;
		bible.unitCanCapture[UnitType::DIPLOMAT] = true;
		bible.unitCanOccupy[UnitType::DIPLOMAT] = true;
		bible.unitStacksMax[UnitType::DIPLOMAT] = 1;
		bible.unitSpeed[UnitType::DIPLOMAT] = 3;
		bible.unitVision[UnitType::DIPLOMAT] = 2;
		bible.unitHitpoints[UnitType::DIPLOMAT] = 1;
		bible.unitCost[UnitType::DIPLOMAT] = 5;

		bible.unitAir[UnitType::ZEPPELIN] = true;
		bible.unitMechanical[UnitType::ZEPPELIN] = true;
		bible.unitCanMove[UnitType::ZEPPELIN] = true;
		bible.unitCanBomb[UnitType::ZEPPELIN] = true;
		bible.unitCanOccupy[UnitType::ZEPPELIN] = false;
		bible.unitStacksMax[UnitType::ZEPPELIN] = 1;
		bible.unitSpeed[UnitType::ZEPPELIN] = 1;
		bible.unitVision[UnitType::ZEPPELIN] = 10;
		bible.unitHitpoints[UnitType::ZEPPELIN] = 3;
		bible.unitAbilityVolleys[UnitType::ZEPPELIN] = 1;
		bible.unitAbilityShots[UnitType::ZEPPELIN] = 1;
		bible.unitAbilityDamage[UnitType::ZEPPELIN] = 0;
		bible.unitRangeMin[UnitType::ZEPPELIN] = 0;
		bible.unitRangeMax[UnitType::ZEPPELIN] = 0;
		bible.unitAbilityGas[UnitType::ZEPPELIN] = 2;
		bible.unitLeakGas[UnitType::ZEPPELIN] = 1;
		bible.unitCost[UnitType::ZEPPELIN] = 20;

		bible.unitMechanical[UnitType::GLIDER] = true;
		bible.unitCanMove[UnitType::GLIDER] = true;
		bible.unitStacksMax[UnitType::GLIDER] = 1;
		bible.unitSpeed[UnitType::GLIDER] = 3;
		bible.unitVision[UnitType::GLIDER] = 2;
		bible.unitHitpoints[UnitType::GLIDER] = 3;
		bible.unitCost[UnitType::GLIDER] = 10;

		bible.unitMechanical[UnitType::NUKE] = true;
		bible.unitCanMove[UnitType::NUKE] = true;
		bible.unitStacksMax[UnitType::NUKE] = 1;
		bible.unitSpeed[UnitType::NUKE] = 2;
		bible.unitVision[UnitType::NUKE] = 2;
		bible.unitHitpoints[UnitType::NUKE] = 3;
		bible.unitLeakRads[UnitType::NUKE] = 1;
		bible.unitCost[UnitType::NUKE] = 100;

		bible.unitSizeMax = bible.unitStacksMax[UnitType::RIFLEMAN];
		bible.unitVisionMax = bible.unitVision[UnitType::ZEPPELIN];

		/* COMBAT */
		bible.missCountGround = 1;
		bible.missCountTrenches = 3;
		bible.missHitpointsGround = 1;
		bible.missHitpointsTrenches = 1;

		/* WEATHER */
		bible.emissionDivisor = 0;
		bible.forestGrowthProbabilityDivisor = 2;
		bible.forestRegrowthProbabilityDivisor = 2;
		bible.grassRegrowthProbabilityDivisor = -1;
		bible.cropsRegrowthProbabilityDivisor = 1;

		bible.groundPollutionAmount = 1;
		bible.gasPollutionAmount = 1;
		bible.aridificationAmountHumid = 1;
		bible.aridificationAmountDegraded = 1;
		bible.forestGrowthAmount = 1;
		bible.forestRegrowthAmount = 1;

		bible.aridificationRange = 2;

		bible.aridificationCount = 10;
		bible.firestormCount = 30;
		bible.deathCount = 1;

		bible.humidityMax = 4;
		bible.humidityMin = 0;
		bible.chaosMax = 1;
		bible.chaosMin = 0;

		bible.chaosThreshold = 25;

		bible.humidityMinWet[Season::SPRING] = 1;
		bible.humidityMaxDegradation[Season::SPRING] = 0;
		bible.humidityMaxDesertification[Season::SPRING] = 0;
		bible.humidityMinSnow[Season::SPRING] = 4;
		bible.humidityMaxBonedrought[Season::SPRING] = 0;
		bible.humidityMaxStonedrought[Season::SPRING] = 0;
		bible.chaosMinDegradation[Season::SPRING] = 0;
		bible.chaosMinDesertification[Season::SPRING] =
				2 * bible.chaosThreshold;
		bible.chaosMinAridification[Season::SPRING] = -1;
		bible.chaosMinSnow[Season::SPRING] = 0;
		bible.chaosMinFrostbite[Season::SPRING] = -1;
		bible.chaosMinFirestorm[Season::SPRING] = -1;
		bible.chaosMinBonedrought[Season::SPRING] = 3 * bible.chaosThreshold;
		bible.chaosMinStonedrought[Season::SPRING] = 4 * bible.chaosThreshold;
		bible.chaosMinDeath[Season::SPRING] = 5 * bible.chaosThreshold;

		bible.humidityMinWet[Season::SUMMER] = 1;
		bible.humidityMaxDegradation[Season::SUMMER] = 0;
		bible.humidityMaxDesertification[Season::SUMMER] = 0;
		bible.humidityMinSnow[Season::SUMMER] = 4;
		bible.humidityMaxFirestorm[Season::SUMMER] = 4;
		bible.humidityMaxBonedrought[Season::SUMMER] = 0;
		bible.humidityMaxStonedrought[Season::SUMMER] = 0;
		bible.chaosMinDegradation[Season::SUMMER] = 0;
		bible.chaosMinDesertification[Season::SUMMER] =
				2 * bible.chaosThreshold;
		bible.chaosMinAridification[Season::SUMMER] = -1;
		bible.chaosMinSnow[Season::SUMMER] = 0;
		bible.chaosMinFrostbite[Season::SUMMER] = -1;
		bible.chaosMinFirestorm[Season::SUMMER] = 2 * bible.chaosThreshold;
		bible.chaosMinBonedrought[Season::SUMMER] = 3 * bible.chaosThreshold;
		bible.chaosMinStonedrought[Season::SUMMER] = 4 * bible.chaosThreshold;
		bible.chaosMinDeath[Season::SUMMER] = 5 * bible.chaosThreshold;

		bible.humidityMinWet[Season::AUTUMN] = 1;
		bible.humidityMaxDegradation[Season::AUTUMN] = 0;
		bible.humidityMaxDesertification[Season::AUTUMN] = 0;
		bible.humidityMinSnow[Season::AUTUMN] = 4;
		bible.humidityMaxBonedrought[Season::AUTUMN] = 0;
		bible.humidityMaxStonedrought[Season::AUTUMN] = 0;
		bible.chaosMinDegradation[Season::AUTUMN] = 0;
		bible.chaosMinDesertification[Season::AUTUMN] =
				2 * bible.chaosThreshold;
		bible.chaosMinAridification[Season::AUTUMN] = 1 * bible.chaosThreshold;
		bible.chaosMinSnow[Season::AUTUMN] = 0;
		bible.chaosMinFrostbite[Season::AUTUMN] = -1;
		bible.chaosMinFirestorm[Season::AUTUMN] = -1;
		bible.chaosMinBonedrought[Season::AUTUMN] = 3 * bible.chaosThreshold;
		bible.chaosMinStonedrought[Season::AUTUMN] = 4 * bible.chaosThreshold;
		bible.chaosMinDeath[Season::AUTUMN] = 5 * bible.chaosThreshold;

		bible.humidityMinWet[Season::WINTER] = 1;
		bible.humidityMaxDegradation[Season::WINTER] = 0;
		bible.humidityMaxDesertification[Season::WINTER] = 0;
		bible.humidityMinSnow[Season::WINTER] = 1;
		bible.humidityMinFrostbite[Season::WINTER] = 0;
		bible.humidityMaxBonedrought[Season::WINTER] = 0;
		bible.humidityMaxStonedrought[Season::WINTER] = 0;
		bible.chaosMinDegradation[Season::WINTER] = 0;
		bible.chaosMinDesertification[Season::WINTER] =
				2 * bible.chaosThreshold;
		bible.chaosMinAridification[Season::WINTER] = -1;
		bible.chaosMinSnow[Season::WINTER] = 0;
		bible.chaosMinFrostbite[Season::WINTER] = 1 * bible.chaosThreshold;
		bible.chaosMinFirestorm[Season::WINTER] = -1;
		bible.chaosMinBonedrought[Season::WINTER] = 3 * bible.chaosThreshold;
		bible.chaosMinStonedrought[Season::WINTER] = 4 * bible.chaosThreshold;
		bible.chaosMinDeath[Season::WINTER] = 5 * bible.chaosThreshold;

		bible.frostbiteShots = 2;
		bible.frostbiteDamage = 1;
		bible.frostbiteThresholdDamage = 1;
		bible.frostbiteThresholdVulnerability = 100;

		bible.firestormShots = 3;
		bible.firestormDamage = 2;

		bible.gasShots = 3;
		bible.gasDamage = 1;
		bible.gasThresholdDamage = 1;
		bible.gasThresholdVulnerability = 1;

		bible.radiationShots = 3;
		bible.radiationDamage = 1;
		bible.radiationThresholdDamage = 2;
		bible.radiationThresholdVulnerability = 1;
		bible.radiationThresholdDeath = 3;

		bible.humGenDefault = 2;
		bible.humGenGainRange = 5;

		// Distance from src =    0    1    2   -    4    5.
		bible.humGenLakeGain      = vec![  1,   1,   1,  0,   1,   1];
		bible.humGenOceanGain     = vec![  1,   1,   1,  0,   0,   0];
		bible.humGenDesertGain    = vec![ -2,  -1,  -1,  0,   0,   0];
		bible.humGenMountainGain  = vec![  3,   2,   2,  0,   1,   1];

		/* MECHANICS */
		bible.separatePowerStages = true;
		bible.industryNicenessQuantitative = false;
		bible.reactorNicenessQuantitative = false;
		bible.grassOnlyRegrowsInSpring = true;
		bible.treesOnlyGrowInSpring = true;
		bible.cropsOnlyGrowInSpring = false;
		bible.cropsConsumedAtNight = true;
		bible.forestChaosProtectionPermanent = true;
		bible.collateralDamageKillsTiles = true;
		bible.gasOnlyTargetsGroundUnits = true;
		bible.frostbiteOnlyTargetsGroundUnits = true;
		bible.trenchesForceOccupy = true;
		bible.trenchesHideBypassedUnit = true;
		bible.captureStrengthCheck = false;
		bible.powerDrainScaled = true;
		bible.groundPollutionOnlyInAutumn = true;
		bible.counterBasedWeather = true;
		bible.quantitativeChaos = true;
		bible.stackBasedFrostbite = false;
		bible.emptyBasedFrostbite = false;
		bible.planeBasedFrostbite = true;
		bible.randomizedFirestorm = true;
		bible.randomizedAridification = true;
		bible.cumulativeDeath = true;
		bible.vulnerabilitiesStack = true;
		bible.markersChangeAtNight = true;
		bible.cityManualStackGrowth = false;
		bible.industryManualStackGrowth = true;
		bible.reactorManualStackGrowth = true;
		bible.publicInitiative = true;

		bible.snowSlowAmount = 1;
		bible.snowSlowMaximum = 1;
		bible.trenchesSlowAmount = 0;
		bible.trenchesSlowMaximum = 0;

		/* COMMANDERS */
		bible.startingIncome = 20;
		bible.newOrderLimit = 5;

		/* POSTFIX */
		bible.fix_costs();

		/* DONE */
		bible
	}

	fn fix_costs(&mut self)
	{
		for (_, ref mut builds) in self.tileProduces.0.iter_mut()
		{
			for &mut UnitBuild{typ, ref mut cost} in builds.iter_mut()
			{
				if *cost < 0
				{
					*cost = self.unitCost[typ];
				}
			}
		}
		for (_, ref mut builds) in self.tileExpands.0.iter_mut()
		{
			for &mut TileBuild{typ, ref mut cost} in builds.iter_mut()
			{
				if *cost < 0
				{
					*cost = self.tileCost[typ];
				}
			}
		}
		for (_, ref mut builds) in self.tileUpgrades.0.iter_mut()
		{
			for &mut TileBuild{typ, ref mut cost} in builds.iter_mut()
			{
				if *cost < 0
				{
					*cost = self.tileCost[typ];
				}
			}
		}
		for (_, ref mut builds) in self.tileCultivates.0.iter_mut()
		{
			for &mut TileBuild{typ, ref mut cost} in builds.iter_mut()
			{
				if *cost < 0
				{
					*cost = self.tileCost[typ];
				}
			}
		}
		for (_, ref mut builds) in self.unitShapes.0.iter_mut()
		{
			for &mut TileBuild{typ, ref mut cost} in builds.iter_mut()
			{
				if *cost < 0
				{
					*cost = self.tileCost[typ];
				}
			}
		}
		for (_, ref mut builds) in self.unitSettles.0.iter_mut()
		{
			for &mut TileBuild{typ, ref mut cost} in builds.iter_mut()
			{
				if *cost < 0
				{
					*cost = self.tileCost[typ];
				}
			}
		}
	}
}

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
