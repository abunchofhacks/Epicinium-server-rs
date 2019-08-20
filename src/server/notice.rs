/* Notice */

use server::message::*;

use std::io;

use tokio::prelude::*;
use tokio::sync::mpsc;

pub fn send_notice(
	mut socket: mpsc::Sender<Message>,
) -> impl Future<Item = (), Error = io::Error>
{
	load_notice()
		.and_then(move |notice| {
			socket
				.try_send(Message::Stamp { metadata: notice })
				.map_err(|error| NoticeError::Send { error })
		})
		.or_else(|e| match e
		{
			NoticeError::Read { error } =>
			{
				eprintln!("Failed to load stamp: {:?}", error);
				Ok(())
			}
			NoticeError::Utf8 { error } =>
			{
				eprintln!("Failed to interpret stamp as utf8: {:?}", error);
				Ok(())
			}
			NoticeError::Parse { error } =>
			{
				eprintln!("Failed to parse stamp: {:?}", error);
				Ok(())
			}
			NoticeError::Send { error } =>
			{
				eprintln!("Failed to send stamp: {:?}", error);
				Err(io::Error::new(io::ErrorKind::ConnectionReset, error))
			}
		})
}

fn load_notice() -> impl Future<Item = StampMetadata, Error = NoticeError>
{
	tokio::fs::read("server-notice.json")
		.map_err(|error| NoticeError::Read { error })
		.and_then(|buffer| {
			String::from_utf8(buffer)
				.map_err(|error| NoticeError::Utf8 { error })
		})
		.and_then(|raw| {
			serde_json::from_str::<StampMetadata>(&raw)
				.map_err(|error| NoticeError::Parse { error })
		})
}

enum NoticeError
{
	Read
	{
		error: io::Error
	},
	Utf8
	{
		error: std::string::FromUtf8Error
	},
	Parse
	{
		error: serde_json::Error
	},
	Send
	{
		error: mpsc::error::TrySendError<Message>,
	},
}
