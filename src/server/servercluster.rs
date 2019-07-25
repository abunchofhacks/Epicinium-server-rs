/* ServerCluster */

use server::clientcluster::*;
use server::logincluster::*;
use server::serverclient::*;

use signal_hook;
use signal_hook::{SIGHUP, SIGTERM};
use std::io;
use std::sync;
use std::sync::atomic;
use std::thread;
use std::time;

pub fn run_server() -> io::Result<()>
{
	let shutdown = sync::Arc::new(atomic::AtomicBool::new(false));

	let (join_in, join_out) = sync::mpsc::channel::<ServerClient>();
	let (leave_in, leave_out) = sync::mpsc::channel::<ServerClient>();

	let login_thread = thread::spawn(move || {
		let mut cluster = LoginCluster::create(join_in, leave_out)?;
		while !cluster.closed()
		{
			cluster.update();
		}
		Ok(())
	});
	let client_thread = thread::spawn(move || {
		let mut cluster = ClientCluster::create(join_out, leave_in)?;
		while !cluster.closed()
		{
			cluster.update();
		}
		Ok(())
	});

	let mut closing = false;
	let mut terminating = false;

	// Install the handler. This happens after the server has been created
	// because if creation hangs we just want to kill it immediately.
	signal_hook::flag::register(SIGTERM, shutdown.clone())?;
	signal_hook::flag::register(SIGHUP, shutdown.clone())?;
	// TODO replace SIGHUP with SIGBREAK on Windows?

	while !terminating
	{
		if shutdown.load(atomic::Ordering::Relaxed)
		{
			if closing
			{
				terminating = true;
			}
			else
			{
				closing = true;
			}
		}

		thread::sleep(time::Duration::from_millis(100));
	}

	match login_thread.join()
	{
		Ok(x) => match x
		{
			Ok(()) =>
			{}
			Err(e) =>
			{
				return Err(e);
			}
		},
		Err(e) =>
		{
			panic!("Thread panicked: {:?}", e);
		}
	}

	match client_thread.join()
	{
		Ok(x) => match x
		{
			Ok(()) =>
			{}
			Err(e) =>
			{
				return Err(e);
			}
		},
		Err(e) =>
		{
			panic!("Thread panicked: {:?}", e);
		}
	}

	Ok(())
}
