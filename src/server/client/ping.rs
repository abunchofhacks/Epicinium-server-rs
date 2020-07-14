/**/

use crate::common::keycode::Keycode;

use log::*;

use futures::stream;
use futures::StreamExt;

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
	let threshold = Duration::from_secs(5);
	let activity_events = activity.map(|()| PingEvent::Activity);
	let trigger_events = trigger.map(|_duration| PingEvent::Forced);
	let mut events = stream::select(activity_events, trigger_events);

	while let Ok(event) = timer::timeout(threshold, events.next()).await
	{
		match event
		{
			Some(PingEvent::Activity) => continue,
			Some(PingEvent::Forced) => break,
			None => return Err(Error::NoMoreActivity),
		}
	}

	Ok(())
}

async fn wait_for_pong(
	client_id: Keycode,
	callback: oneshot::Receiver<()>,
	tolerance_updates: &mut watch::Receiver<Duration>,
) -> Result<(), Error>
{
	let sendtime = Instant::now();
	let tolerance: Duration = *tolerance_updates.borrow();
	let mut endtime = sendtime + tolerance;

	let mut events = tolerance_updates.take_until(callback);

	while let Some(result) =
		timer::timeout_at(endtime, events.next()).await.transpose()
	{
		if let Ok(tolerance) = result
		{
			endtime = sendtime + tolerance;
		}
		else
		{
			warn!("Disconnecting inactive client {}...", client_id);
			// TODO slack
			return Err(Error::Timeout);
		}
	}

	if let Some(Ok(())) = events.take_result()
	{
		debug!(
			"Client {} has {}ms ping.",
			client_id,
			sendtime.elapsed().as_millis()
		);

		Ok(())
	}
	else
	{
		Err(Error::NoMoreActivity)
	}
}

enum PingEvent
{
	Activity,
	Forced,
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
