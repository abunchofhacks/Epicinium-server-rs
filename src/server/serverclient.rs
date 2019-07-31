/* ServerClient */

use common::keycode::*;
use common::version::*;
use server::limits::*;
use server::message::*;
use server::patch::*;

use std::collections::VecDeque;
use std::fs;
use std::fs::File;
use std::io;
use std::io::{Read, Write};
use std::net;
use std::path;
use std::time;

pub struct ServerClient
{
	stream: net::TcpStream,
	active_receive_length: Option<u32>,
	chunk_incoming: bool,
	already_sent_amount: usize,
	sendqueue: VecDeque<Vec<u8>>,

	pub version: Version,
	pub platform: Platform,
	pub patchmode: Patchmode,
	pub supports_empty_pulses: bool,

	pub id: Keycode,
	pub username: String,
	pub id_and_username: String,
	pub online: bool,
	pub hidden: bool,
	pub lobby: Option<String>,

	last_receive_time: time::Instant,
	last_queue_time: time::Instant,
	ping_send_time: Option<time::Instant>,
	last_known_ping: time::Duration,
	ping_tolerance: time::Duration,

	stopped_receiving: bool,
	killed: bool,
}

const MAX_SENDQUEUE_SIZE: usize = 10000;

impl ServerClient
{
	pub fn create(
		stream: io::Result<net::TcpStream>,
		serial: u64,
	) -> io::Result<ServerClient>
	{
		let stream = stream?;

		println!("Incoming connection: {:?}", stream.peer_addr()?);

		stream.set_nonblocking(true)?;

		let key: u16 = rand::random();
		let id = keycode(key, serial);

		Ok(ServerClient {
			stream: stream,
			active_receive_length: None,
			chunk_incoming: false,
			already_sent_amount: 0,
			sendqueue: VecDeque::new(),
			version: Version::undefined(),
			platform: Platform::Unknown,
			patchmode: Patchmode::None,
			supports_empty_pulses: false,
			id: id,
			username: "".to_string(),
			id_and_username: format!("{}", id),
			online: false,
			hidden: false,
			lobby: None,
			last_receive_time: time::Instant::now(),
			last_queue_time: time::Instant::now(),
			ping_send_time: None,
			last_known_ping: time::Duration::from_secs(0),
			// The client should reset the connection after 71 seconds of
			// no contact with the server. Therefore, a 2-minute tolerance
			// seems reasonable.
			ping_tolerance: time::Duration::from_secs(120),
			stopped_receiving: false,
			killed: false,
		})
	}

	pub fn receiving(&self) -> bool
	{
		!self.killed && !self.stopped_receiving
	}

	pub fn stop_receiving(&mut self)
	{
		self.stopped_receiving = true;
	}

	pub fn kill(&mut self)
	{
		self.killed = true;
	}

	pub fn dead(&self) -> bool
	{
		self.killed || (self.stopped_receiving && !self.has_queued())
	}

	pub fn unversioned(&self) -> bool
	{
		self.version == Version::undefined()
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

				length = u32::from_le_bytes(lengthbuffer);
				self.active_receive_length = Some(length);
			}
		}

		if length == 0
		{
			println!("Received pulse.");
			self.active_receive_length = None;

			// An empty message (i.e. without a body) is a pulse message.
			// We just received something, thus the client is not silent.
			self.last_receive_time = time::Instant::now();
			return Ok(Message::Pulse);
		}
		else if self.unversioned()
			&& length as usize >= MESSAGE_SIZE_UNVERSIONED_LIMIT
		{
			println!(
				"Unversioned client tried to send very large message of length \
				 {}, which is more than MESSAGE_SIZE_UNVERSIONED_LIMIT",
				length
			);
			return Err(io::Error::new(
				io::ErrorKind::InvalidInput,
				"Message too large".to_string(),
			));
		}
		else if length as usize >= MESSAGE_SIZE_LIMIT
		{
			println!(
				"Refusing to receive very large message of length \
				 {}, which is more than MESSAGE_SIZE_LIMIT",
				length
			);
			return Err(io::Error::new(
				io::ErrorKind::InvalidInput,
				"Message too large".to_string(),
			));
		}
		else if length as usize >= MESSAGE_SIZE_WARNING_LIMIT
		{
			println!("Receiving very large message of length {}", length);
		}

		println!("Receiving message of length {}...", length);

		let mut buffer = vec![0; length as usize];
		self.stream.read_exact(&mut buffer)?;
		self.active_receive_length = None;

		println!("Received message of length {}.", length);

		// We just received something, thus the client is not silent.
		self.last_receive_time = time::Instant::now();

		// TODO if download
		if self.chunk_incoming
		{
			panic!("Not implemented yet.");
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

	fn pulse(&mut self)
	{
		println!("Queuing pulse...");

		let zeroes = [0u8; 4];
		self.sendqueue.push_back(zeroes.to_vec());

		println!("Queued pulse.");

		// After a couple of seconds of silence (i.e. not sending any message)
		// we send a pulse message so the client knows we are still breathing.
		// We are now actively sending, thus not silent.
		self.last_queue_time = time::Instant::now();
	}

	pub fn ping(&mut self)
	{
		self.ping_send_time = Some(time::Instant::now());
		self.send(Message::Ping);
	}

	pub fn send(&mut self, message: Message)
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

		// TODO compression

		let length = jsonstr.len() as u32;

		if length as usize >= MESSAGE_SIZE_WARNING_LIMIT
		{
			println!("Queueing very large message of length {}", length);
		}

		println!("Queueing message of length {}...", length);

		if self.sendqueue.len() >= MAX_SENDQUEUE_SIZE
		{
			eprintln!("Send buffer overflow");
			eprintln!("Force killing client {}", self.id);
			self.kill();
		}

		let lengthbuffer = length.to_le_bytes();
		self.sendqueue.push_back(lengthbuffer.to_vec());

		let data = jsonstr.as_bytes();
		self.sendqueue.push_back(data.to_vec());

		println!("Queued message of length {}.", length);

		if length < 200
		{
			println!("Queued message: {}", jsonstr);
		}

		// After a couple of seconds of silence (i.e. not sending any message)
		// we send a pulse message so the client knows we are still breathing.
		// We are now actively sending, thus not silent.
		self.last_queue_time = time::Instant::now();
	}

	pub fn has_queued(&self) -> bool
	{
		!self.killed && !self.sendqueue.is_empty()
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

	pub fn check_vitals(&mut self)
	{
		// If the client does not respond to a ping within
		// their ping tolerance (by default 2 minutes)...
		match self.ping_send_time
		{
			Some(time) =>
			{
				if time.elapsed() > self.ping_tolerance
				{
					println!("Disconnecting inactive client");

					self.kill();

					// TODO ghostbusters, but how?
				}
			}
			None =>
			{
				if self.last_receive_time.elapsed()
					> time::Duration::from_secs(5)
				{
					self.ping();
				}
			}
		}

		if self.supports_empty_pulses
			&& self.last_queue_time.elapsed() > time::Duration::from_secs(1)
		{
			self.pulse();
		}
	}

	pub fn handle_pong(&mut self)
	{
		match self.ping_send_time
		{
			Some(time) =>
			{
				println!("Client has {}ms ping.", time.elapsed().as_millis());

				self.last_known_ping = time.elapsed();
				self.ping_send_time = None;
			}
			None =>
			{}
		}
	}

	pub fn fulfil_request(&mut self, filename: String)
	{
		if !is_requestable(path::Path::new(&filename))
		{
			self.send(Message::RequestDenied {
				content: filename,
				metadata: DenyMetadata {
					reason: "File not requestable.".to_string(),
				},
			});
			return;
		}

		// TODO gzipped downloads

		match self.send_file(&filename)
		{
			Ok(()) =>
			{
				// TODO checksum

				// TODO requestfulfilled w/ metadata
			}
			Err(e) =>
			{
				// Server error, leave the request unanswered.
				// TODO should we though?
				eprintln!("Error while fulfilling request: {:?}", e);
			}
		}
	}

	fn send_file(&mut self, filename: &str) -> io::Result<()>
	{
		println!("Buffering file '{}' to client {}...", filename, self.id);

		let metadata = fs::metadata(filename)?;
		let filesize = metadata.len() as usize;

		if filesize >= SEND_FILE_SIZE_LIMIT
		{
			eprintln!(
				"Cannot send file of size {}, \
				 which is larger than SEND_FILE_SIZE_LIMIT.",
				filesize
			);
		}
		else if filesize >= SEND_FILE_SIZE_WARNING_LIMIT
		{
			println!("Sending very large file of size {}...", filesize);
		}

		// This is used clientside to generate the sourcefilename.
		// TODO compression
		let compressed = false;

		// This is unused since 0.32.0 but needed for earlier versions.
		// TODO implement is_file_executable?
		let executable = false;

		let mut file = File::open(filename)?;
		let mut offset = 0;

		while offset + SEND_FILE_CHUNK_SIZE < filesize
		{
			let mut buffer = vec![0u8; SEND_FILE_CHUNK_SIZE];
			file.read_exact(&mut buffer)?;

			// This is just for aesthetics.
			let progressmask = ((0xFFFF * offset) / filesize) as u16;

			let message = Message::Download {
				content: filename.to_string(),
				metadata: DownloadMetadata {
					offset: offset,
					signature: None,
					compressed: compressed,
					executable: (offset == 0 && executable),
					symbolic: false,
					progressmask: Some(progressmask),
				},
			};
			self.send_chunk(message, buffer);

			offset += SEND_FILE_CHUNK_SIZE;
		}

		{
			let mut buffer: Vec<u8> = Vec::new();
			file.read_to_end(&mut buffer)?;

			let message = Message::Download {
				content: filename.to_string(),
				metadata: DownloadMetadata {
					offset: offset,
					signature: None,
					compressed: compressed,
					executable: false,
					symbolic: false,
					progressmask: None,
				},
			};
			self.send_chunk(message, buffer);
		}

		println!("Buffered file '{}' to client {}.", filename, self.id);

		Ok(())
	}

	fn send_chunk(&mut self, message: Message, buffer: Vec<u8>)
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
		else if jsonstr.len() >= MESSAGE_SIZE_WARNING_LIMIT
		{
			println!(
				"Queueing very large message of length {}...",
				jsonstr.len()
			);
		}
		else if buffer.len() >= MESSAGE_SIZE_LIMIT
		{
			panic!(
				"Cannot send chunk of size {}, \
				 which is larger than MESSAGE_SIZE_LIMIT.",
				buffer.len()
			);
		}
		else if buffer.len() > SEND_FILE_CHUNK_SIZE
		{
			println!(
				"Queueing chunk of size {} \
				 which is larger than SEND_FILE_CHUNK_SIZE.",
				buffer.len()
			);
		}

		let length = jsonstr.len() as u32;
		let size = buffer.len() as u32;

		println!("Queueing message of length {}...", length);
		println!("And queueing chunk of size {}...", size);

		if self.sendqueue.len() >= MAX_SENDQUEUE_SIZE
		{
			eprintln!("Send buffer overflow");
			eprintln!("Force killing client {}", self.id);
			self.kill();
		}

		let lengthbuffer = length.to_le_bytes();
		self.sendqueue.push_back(lengthbuffer.to_vec());

		let data = jsonstr.as_bytes();
		self.sendqueue.push_back(data.to_vec());

		let sizebuffer = size.to_le_bytes();
		self.sendqueue.push_back(sizebuffer.to_vec());

		self.sendqueue.push_back(buffer);

		println!("Queued message of length {}.", length);
		println!("And queued chunk.");

		if length < 200
		{
			println!("Queued message: {}", jsonstr);
		}

		// After a couple of seconds of silence (i.e. not sending any message)
		// we send a pulse message so the client knows we are still breathing.
		// We are now actively sending, thus not silent.
		self.last_queue_time = time::Instant::now();
	}
}
