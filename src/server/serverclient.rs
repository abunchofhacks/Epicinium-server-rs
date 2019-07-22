/* ServerClient */

use common::version::*;
use server::limits;
use server::message::*;

use std::collections::VecDeque;
use std::io;
use std::io::{Read, Write};
use std::net;

pub struct ServerClient
{
	stream: net::TcpStream,
	active_receive_length: Option<u32>,
	already_sent_amount: usize,
	sendqueue: VecDeque<Vec<u8>>,

	pub version: Version,
	pub platform: Platform,
	pub patchmode: Patchmode,
	pub killed: bool,
}

impl ServerClient
{
	pub fn create(
		stream: io::Result<net::TcpStream>,
	) -> io::Result<ServerClient>
	{
		let stream = stream?;

		println!("Incoming connection: {:?}", stream.peer_addr()?);

		stream.set_nonblocking(true)?;

		Ok(ServerClient {
			stream: stream,
			active_receive_length: None,
			already_sent_amount: 0,
			sendqueue: VecDeque::new(),
			version: Version::undefined(),
			platform: Platform::Unknown,
			patchmode: Patchmode::None,
			killed: false,
		})
	}

	pub fn receive(&mut self) -> io::Result<Message>
	{
		let length: u32;
		match self.active_receive_length
		{
			Some(len) =>
			{
				length = len;
			}
			None =>
			{
				let mut lengthbuffer = [0u8; 4];
				self.stream.read_exact(&mut lengthbuffer)?;

				length = u32_from_little_endian_bytes(&lengthbuffer);
				self.active_receive_length = Some(length);
			}
		}

		println!("Receiving message of length {}...", length);

		let mut buffer = vec![0; length as usize];
		self.stream.read_exact(&mut buffer)?;
		self.active_receive_length = None;

		println!("Received message of length {}.", length);

		// TODO if download
		if buffer.len() == 0
		{
			Ok(Message::Pulse)
		}
		else if buffer[0] == '=' as u8
		{
			panic!("Not implemented yet.");
		}
		else
		{
			let jsonstr = match String::from_utf8(buffer)
			{
				Ok(x) => x,
				Err(e) =>
				{
					return Err(io::Error::new(io::ErrorKind::InvalidData, e));
				}
			};

			if jsonstr.len() < 200
			{
				println!("Received message: {}", jsonstr);
			}

			let message: Message = serde_json::from_str(&jsonstr)?;

			Ok(message)
		}
	}

	pub fn pulse(&mut self)
	{
		println!("Queuing pulse...");

		let zeroes = [0u8; 4];
		self.sendqueue.push_back(zeroes.to_vec());

		println!("Queued pulse.");
	}

	pub fn send(&mut self, message: Message)
	{
		let jsonstr = match serde_json::to_string(&message)
		{
			Ok(data) => data,
			Err(e) =>
			{
				eprintln!("Invalid message: {:?}", e);
				self.killed = true;
				return;
			}
		};

		if jsonstr.len() >= limits::MESSAGE_SIZE_LIMIT
		{
			panic!(
				"Cannot send message of length {}, \
				 which is larger than MESSAGE_SIZE_LIMIT.",
				jsonstr.len()
			);
		}

		let length = jsonstr.len() as u32;

		println!("Queueing message of length {}...", length);

		let lengthbuffer = little_endian_bytes_from_u32(length);
		self.sendqueue.push_back(lengthbuffer.to_vec());

		let data = jsonstr.as_bytes();
		self.sendqueue.push_back(data.to_vec());

		println!("Queued message of length {}.", length);

		if length < 200
		{
			println!("Queued message: {}", jsonstr);
		}
	}

	pub fn has_queued(&self) -> bool
	{
		!self.sendqueue.is_empty()
	}

	pub fn send_queued(&mut self) -> io::Result<()>
	{
		match self.sendqueue.get(0)
		{
			Some(data) =>
			{
				let remainingdata = &data[self.already_sent_amount..];

				println!("Sending {} bytes...", remainingdata.len());

				match self.stream.write(remainingdata)
				{
					Ok(n) if n == remainingdata.len() =>
					{
						println!("Sent {} bytes.", remainingdata.len());

						self.sendqueue.pop_front();
						self.already_sent_amount = 0;

						Ok(())
					}
					Ok(n) =>
					{
						self.already_sent_amount += n;

						println!(
							"Sent {}/{} bytes.",
							self.already_sent_amount,
							data.len()
						);

						Ok(())
					}
					Err(e) => Err(e),
				}
			}
			None => Ok(()),
		}
	}
}

#[allow(dead_code)]
fn u32_from_big_endian_bytes(data: &[u8; 4]) -> u32
{
	(/*  */(data[0] as u32) << 24)
		+ ((data[1] as u32) << 16)
		+ ((data[2] as u32) << 8)
		+ ((data[3] as u32) << 0)
}

fn u32_from_little_endian_bytes(data: &[u8; 4]) -> u32
{
	(/*  */(data[0] as u32) << 0)
		+ ((data[1] as u32) << 8)
		+ ((data[2] as u32) << 16)
		+ ((data[3] as u32) << 24)
}

fn little_endian_bytes_from_u32(x: u32) -> [u8; 4]
{
	[
		(x & 0xFF) as u8,
		((x >> 8) & 0xFF) as u8,
		((x >> 16) & 0xFF) as u8,
		((x >> 24) & 0xFF) as u8,
	]
}
