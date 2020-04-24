/* Killer */

use tokio::signal::unix::SignalKind;
use tokio::sync::watch;

pub async fn run(watchers: watch::Sender<u8>) -> Result<(), std::io::Error>
{
	let mut signals = tokio::signal::unix::signal(SignalKind::terminate())?;

	let mut killcount = 0;

	loop
	{
		signals.recv().await;
		killcount += 1;
		match watchers.broadcast(killcount)
		{
			Ok(()) =>
			{}
			Err(_error) => break,
		}
	}

	Ok(())
}
