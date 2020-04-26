/* Killer */

use std::error;

use futures::select;
use futures::FutureExt;

use tokio::signal::unix::SignalKind;
use tokio::sync::watch;

pub async fn run(
	mut watchers: watch::Sender<u8>,
) -> Result<(), Box<dyn error::Error>>
{
	let mut signals = tokio::signal::unix::signal(SignalKind::terminate())?;

	let mut killcount = 0;

	loop
	{
		let signal = select! {
			() = watchers.closed().fuse() => break,
			x = signals.recv().fuse() => x,
		};
		match signal
		{
			Some(()) =>
			{
				killcount += 1;
				watchers.broadcast(killcount)?;
			}
			None => break,
		}
	}

	println!("Killer ended.");
	Ok(())
}
