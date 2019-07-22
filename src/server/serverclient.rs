/* ServerClient */

use std::io;
use std::io::Read;
use std::net;

pub struct ServerClient
{
	stream: net::TcpStream,
	last_length: Option<u32>,

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
			last_length: None,
			killed: false,
		})
	}

	pub fn receive(&mut self) -> io::Result<String>
	{
		let length: u32;
		match self.last_length
		{
			Some(len) =>
			{
				length = len;
			}
			None =>
			{
				let mut lengthbuffer = [0u8; 4];
				self.stream.read_exact(&mut lengthbuffer)?;
				println!("Read bytes {:?}", lengthbuffer);

				length = u32_from_little_endian_bytes(&lengthbuffer);
				self.last_length = Some(length);
			}
		}

		println!("Receiving message of length {}", length);

		let mut buffer = vec![0; length as usize];
		self.stream.read_exact(&mut buffer)?;
		self.last_length = None;

		println!("Received message of length {}", length);
		if length < 100
		{
			println!("Received message: {:?}", buffer);
		}

		// TODO if download
		if buffer.len() == 0
		{
			Ok("".to_string())
		}
		else if buffer[0] == '=' as u8
		{
			Err(io::Error::new(
				io::ErrorKind::InvalidData,
				"Not implemented yet.",
			))
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

			Ok(jsonstr)
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
