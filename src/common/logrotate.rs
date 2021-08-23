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

use crate::common::log;

use std::io::Write;

use unindent::Unindent;

pub struct Setup
{
	pub conffilename: String,
	pub statusfilename: String,
}

impl Drop for Setup
{
	fn drop(&mut self)
	{
		match std::fs::remove_file(&self.conffilename)
		{
			Ok(()) =>
			{}
			Err(error) =>
			{
				eprintln!(
					"Failed to remove '{}': {}",
					self.conffilename, error
				);
			}
		}
	}
}

pub fn setup(logname: &str) -> Result<Setup, std::io::Error>
{
	let conffilename = format!("logs/.{}.logrotate.conf", logname);
	let mut file = std::fs::OpenOptions::new()
		.write(true)
		.create_new(true)
		.open(&conffilename)?;

	let conf = format!(
		"
		\"{trace}\" \"{info}\" \"{error}\" {{
			rotate 500
			size 1M
			extension .log
			compress
			delaycompress
			sharedscripts
			missingok
			postrotate
				kill -HUP {pid}
			endscript
		}}",
		trace = log::trace_filename(logname),
		info = log::info_filename(logname),
		error = log::error_filename(logname),
		pid = std::process::id(),
	)
	.unindent();

	file.write_all(conf.as_bytes())?;

	let statusfilename = format!("logs/.{}.logrotate.status", logname);

	let setup = Setup {
		conffilename,
		statusfilename,
	};
	Ok(setup)
}
