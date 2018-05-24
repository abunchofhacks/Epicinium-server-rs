/* Bible */

use enum_map::EnumMap;
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
	pub tileAccessible : EnumMap<TileType, bool>,
	pub tileWalkable : EnumMap<TileType, bool>,
	pub tileBuildable : EnumMap<TileType, bool>,
	pub tileDestructible : EnumMap<TileType, bool>,
	pub tileGrassy : EnumMap<TileType, bool>,
	pub tileNatural : EnumMap<TileType, bool>,
	pub tileLaboring : EnumMap<TileType, bool>,
	pub tileEnergizing : EnumMap<TileType, bool>,
	pub tilePowered : EnumMap<TileType, bool>,
	pub tileOwnable : EnumMap<TileType, bool>,
	pub tileControllable : EnumMap<TileType, bool>,
	pub tileAutoCultivates : EnumMap<TileType, bool>,
	pub tilePlane : EnumMap<TileType, bool>,

	pub tileStacksBuilt : EnumMap<TileType, i8>,
	pub tileStacksMax : EnumMap<TileType, i8>,
	pub tilePowerBuilt : EnumMap<TileType, i8>,
	pub tilePowerMax : EnumMap<TileType, i8>,
	pub tileVision : EnumMap<TileType, i8>,
	pub tileHitpoints : EnumMap<TileType, i8>,
	pub tileIncome : EnumMap<TileType, i8>,
	pub tileLeakGas : EnumMap<TileType, i8>,
	pub tileLeakRads : EnumMap<TileType, i8>,
	pub tileEmitChaos : EnumMap<TileType, i8>,

	pub tileProduces : EnumMap<TileType, Vec<UnitBuild>>,
	pub tileExpands : EnumMap<TileType, Vec<TileBuild>>,
	pub tileUpgrades : EnumMap<TileType, Vec<TileBuild>>,
	pub tileCultivates : EnumMap<TileType, Vec<TileBuild>>,

	pub tileScoreBase : EnumMap<TileType, i16>,
	pub tileScoreStack : EnumMap<TileType, i16>,

	pub tileDestroyed : EnumMap<TileType, TileType>,

	pub tileExpandRangeMin : i8,
	pub tileExpandRangeMax : i8,
	pub tileProduceRangeMin : i8,
	pub tileProduceRangeMax : i8,

	/* UNITS */
	pub unitAir : EnumMap<UnitType, bool>,
	pub unitInfantry : EnumMap<UnitType, bool>,
	pub unitMechanical : EnumMap<UnitType, bool>,
	pub unitCanMove : EnumMap<UnitType, bool>,
	pub unitCanAttack : EnumMap<UnitType, bool>,
	pub unitCanGuard : EnumMap<UnitType, bool>,
	pub unitCanFocus : EnumMap<UnitType, bool>,
	pub unitCanShell : EnumMap<UnitType, bool>,
	pub unitCanBombard : EnumMap<UnitType, bool>,
	pub unitCanBomb : EnumMap<UnitType, bool>,
	pub unitCanCapture : EnumMap<UnitType, bool>,
	pub unitCanOccupy : EnumMap<UnitType, bool>,

	pub unitStacksMax : EnumMap<UnitType, i8>,
	pub unitSpeed : EnumMap<UnitType, i8>,
	pub unitVision : EnumMap<UnitType, i8>,
	pub unitHitpoints : EnumMap<UnitType, i8>,
	pub unitAttackShots : EnumMap<UnitType, i8>,
	pub unitAttackDamage : EnumMap<UnitType, i8>,
	pub unitTrampleShots : EnumMap<UnitType, i8>,
	pub unitTrampleDamage : EnumMap<UnitType, i8>,
	pub unitAbilityShots : EnumMap<UnitType, i8>,
	pub unitAbilityVolleys : EnumMap<UnitType, i8>,
	pub unitAbilityDamage : EnumMap<UnitType, i8>,
	pub unitAbilityGas : EnumMap<UnitType, i8>,
	pub unitAbilityRads : EnumMap<UnitType, i8>,
	pub unitAbilityRadius : EnumMap<UnitType, i8>,
	pub unitRangeMin : EnumMap<UnitType, i8>,
	pub unitRangeMax : EnumMap<UnitType, i8>,
	pub unitLeakGas : EnumMap<UnitType, i8>,
	pub unitLeakRads : EnumMap<UnitType, i8>,

	pub unitShapes : EnumMap<UnitType, Vec<TileType>>,
	pub unitSettles : EnumMap<UnitType, Vec<TileType>>,

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
	pub seasonTemperatureSwing : EnumMap<Season, i8>,
	pub seasonGlobalWarmingFactor : EnumMap<Season, i8>,

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

	pub temperatureMinHotDeath : EnumMap<Season, i8>,
	pub temperatureMinFirestorm : EnumMap<Season, i8>,
	pub temperatureMinAridification : EnumMap<Season, i8>,
	pub temperatureMaxComfortable : EnumMap<Season, i8>,
	pub temperatureMinComfortable : EnumMap<Season, i8>,
	pub temperatureMaxSnow : EnumMap<Season, i8>,
	pub temperatureMaxFrostbite : EnumMap<Season, i8>,
	pub temperatureMaxColdDeath : EnumMap<Season, i8>,

	pub humidityMinWet : EnumMap<Season, i8>,
	pub humidityMaxDegradation : EnumMap<Season, i8>,
	pub humidityMaxDesertification : EnumMap<Season, i8>,
	pub humidityMinSnow : EnumMap<Season, i8>,
	pub humidityMinFrostbite : EnumMap<Season, i8>,
	pub humidityMaxFirestorm : EnumMap<Season, i8>,
	pub humidityMaxBonedrought : EnumMap<Season, i8>,
	pub humidityMaxStonedrought : EnumMap<Season, i8>,
	pub humidityMaxDeath : EnumMap<Season, i8>,

	pub chaosMinDegradation : EnumMap<Season, i8>,
	pub chaosMinDesertification : EnumMap<Season, i8>,
	pub chaosMinAridification : EnumMap<Season, i8>,
	pub chaosMinSnow : EnumMap<Season, i8>,
	pub chaosMinFrostbite : EnumMap<Season, i8>,
	pub chaosMinFirestorm : EnumMap<Season, i8>,
	pub chaosMinBonedrought : EnumMap<Season, i8>,
	pub chaosMinStonedrought : EnumMap<Season, i8>,
	pub chaosMinDeath : EnumMap<Season, i8>,

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
		let bible = Bible::default();

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
