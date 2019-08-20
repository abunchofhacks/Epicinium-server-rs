/* Notice */

use server::message::*;

use std::io;

use tokio::prelude::*;
use tokio::sync::mpsc;

pub type NoticeService = mpsc::Sender<mpsc::Sender<Message>>;

pub fn run_notice_service() -> io::Result<NoticeService>
{
	let (handle, requests) = mpsc::channel::<mpsc::Sender<Message>>(1000);
	let notice_task = requests
		.map_err(|e| {
			eprintln!("Error in notice task: {:?}", e);
		})
		.for_each(send_notice);

	tokio::spawn(notice_task);

	Ok(handle)
}

fn send_notice(
	mut socket: mpsc::Sender<Message>,
) -> impl Future<Item = (), Error = ()>
{
	load_notice()
		.and_then(move |notice| {
			socket
				.try_send(Message::Stamp { metadata: notice })
				.or_else(|e| {
					eprintln!("Failed to send stamp: {:?}", e);
					Ok(())
				})
		})
		.or_else(|e| {
			match e
			{
				LoadError::Read { error } =>
				{
					eprintln!("Failed to load stamp: {:?}", error)
				}
				LoadError::Utf8 { error } =>
				{
					eprintln!("Failed to interpret stamp as utf8: {:?}", error)
				}
				LoadError::Parse { error } =>
				{
					eprintln!("Failed to parse stamp: {:?}", error)
				}
			}
			Ok(())
		})
}

fn load_notice() -> impl Future<Item = StampMetadata, Error = LoadError>
{
	tokio::fs::read("server-notice.json")
		.map_err(|error| LoadError::Read { error })
		.and_then(|buffer| {
			String::from_utf8(buffer).map_err(|error| LoadError::Utf8 { error })
		})
		.and_then(|raw| {
			serde_json::from_str::<StampMetadata>(&raw)
				.map_err(|error| LoadError::Parse { error })
		})
}

enum LoadError
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
}
