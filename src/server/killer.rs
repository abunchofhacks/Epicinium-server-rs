/* Killer */

use signal_hook::iterator::Signals;
use signal_hook::{SIGHUP, SIGTERM};

use futures::future::Future;
use futures::future::IntoFuture;
use futures::stream::Stream;

use tokio::sync::watch;

pub fn start_task(
	mut watchers: watch::Sender<u8>,
) -> impl Future<Item = (), Error = ()> + Send
{
	let signals = Signals::new(&[SIGTERM, SIGHUP])
		.and_then(|x| x.into_async())
		.into_future()
		.flatten_stream()
		.map_err(|error| KillerError::Signals { error });

	signals
		.fold(0, move |killcount, _signal| {
			if killcount < 2
			{
				watchers
					.broadcast(killcount + 1)
					.map_err(|error| KillerError::Broadcast { error })
					.map(|()| killcount + 1)
			}
			else
			{
				Err(KillerError::TripleTerminate)
			}
		})
		.map(|_killcount| debug_assert!(false, "Kill task dropped"))
		.map_err(|error| eprintln!("Error in killer: {:?}", error))
}

#[derive(Debug)]
enum KillerError
{
	Signals
	{
		error: std::io::Error,
	},
	Broadcast
	{
		error: watch::error::SendError<u8>,
	},
	TripleTerminate,
}
