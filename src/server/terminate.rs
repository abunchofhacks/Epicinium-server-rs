/*
 * Part of epicinium_server
 * developed by A Bunch of Hacks.
 *
 * Copyright (c) 2018-2021 A Bunch of Hacks
 *
 * This library is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This library is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 * [authors:]
 * Sander in 't Veld (sander@abunchofhacks.coop)
 */

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
