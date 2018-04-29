/* Cycle */


#[derive(PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Season
{
	SPRING,
	SUMMER,
	AUTUMN,
	WINTER,
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Daytime
{
	LATE,
	EARLY,
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Debug)]
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
