/* Ruleset */

use crate::logic::epicinium;

pub fn initialize_collection() -> Result<(), InitializationError>
{
	epicinium::initialize_ruleset_collection()
}

#[derive(Debug)]
pub enum InitializationError
{
	Failed,
}

impl std::fmt::Display for InitializationError
{
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
	{
		match self
		{
			InitializationError::Failed => write!(f, "initialization failed"),
		}
	}
}

impl std::error::Error for InitializationError {}
