/* ServerCluster */

use server::logincluster::*;

use std::thread;
use std::time;

pub struct ServerCluster
{
	login: LoginCluster,
	killcount: i32,
	closing: bool,
}

impl ServerCluster
{
	pub fn create() -> ServerCluster
	{
		ServerCluster {
			login: LoginCluster::create(),
			killcount: 0,
			closing: false,
		}
	}

	pub fn run(&mut self)
	{
		let mut lastkillcount = self.killcount;

		while self.killcount <= 1
		{
			if lastkillcount != self.killcount
			{
				if self.killcount == 1
				{
					self.closing = true;
					self.login.close();
				}

				lastkillcount = self.killcount;
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
	}
}
