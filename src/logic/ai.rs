/* AI */

pub use crate::logic::epicinium::allocate_ai;
pub use crate::logic::epicinium::AllocatedAi;
pub use crate::logic::epicinium::AllocationError;

use crate::logic::epicinium;

pub fn exists(ainame: &str) -> bool
{
	epicinium::ai_exists(ainame)
}

pub fn load_pool() -> Vec<String>
{
	epicinium::ai_pool()
}
