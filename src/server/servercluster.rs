/* ServerCluster */

use server::logincluster::*;

use signal_hook;
use signal_hook::{SIGHUP, SIGTERM};
use std::io;
use std::sync;
use std::sync::atomic;
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
	pub fn create() -> io::Result<ServerCluster>
	{
		Ok(ServerCluster {
			login: LoginCluster::create()?,
			closing: false,
			terminating: false,
		})
	}

	pub fn run(&mut self) -> io::Result<()>
	{
		let shutdown = sync::Arc::new(atomic::AtomicBool::new(false));

		// Install the handler. This happens after the server has been created
		// because if creation hangs we just want to kill it immediately.
		signal_hook::flag::register(SIGTERM, sync::Arc::clone(&shutdown))?;
		signal_hook::flag::register(SIGHUP, sync::Arc::clone(&shutdown))?;
		// TODO replace SIGHUP with SIGBREAK on Windows?

		while !self.terminating
		{
			if shutdown.load(atomic::Ordering::Relaxed)
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

			self.login.update();

			thread::sleep(time::Duration::from_millis(100));
		}

		Ok(())
	}
}
