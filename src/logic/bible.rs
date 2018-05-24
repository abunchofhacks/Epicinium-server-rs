/* Bible */

use std::collections::HashMap;
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
	pub tileAccessible : HashMap<TileType, bool>,
	pub tileWalkable : HashMap<TileType, bool>,
	pub tileBuildable : HashMap<TileType, bool>,
	pub tileDestructible : HashMap<TileType, bool>,
	pub tileGrassy : HashMap<TileType, bool>,
	pub tileNatural : HashMap<TileType, bool>,
	pub tileLaboring : HashMap<TileType, bool>,
	pub tileEnergizing : HashMap<TileType, bool>,
	pub tilePowered : HashMap<TileType, bool>,
	pub tileOwnable : HashMap<TileType, bool>,
	pub tileControllable : HashMap<TileType, bool>,
	pub tileAutoCultivates : HashMap<TileType, bool>,
	pub tilePlane : HashMap<TileType, bool>,

	pub tileStacksBuilt : HashMap<TileType, i8>,
	pub tileStacksMax : HashMap<TileType, i8>,
	pub tilePowerBuilt : HashMap<TileType, i8>,
	pub tilePowerMax : HashMap<TileType, i8>,
	pub tileVision : HashMap<TileType, i8>,
	pub tileHitpoints : HashMap<TileType, i8>,
	pub tileIncome : HashMap<TileType, i8>,
	pub tileLeakGas : HashMap<TileType, i8>,
	pub tileLeakRads : HashMap<TileType, i8>,
	pub tileEmitChaos : HashMap<TileType, i8>,

	pub tileProduces : HashMap<TileType, Vec<UnitBuild>>,
	pub tileExpands : HashMap<TileType, Vec<TileBuild>>,
	pub tileUpgrades : HashMap<TileType, Vec<TileBuild>>,
	pub tileCultivates : HashMap<TileType, Vec<TileBuild>>,

	pub tileScoreBase : HashMap<TileType, i16>,
	pub tileScoreStack : HashMap<TileType, i16>,

	pub tileDestroyed : HashMap<TileType, TileType>,

	pub tileExpandRangeMin : i8,
	pub tileExpandRangeMax : i8,
	pub tileProduceRangeMin : i8,
	pub tileProduceRangeMax : i8,

	/* UNITS */
	pub unitAir : HashMap<UnitType, bool>,
	pub unitInfantry : HashMap<UnitType, bool>,
	pub unitMechanical : HashMap<UnitType, bool>,
	pub unitCanMove : HashMap<UnitType, bool>,
	pub unitCanAttack : HashMap<UnitType, bool>,
	pub unitCanGuard : HashMap<UnitType, bool>,
	pub unitCanFocus : HashMap<UnitType, bool>,
	pub unitCanShell : HashMap<UnitType, bool>,
	pub unitCanBombard : HashMap<UnitType, bool>,
	pub unitCanBomb : HashMap<UnitType, bool>,
	pub unitCanCapture : HashMap<UnitType, bool>,
	pub unitCanOccupy : HashMap<UnitType, bool>,

	pub unitStacksMax : HashMap<UnitType, i8>,
	pub unitSpeed : HashMap<UnitType, i8>,
	pub unitVision : HashMap<UnitType, i8>,
	pub unitHitpoints : HashMap<UnitType, i8>,
	pub unitAttackShots : HashMap<UnitType, i8>,
	pub unitAttackDamage : HashMap<UnitType, i8>,
	pub unitTrampleShots : HashMap<UnitType, i8>,
	pub unitTrampleDamage : HashMap<UnitType, i8>,
	pub unitAbilityShots : HashMap<UnitType, i8>,
	pub unitAbilityVolleys : HashMap<UnitType, i8>,
	pub unitAbilityDamage : HashMap<UnitType, i8>,
	pub unitAbilityGas : HashMap<UnitType, i8>,
	pub unitAbilityRads : HashMap<UnitType, i8>,
	pub unitAbilityRadius : HashMap<UnitType, i8>,
	pub unitRangeMin : HashMap<UnitType, i8>,
	pub unitRangeMax : HashMap<UnitType, i8>,
	pub unitLeakGas : HashMap<UnitType, i8>,
	pub unitLeakRads : HashMap<UnitType, i8>,

	pub unitShapes : HashMap<UnitType, Vec<TileType>>,
	pub unitSettles : HashMap<UnitType, Vec<TileType>>,

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
	pub seasonTemperatureSwing : HashMap<Season, i8>,
	pub seasonGlobalWarmingFactor : HashMap<Season, i8>,

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

	pub temperatureMinHotDeath : HashMap<Season, i8>,
	pub temperatureMinFirestorm : HashMap<Season, i8>,
	pub temperatureMinAridification : HashMap<Season, i8>,
	pub temperatureMaxComfortable : HashMap<Season, i8>,
	pub temperatureMinComfortable : HashMap<Season, i8>,
	pub temperatureMaxSnow : HashMap<Season, i8>,
	pub temperatureMaxFrostbite : HashMap<Season, i8>,
	pub temperatureMaxColdDeath : HashMap<Season, i8>,

	pub humidityMinWet : HashMap<Season, i8>,
	pub humidityMaxDegradation : HashMap<Season, i8>,
	pub humidityMaxDesertification : HashMap<Season, i8>,
	pub humidityMinSnow : HashMap<Season, i8>,
	pub humidityMinFrostbite : HashMap<Season, i8>,
	pub humidityMaxFirestorm : HashMap<Season, i8>,
	pub humidityMaxBonedrought : HashMap<Season, i8>,
	pub humidityMaxStonedrought : HashMap<Season, i8>,
	pub humidityMaxDeath : HashMap<Season, i8>,

	pub chaosMinDegradation : HashMap<Season, i8>,
	pub chaosMinDesertification : HashMap<Season, i8>,
	pub chaosMinAridification : HashMap<Season, i8>,
	pub chaosMinSnow : HashMap<Season, i8>,
	pub chaosMinFrostbite : HashMap<Season, i8>,
	pub chaosMinFirestorm : HashMap<Season, i8>,
	pub chaosMinBonedrought : HashMap<Season, i8>,
	pub chaosMinStonedrought : HashMap<Season, i8>,
	pub chaosMinDeath : HashMap<Season, i8>,

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
