/**/

use super::limit::*;

use crate::common::keycode::Keycode;
use crate::server::message::*;

use log::*;

use futures::pin_mut;
use futures::StreamExt;

use tokio::io::WriteHalf;
use tokio::net::TcpStream;
use tokio::prelude::*;
use tokio::sync::mpsc;

use itertools::Itertools;

pub async fn run(
	client_id: Keycode,
	sendbuffer: mpsc::Receiver<Message>,
	mut socket: WriteHalf<TcpStream>,
) -> Result<(), Error>
{
	let last_messages = futures::stream::once(async { Message::Quit });
	let messages = sendbuffer.chain(last_messages);
	pin_mut!(messages);

	while let Some(message) = messages.next().await
	{
		let buffer = prepare_message(message);
		send_bytes(&mut socket, buffer).await?;
	}

	debug!("Client {} stopped sending.", client_id);
	Ok(())
}

async fn send_bytes(
	socket: &mut WriteHalf<TcpStream>,
	buffer: Vec<u8>,
) -> Result<(), std::io::Error>
{
	socket.write_all(&buffer).await?;

	trace!("Sent {} bytes.", buffer.len());
	Ok(())
}

fn prepare_message(message: Message) -> Vec<u8>
{
	if let Message::Pulse = message
	{
		trace!("Sending pulse...");

		let zeroes = [0u8; 4];
		return zeroes.to_vec();
	}

	let (jsonstr, length) = prepare_message_data(message);

	let mut buffer = length.to_be_bytes().to_vec();
	buffer.append(&mut jsonstr.into_bytes());

	buffer
}

fn prepare_message_data(message: Message) -> (String, u32)
{
	let jsonstr = match serde_json::to_string(&message)
	{
		Ok(data) => data,
		Err(e) =>
		{
			panic!("Invalid message: {:?}", e);
		}
	};

	if jsonstr.len() >= MESSAGE_SIZE_LIMIT
	{
		panic!(
			"Cannot send message of length {}, \
			 which is larger than MESSAGE_SIZE_LIMIT.",
			jsonstr.len()
		);
	}

	let length = jsonstr.len() as u32;

	if length as usize >= MESSAGE_SIZE_WARNING_LIMIT
	{
		warn!("Sending very large message of length {}", length);
	}

	trace!("Sending message of length {}...", length);

	if log_enabled!(log::Level::Trace)
	{
		trace!(
			"Sending message: {}",
			jsonstr
				.chars()
				.take(500)
				.map(|x| {
					if x == '"'
					{
						x.to_string()
					}
					else
					{
						x.escape_debug().to_string()
					}
				})
				.format("")
		);
	}

	(jsonstr, length)
}

#[derive(Debug)]
pub enum Error
{
	Io
	{
		error: std::io::Error
	},
}

impl From<std::io::Error> for Error
{
	fn from(error: std::io::Error) -> Self
	{
		Error::Io { error }
	}
}

impl std::fmt::Display for Error
{
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
	{
		match self
		{
			Error::Io { error } => error.fmt(f),
		}
	}
}
