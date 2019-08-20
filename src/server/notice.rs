/* Notice */

use server::message::*;

use std::fs;
use std::io;

use tokio::prelude::*;
use tokio::sync::mpsc;

pub type NoticeService = mpsc::Sender<mpsc::Sender<Message>>;

pub fn run_notice_service() -> io::Result<NoticeService>
{
	let (handle, requests) = mpsc::channel::<mpsc::Sender<Message>>(1000);
	let notice_task = requests
		.for_each(|mut socket| {
			if let Some(notice) = load_notice()
			{
				let future = socket
					.try_send(Message::Stamp { metadata: notice })
					.or_else(|e| {
						eprintln!("Failed to send stamp: {:?}", e);
						Ok(())
					});
				future
			}
			else
			{
				Ok(())
			}
		})
		.map_err(|e| {
			eprintln!("Error in notice task: {:?}", e);
		});

	tokio::spawn(notice_task);

	Ok(handle)
}

fn load_notice() -> Option<StampMetadata>
{
	match fs::read_to_string("server-notice.json")
	{
		Ok(raw) => match serde_json::from_str::<StampMetadata>(&raw)
		{
			Ok(value) => Some(value),
			Err(e) =>
			{
				eprintln!("Notice file could not be loaded: {:?}", e);
				None
			}
		},
		Err(e) =>
		{
			eprintln!("Notice file could not be loaded: {:?}", e);
			None
		}
	}
}
