/*
 * Part of epicinium_server
 * developed by A Bunch of Hacks.
 *
 * Copyright (c) 2018-2021 A Bunch of Hacks
 *
 * This library is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This library is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 * [authors:]
 * Sander in 't Veld (sander@abunchofhacks.coop)
 */

use super::limit::*;

use crate::common::keycode::Keycode;
use crate::server::message::*;

use std::sync;
use std::sync::atomic;

use log::*;

use tokio::io::ReadHalf;
use tokio::net::TcpStream;
use tokio::prelude::*;

use itertools::Itertools;

pub struct Client
{
	pub socket: ReadHalf<TcpStream>,
	pub client_id: Keycode,
	pub has_proper_version: sync::Arc<atomic::AtomicBool>,
}

impl Client
{
	pub async fn receive(&mut self) -> Result<Message, Error>
	{
		let socket = &mut self.socket;
		let id = self.client_id;
		let versioned = self.has_proper_version.load(atomic::Ordering::Relaxed);
		let message = receive_message(socket, id, versioned).await?;
		Ok(message)
	}
}

async fn receive_message(
	socket: &mut ReadHalf<TcpStream>,
	client_id: Keycode,
	versioned: bool,
) -> Result<Message, Error>
{
	trace!("Starting to receive...");
	let length = socket.read_u32().await?;

	if length == 0
	{
		trace!("Received pulse.");

		return Ok(Message::Pulse);
	}
	else if !versioned && length as usize >= MESSAGE_SIZE_UNVERSIONED_LIMIT
	{
		warn!(
			"Unversioned client {} tried to send \
			 very large message of length {}, \
			 which is more than MESSAGE_SIZE_UNVERSIONED_LIMIT.",
			client_id, length
		);
		return Err(Error::MessageTooLargeFromUnversioned { length });
	}
	else if length as usize >= MESSAGE_SIZE_LIMIT
	{
		warn!(
			"Client {} tried to send very large message of length {}, \
			 which is more than MESSAGE_SIZE_LIMIT.",
			client_id, length
		);
		return Err(Error::MessageTooLarge { length });
	}
	else if length as usize >= MESSAGE_SIZE_WARNING_LIMIT
	{
		warn!("Receiving very large message of length {}...", length);
	}

	trace!("Receiving message of length {}...", length);

	let mut buffer = vec![0; length as usize];
	socket.read_exact(&mut buffer).await?;

	trace!("Received message of length {}.", buffer.len());
	let message = parse_message(buffer)?;

	Ok(message)
}

fn parse_message(buffer: Vec<u8>) -> Result<Message, Error>
{
	let jsonstr = String::from_utf8(buffer)?;

	if log_enabled!(log::Level::Trace)
	{
		trace!(
			"Received message: {}",
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
