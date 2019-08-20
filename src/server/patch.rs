/* Patch */

use server::message::*;

use std::io;
use std::path::*;

use tokio::prelude::*;
use tokio::sync::mpsc;

pub fn fulfil_request(
	sendbuffer: mpsc::Sender<Message>,
	name: String,
) -> impl Future<Item = (), Error = io::Error>
{
	future::ok(PathBuf::from(name.clone()))
		.and_then(|filename| {
			if is_requestable(&filename)
			{
				Ok(filename)
			}
			else
			{
				Err(RequestError::IsNotRequestable)
			}
		})
		.and_then(move |filename| send_file(sendbuffer, &filename))
		.and_then(|_checksum| {
			// TODO implement
			Ok(())
		})
		.or_else(move |error| match error
		{
			RequestError::IsNotRequestable =>
			{
				let message = Message::RequestDenied {
					content: name,
					metadata: DenyMetadata {
						reason: "File not requestable.".to_string(),
					},
				};
				// TODO sendbuffer.try_send(message)
				let _ = message;
				Ok(())
			}
		})
}

enum RequestError
{
	IsNotRequestable,
	//DoesNotExist,
}

fn is_requestable(filepath: &Path) -> bool
{
	(is_picture(&filepath) || is_ruleset(&filepath) || is_fzmodel(&filepath))
		&& filepath.is_relative()
}

fn is_picture(filepath: &Path) -> bool
{
	filepath.starts_with("pictures/")
		&& match filepath.extension()
		{
			Some(x) => x == "png",
			None => false,
		}
}

fn is_ruleset(filepath: &Path) -> bool
{
	filepath.starts_with("rulesets/")
		&& match filepath.extension()
		{
			Some(x) => x == "json",
			None => false,
		}
}

fn is_fzmodel(filepath: &Path) -> bool
{
	filepath.starts_with("sessions/")
		&& match filepath.extension()
		{
			Some(x) => x == "fzm",
			None => false,
		}
}

fn send_file(
	sendbuffer: mpsc::Sender<Message>,
	filename: &Path,
) -> impl Future<Item = Vec<u8>, Error = RequestError>
{
	// TODO implement
	future::ok([0u8; 100].to_vec())
}
