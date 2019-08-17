/* ServerCluster */

use server::clientcluster::*;
use server::logincluster::*;
use server::serverclient::*;
use server::settings::*;

use signal_hook;
use signal_hook::{SIGHUP, SIGTERM};
use std::io;
use std::sync;
use std::sync::atomic;
use std::thread;
use std::time;

use futures::Future;

pub fn run_server(settings: &Settings) -> io::Result<()>
{
	let shutdown = sync::Arc::new(atomic::AtomicBool::new(false));
	let shutdown_killcount = sync::Arc::new(atomic::AtomicU8::new(0));

	signal_hook::flag::register(SIGTERM, shutdown.clone())?;
	signal_hook::flag::register(SIGHUP, shutdown.clone())?;
	// On Windows we would want to use SIGBREAK instead of SIGHUP, but the
	// crate we use does not support that.

	// LoginCluster should be fully closed before this thread ends.
	let login_closed = sync::Arc::new(atomic::AtomicBool::new(false));

	let signal_dep = login_closed.clone();
	let signal_killcount = shutdown_killcount.clone();
	let signal_future = futures::future::loop_fn((), move |_| {
		if shutdown.load(atomic::Ordering::Relaxed)
		{
			shutdown.store(false, atomic::Ordering::Relaxed);
			signal_killcount.fetch_add(1, atomic::Ordering::Relaxed);
		}

		if false
		{
			// Instead of declaring the type of signal_future, let the compiler
			// infer it by pretending that this closure can return io::Error.
			Err(io::Error::new(io::ErrorKind::Other, "dummy"))
		}
		else if signal_dep.load(atomic::Ordering::Relaxed)
		{
			Ok(futures::future::Loop::Break(()))
		}
		else
		{
			//thread::sleep(time::Duration::from_millis(50));

			Ok(futures::future::Loop::Continue(()))
		}
	});

	let (join_in, join_out) = sync::mpsc::channel::<ServerClient>();
	let (leave_in, leave_out) = sync::mpsc::channel::<ServerClient>();

	// ClientCluster should be fully closed before LoginCluster is destroyed.
	let client_closed = sync::Arc::new(atomic::AtomicBool::new(false));

	let login_dep = client_closed.clone();
	let login_killcount = shutdown_killcount.clone();
	let mut login_cluster =
		LoginCluster::create(settings, join_in, leave_out, login_dep)?;

	let login_future = futures::future::loop_fn(0, move |last_killcount| {
		let killcount = login_killcount.load(atomic::Ordering::Relaxed);
		if killcount > last_killcount
		{
			if killcount == 1
			{
				login_cluster.close();
			}
			else if killcount == 2
			{
				login_cluster.close_and_kick();
			}
			else
			{
				login_cluster.close_and_terminate();
			}
		}
		login_cluster.update();

		if login_cluster.closed()
		{
			login_closed.store(true, atomic::Ordering::Relaxed);

			Ok(futures::future::Loop::Break(killcount))
		}
		else
		{
			// TODO remove or replace with socketsets
			//thread::sleep(time::Duration::from_millis(10));

			Ok(futures::future::Loop::Continue(killcount))
		}
	});

	let client_killcount = shutdown_killcount.clone();
	let mut client_cluster =
		ClientCluster::create(settings, join_out, leave_in)?;

	let client_future = futures::future::loop_fn(0, move |last_killcount| {
		let killcount = client_killcount.load(atomic::Ordering::Relaxed);
		if killcount > last_killcount
		{
			if killcount == 1
			{
				client_cluster.close();
			}
			else if killcount == 2
			{
				client_cluster.close_and_kick();
			}
			else
			{
				client_cluster.close_and_terminate();
			}
		}
		client_cluster.update();

		if client_cluster.closed()
		{
			client_closed.store(true, atomic::Ordering::Relaxed);

			Ok(futures::future::Loop::Break(killcount))
		}
		else
		{
			// TODO remove or replace with socketsets
			//thread::sleep(time::Duration::from_millis(10));

			Ok(futures::future::Loop::Continue(killcount))
		}
	});

	signal_future
		.join3(login_future, client_future)
		.map(|_| ())
		.wait()?;

	Ok(())
}
