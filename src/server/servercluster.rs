/* ServerCluster */

use server::logincluster::*;

use signal_hook;
use signal_hook::{SIGHUP, SIGTERM};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time;

pub struct ServerCluster
{
	login: LoginCluster,
	closing: bool,
	terminating: bool,
}

impl ServerCluster
{
	pub fn create() -> ServerCluster
	{
		ServerCluster {
			login: LoginCluster::create(),
			closing: false,
			terminating: false,
		}
	}

	pub fn run(&mut self) -> std::result::Result<(), std::io::Error>
	{
		let term = Arc::new(AtomicBool::new(false));
		// Install the handler. This happens after the server has been created
		// because if creation hangs we just want to kill it immediately.
		signal_hook::flag::register(SIGTERM, Arc::clone(&term))?;
		signal_hook::flag::register(SIGHUP, Arc::clone(&term))?;
		// TODO replace SIGHUP with SIGBREAK on Windows?

		while !self.terminating
		{
			if term.load(Ordering::Relaxed)
			{
				if self.closing
				{
					self.terminating = true;
				}
				else
				{
					self.login.close();
					self.closing = true;
				}
			}

			if self.closing
			{
				if self.login.closed()
				{
					break /* out of main while loop */;
				}
			}

			thread::sleep(time::Duration::from_millis(100));

			println!("Tick");
		}

		Ok(())
	}
}
