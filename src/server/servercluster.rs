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

pub fn run_server() -> io::Result<()>
{
	let shutdown = sync::Arc::new(atomic::AtomicBool::new(false));

	// Install the handler. This happens after the server has been created
	// because if creation hangs we just want to kill it immediately.
	signal_hook::flag::register(SIGTERM, shutdown.clone())?;
	signal_hook::flag::register(SIGHUP, shutdown.clone())?;
	// TODO replace SIGHUP with SIGBREAK on Windows?

	let (join_in, join_out) = sync::mpsc::channel::<ServerClient>();
	let (leave_in, leave_out) = sync::mpsc::channel::<ServerClient>();

	// ClientCluster should be fully closed before LoginCluster is destroyed.
	let client_closed = sync::Arc::new(atomic::AtomicBool::new(false));

	let login_dep = client_closed.clone();
	let login_shutdown = shutdown.clone();
	let login_thread = thread::spawn(move || {
		let mut cluster = LoginCluster::create(join_in, leave_out, login_dep)?;
		while !cluster.closed()
		{
			if login_shutdown.load(atomic::Ordering::Relaxed)
			{
				cluster.close();
			}
			cluster.update();
		}
		Ok(())
	});

	let client_shutdown = shutdown.clone();
	let client_thread = thread::spawn(move || {
		let mut cluster = ClientCluster::create(join_out, leave_in)?;
		while !cluster.closed()
		{
			if client_shutdown.load(atomic::Ordering::Relaxed)
			{
				cluster.close();
			}
			cluster.update();
		}
		client_closed.store(true, atomic::Ordering::Relaxed);
		Ok(())
	});

	drop(shutdown);

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
