/**/

use crate::server::message::*;

use tokio::sync::mpsc;
use tokio::time as timer;
use tokio::time::{Duration, Instant};

pub async fn run(mut sendbuffer: mpsc::Sender<Message>) -> Result<(), Error>
{
	let start = Instant::now() + Duration::from_secs(4);
	let mut interval = timer::interval_at(start, Duration::from_secs(4));

	loop
	{
		interval.tick().await;
		sendbuffer.send(Message::Pulse).await?;
	}
}

#[derive(Debug)]
pub enum Error
{
	SendMessage(mpsc::error::SendError<Message>),
}

impl From<mpsc::error::SendError<Message>> for Error
{
	fn from(error: mpsc::error::SendError<Message>) -> Error
	{
		Error::SendMessage(error)
	}
}

impl std::fmt::Display for Error
{
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
	{
		match self
		{
			Error::SendMessage(error) => error.fmt(f),
		}
	}
}
