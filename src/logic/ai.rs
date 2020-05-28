/* AI */

use crate::logic::epicinium;

pub fn exists(ainame: &str) -> bool
{
	epicinium::ai_exists(ainame)
}

pub fn load_pool() -> Vec<String>
{
	epicinium::ai_pool()
}

#[derive(Debug)]
pub struct Commander;
