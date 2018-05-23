/* Version */

//use serde::Serialize;
//use serde::Deserialize;


// TODO write custom serialize and deserialize
#[derive(PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Version
{
	pub major : i8,
	pub minor : i8,
	pub patch : i8,
	pub release : i8
}

impl Default for Version
{
	fn default() -> Version
	{
		Version{
			major: 0,
			minor: 1,
			patch: 0,
			release: 0
		}
	}
}

/*
impl Serialize for Version
{
	fn serialize()
	{
		_unimplemented
	}
}

impl Deserialize for Version
{
	fn deserialize()
	{
		_unimplemented
	}
}
*/
