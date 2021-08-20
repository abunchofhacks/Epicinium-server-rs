/* LogRotate */

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
