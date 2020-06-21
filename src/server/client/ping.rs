/**/

use crate::common::keycode::Keycode;

use futures::select;
use futures::{FutureExt, TryFutureExt};

use tokio::sync::mpsc;
use tokio::sync::oneshot;
use tokio::sync::watch;
use tokio::time as timer;
use tokio::time::{Duration, Instant};

#[derive(Debug)]
pub struct Request
{
	pub callback: oneshot::Sender<()>,
}

pub async fn run(
	client_id: Keycode,
	mut sendbuffer: mpsc::Sender<Request>,
	mut activity: watch::Receiver<()>,
	mut ping_tolerance: watch::Receiver<Duration>,
) -> Result<(), Error>
{
	loop
	{
		let (callback_in, callback_out) = oneshot::channel::<()>();
		let request = Request {
			callback: callback_in,
		};
		sendbuffer.send(request).await?;

		wait_for_pong(client_id, callback_out, &mut ping_tolerance).await?;

		wait_for_inactivity(&mut activity, &mut ping_tolerance).await?;
	}
}

async fn wait_for_inactivity(
	activity: &mut watch::Receiver<()>,
	trigger: &mut watch::Receiver<Duration>,
) -> Result<(), Error>
{
	loop
	{
		let threshold = Duration::from_secs(5);
		let activity_event = activity.recv().map(|x| match x
		{
			Some(()) => Ok(PingEvent::Activity),
			None => Err(Error::NoMoreActivity),
		});
		let trigger_event = trigger.recv().map(|x| match x
		{
			Some(_) => Ok(PingEvent::Forced),
			None => Err(Error::NoMoreActivity),
		});
		let timeout_event =
			timer::delay_for(threshold).map(|()| PingEvent::Timeout);
		let event = select! {
			x = activity_event.fuse() => x?,
			x = trigger_event.fuse() => x?,
			x = timeout_event.fuse() => x,
		};
		match event
		{
			PingEvent::Activity => continue,
			PingEvent::Forced => return Ok(()),
			PingEvent::Timeout => return Ok(()),
		}
	}
}

async fn wait_for_pong(
	client_id: Keycode,
	callback: oneshot::Receiver<()>,
	tolerance_updates: &mut watch::Receiver<Duration>,
) -> Result<(), Error>
{
	let sendtime = Instant::now();
	let mut tolerance: Duration = *tolerance_updates.borrow();

	let mut received_event = callback.map_ok(|()| PongEvent::Received).fuse();
	loop
	{
		let tolerance_event = tolerance_updates.recv().map(|x| match x
		{
			Some(value) => Ok(PongEvent::NewTolerance { value }),
			None => Err(Error::NoMoreActivity),
		});
		let timeout_event = timer::delay_until(sendtime + tolerance)
			.map(|()| PongEvent::Timeout);
		let event = select! {
			x = tolerance_event.fuse() => x?,
			x = received_event => x?,
			x = timeout_event.fuse() => x,
		};
		match event
		{
			PongEvent::NewTolerance { value } =>
			{
				tolerance = value;
			}
			PongEvent::Timeout =>
			{
				eprintln!("Disconnecting inactive client {}", client_id);
				// TODO slack
				return Err(Error::Timeout);
			}
			PongEvent::Received => break,
		}
	}

	println!(
		"Client {} has {}ms ping",
		client_id,
		sendtime.elapsed().as_millis()
	);

	Ok(())
}

enum PingEvent
{
	Activity,
	Timeout,
	Forced,
}

enum PongEvent
{
	NewTolerance
	{
		value: Duration,
	},
	Timeout,
	Received,
}

#[derive(Debug)]
pub enum Error
{
	Send(mpsc::error::SendError<Request>),
	Receive(oneshot::error::RecvError),
	NoMoreActivity,
	Timeout,
}

impl From<mpsc::error::SendError<Request>> for Error
{
	fn from(error: mpsc::error::SendError<Request>) -> Error
	{
		Error::Send(error)
	}
}

impl From<oneshot::error::RecvError> for Error
{
	fn from(error: oneshot::error::RecvError) -> Error
	{
		Error::Receive(error)
	}
}

impl std::fmt::Display for Error
{
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
	{
		match self
		{
			Error::Send(error) => error.fmt(f),
			Error::Receive(error) => error.fmt(f),
			Error::NoMoreActivity => write!(f, "Activity stream ended"),
			Error::Timeout => write!(f, "Client failed to respond in time"),
		}
	}
}
