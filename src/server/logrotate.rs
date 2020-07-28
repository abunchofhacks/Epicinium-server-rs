/* Server::LogRotate */

pub use crate::common::logrotate::Setup;

use crate::server::slack_api;
use crate::server::tokio::State as ServerState;

use log::*;

use futures::{FutureExt, StreamExt};

use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::time::Duration;

pub async fn run(
	setup: Setup,
	server_state: watch::Receiver<ServerState>,
	mut slack_api: mpsc::Sender<slack_api::Post>,
)
{
	let interval = tokio::time::interval(Duration::from_secs(300));
	let closed = wait_until_closed(server_state).boxed();
	let mut events = interval.take_until(closed);

	while let Some(_event) = events.next().await
	{
		do_check(&setup, &mut slack_api).await;
	}
}

async fn wait_until_closed(mut server_state: watch::Receiver<ServerState>)
{
	while let Some(state) = server_state.recv().await
	{
		match state
		{
			ServerState::Open => (),
			ServerState::Closing => (),
			ServerState::Closed => break,
		}
	}
}

async fn do_check(setup: &Setup, slack_api: &mut mpsc::Sender<slack_api::Post>)
{
	trace!("Checking...");
	match check(&setup).await
	{
		Ok(()) =>
		{
			trace!("Checked.");
		}
		Err(error) =>
		{
			error!("Error rotating logs: {:?}", error);

			let post = slack_api::Post {
				message: "Failed to rotate logs.".to_string(),
			};
			match slack_api.try_send(post)
			{
				Ok(()) => (),
				Err(e) => error!("Error reporting error: {:?}", e),
			}
		}
	}
}

async fn check(setup: &Setup) -> Result<(), Error>
{
	let command = tokio::process::Command::new("logrotate")
		.arg("--state")
		.arg(&setup.statusfilename)
		.arg(&setup.conffilename)
		.status();
	let status = command.await?;
	if status.success()
	{
		Ok(())
	}
	else
	{
		Err(Error::Failure(status))
	}
}

#[derive(Debug)]
enum Error
{
	Io(tokio::io::Error),
	Failure(std::process::ExitStatus),
}

impl From<tokio::io::Error> for Error
{
	fn from(error: tokio::io::Error) -> Error
	{
		Error::Io(error)
	}
}

impl std::fmt::Display for Error
{
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
	{
		match self
		{
			Error::Io(error) => error.fmt(f),
			Error::Failure(status) =>
			{
				write!(f, "exited with status {}", status)
			}
		}
	}
}

impl std::error::Error for Error {}
