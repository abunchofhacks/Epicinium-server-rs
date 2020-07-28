/* Terminate */

use std::fs::Permissions;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;

use log::*;

pub struct Setup
{
	filename: String,
}

impl Drop for Setup
{
	fn drop(&mut self)
	{
		match std::fs::remove_file(&self.filename)
		{
			Ok(()) =>
			{}
			Err(error) =>
			{
				error!("Failed to remove '{}': {}", self.filename, error);
			}
		}
	}
}

pub fn setup() -> Result<Setup, std::io::Error>
{
	let filename = "terminate.sh".to_string();
	let mut file = std::fs::OpenOptions::new()
		.write(true)
		.create_new(true)
		.open(&filename)?;
	write!(file, "kill -TERM {}", std::process::id())?;
	file.set_permissions(Permissions::from_mode(0o744))?;

	let setup = Setup { filename };
	Ok(setup)
}
