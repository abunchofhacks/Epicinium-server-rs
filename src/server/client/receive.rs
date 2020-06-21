/**/

use super::limit::*;

use crate::common::keycode::Keycode;
use crate::server::message::*;

use tokio::io::ReadHalf;
use tokio::net::TcpStream;
use tokio::prelude::*;

pub async fn receive_message(
	socket: &mut ReadHalf<TcpStream>,
	client_id: Keycode,
	versioned: bool,
) -> Result<Message, Error>
{
	println!("Starting to receive...");
	let length = socket.read_u32().await?;
	if length == 0
	{
		/*verbose*/
		println!("Received pulse.");

		return Ok(Message::Pulse);
	}
	else if !versioned && length as usize >= MESSAGE_SIZE_UNVERSIONED_LIMIT
	{
		println!(
			"Unversioned client {} tried to send \
			 very large message of length {}, \
			 which is more than MESSAGE_SIZE_UNVERSIONED_LIMIT.",
			client_id, length
		);
		return Err(Error::MessageTooLargeFromUnversioned { length });
	}
	else if length as usize >= MESSAGE_SIZE_LIMIT
	{
		println!(
			"Client {} tried to send very large message of length {}, \
			 which is more than MESSAGE_SIZE_LIMIT.",
			client_id, length
		);
		return Err(Error::MessageTooLarge { length });
	}
	else if length as usize >= MESSAGE_SIZE_WARNING_LIMIT
	{
		println!("Receiving very large message of length {}...", length);
	}

	/*verbose*/
	println!("Receiving message of length {}...", length);

	let mut buffer = vec![0; length as usize];
	socket.read_exact(&mut buffer).await?;

	/*verbose*/
	println!("Received message of length {}.", buffer.len());
	let message = parse_message(buffer)?;

	Ok(message)
}

fn parse_message(buffer: Vec<u8>) -> Result<Message, Error>
{
	let jsonstr = String::from_utf8(buffer)?;

	if jsonstr.len() < 200
	{
		/*verbose*/
		println!("Received message: {}", jsonstr);
	}

	let message: Message = serde_json::from_str(&jsonstr)?;

	Ok(message)
}

#[derive(Debug)]
pub enum Error
{
	MessageTooLarge
	{
		length: u32,
	},
	MessageTooLargeFromUnversioned
	{
		length: u32,
	},
	Io(std::io::Error),
	Utf8(std::string::FromUtf8Error),
	Json(serde_json::Error),
}

impl From<std::io::Error> for Error
{
	fn from(error: std::io::Error) -> Error
	{
		Error::Io(error)
	}
}

impl From<std::string::FromUtf8Error> for Error
{
	fn from(error: std::string::FromUtf8Error) -> Error
	{
		Error::Utf8(error)
	}
}

impl From<serde_json::Error> for Error
{
	fn from(error: serde_json::Error) -> Error
	{
		Error::Json(error)
	}
}

impl std::fmt::Display for Error
{
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
	{
		match self
		{
			Error::MessageTooLarge { length } => write!(
				f,
				"Refusing message of length {}, \
				 which is more than MESSAGE_SIZE_LIMIT.",
				length
			),
			Error::MessageTooLargeFromUnversioned { length } => write!(
				f,
				"Refusing message of length {}, \
				 which is more than MESSAGE_SIZE_UNVERSIONED_LIMIT.",
				length
			),
			Error::Io(error) => error.fmt(f),
			Error::Utf8(error) => error.fmt(f),
			Error::Json(error) => error.fmt(f),
		}
	}
}
