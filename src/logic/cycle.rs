/* Cycle */


#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
#[derive(EnumMap)]
#[serde(rename_all = "lowercase")]
pub enum Season
{
	SPRING,
	SUMMER,
	AUTUMN,
	WINTER,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
#[derive(EnumMap)]
#[serde(rename_all = "lowercase")]
pub enum Daytime
{
	LATE,
	EARLY,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
#[derive(EnumMap)]
#[serde(rename_all = "lowercase")]
pub enum Phase
{
	GROWTH,
	RESTING,
	PLANNING,
	STAGING,
	ACTION,
	DECAY,
}
