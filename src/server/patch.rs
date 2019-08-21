/* Patch */

use server::message::*;

use std::io;
use std::path::*;

use tokio::prelude::*;

pub fn fulfil_request(
	name: String,
) -> impl Stream<Item = Result<PathBuf, Message>, Error = io::Error>
{
	future::ok(PathBuf::from(name.clone()))
		.into_stream()
		.map(move |filename| {
			if is_requestable(&filename)
			{
				Ok(filename)
			}
			else
			{
				let message = Message::RequestDenied {
					content: name.clone(),
					metadata: DenyMetadata {
						reason: "File not requestable.".to_string(),
					},
				};
				Err(message)
			}
		})
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
