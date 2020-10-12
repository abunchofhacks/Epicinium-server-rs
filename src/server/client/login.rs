/**/

use crate::server::message::*;

pub use crate::server::login::LoginData;
pub use crate::server::login::Request;
pub use crate::server::login::Server;

use std::sync;

use log::*;

use tokio::sync::mpsc;

pub async fn run(
	mut sendbuffer: mpsc::Sender<Message>,
	mut joinedbuffer: mpsc::Sender<LoginData>,
	mut requestbuffer: mpsc::Receiver<Request>,
	login_server: sync::Arc<Server>,
) -> Result<(), Error>
{
	while let Some(request) = requestbuffer.recv().await
	{
		match login_server.login(request).await
		{
			Ok(logindata) =>
			{
				joinedbuffer.send(logindata).await?;
			}
			Err(responsestatus) =>
			{
				debug!("Login failed with {:?}", responsestatus);
				let message = Message::JoinServer {
					status: Some(responsestatus),
					content: None,
					sender: None,
					metadata: Default::default(),
				};
				sendbuffer.send(message).await?;
			}
		}
	}

	Ok(())
}

#[derive(Debug)]
pub enum Error
{
	SendLoginData(mpsc::error::SendError<LoginData>),
	SendMessage(mpsc::error::SendError<Message>),
}

impl From<mpsc::error::SendError<LoginData>> for Error
{
	fn from(error: mpsc::error::SendError<LoginData>) -> Error
	{
		Error::SendLoginData(error)
	}
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
			Error::SendLoginData(error) => error.fmt(f),
			Error::SendMessage(error) => error.fmt(f),
		}
	}
}
