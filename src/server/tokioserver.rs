use server::settings::*;

use common::version::*;
use server::limits::*;
use server::message::*;

use std::error;
use std::io;
use std::net::SocketAddr;
use std::time;

use tokio::net::{TcpListener, TcpStream};
use tokio::prelude::*;

use futures::{Future, Stream};

pub fn run_server(settings: &Settings) -> Result<(), Box<dyn error::Error>>
{
	let server = match settings.server()
	{
		Some(x) => x,
		None =>
		{
			return Err(Box::new(io::Error::new(
				io::ErrorKind::InvalidInput,
				"No ip mask (setting 'server') defined.",
			)));
		}
	};
	let port = match settings.port()
	{
		Some(x) => x,
		None =>
		{
			return Err(Box::new(io::Error::new(
				io::ErrorKind::InvalidInput,
				"No port (setting 'port') defined.",
			)));
		}
	};
	let address: SocketAddr = format!("{}:{}", server, port).parse()?;
	let listener = TcpListener::bind(&address)?;

	println!("Listening on {}:{}", server, port);

	let server = listener
		.incoming()
		.for_each(|socket| {
			println!("Incoming connection: {:?}", socket);
			accept_client(socket)
		})
		.map_err(|e| {
			eprintln!("Incoming connection failed: {}", e);
		});

	// TODO

	tokio::run(server);

	Ok(())
}

struct ClientConnection
{
	stream: TcpStream,
	active_receive_length: Option<u32>,
	chunk_incoming: bool,

	versioned: bool,

	last_receive_time: time::Instant,
	last_queue_time: time::Instant,
	ping_send_time: Option<time::Instant>,
	last_known_ping: time::Duration,
	ping_tolerance: time::Duration,
}

impl ClientConnection
{
	fn new(stream: TcpStream) -> Self
	{
		ClientConnection {
			stream: stream,
			active_receive_length: None,
			chunk_incoming: false,

			versioned: false,

			last_receive_time: time::Instant::now(),
			last_queue_time: time::Instant::now(),
			ping_send_time: None,
			last_known_ping: time::Duration::from_secs(0),
			// The client should reset the connection after 71 seconds of
			// no contact with the server. Therefore, a 2-minute tolerance
			// seems reasonable.
			ping_tolerance: time::Duration::from_secs(120),
		}
	}

	fn receive(&mut self) -> io::Result<Message>
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
		else if self.versioned
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
}

impl Stream for ClientConnection
{
	type Item = Message;
	type Error = io::Error;

	fn poll(&mut self) -> Result<Async<Option<Self::Item>>, Self::Error>
	{
		match self.receive()
		{
			Ok(message) => Ok(Async::Ready(Some(message))),
			Err(ref e) if e.kind() == io::ErrorKind::WouldBlock =>
			{
				Ok(Async::NotReady)
			}
			Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof =>
			{
				// The client has disconnected.
				println!("Client ungracefully disconnected.");
				Ok(Async::Ready(None))
			}
			Err(e) => Err(e),
		}
	}
}

struct Client
{
	connection: ClientConnection,

	pub id: String,
	pub version: Version,
}

impl Client
{
	fn new(stream: TcpStream) -> io::Result<Self>
	{
		let id = format!("{}", stream.peer_addr()?);

		Ok(Client {
			connection: ClientConnection::new(stream),

			id: id,
			version: Version::undefined(),
		})
	}
}

impl Future for Client
{
	type Item = ();
	type Error = io::Error;

	fn poll(&mut self) -> Poll<(), io::Error>
	{
		while let Async::Ready(activity) = self.connection.poll()?
		{
			if let Some(message) = activity
			{
				// TODO handle
				println!("Message: {:?}", message);
			}
			else
			{
				// No more incoming messages.
				return Ok(Async::Ready(()));
			}
		}

		Ok(Async::NotReady)
	}
}

fn accept_client(socket: TcpStream) -> io::Result<()>
{
	let client = Client::new(socket)?;
	let task = client.map_err(move |e| eprintln!("Error in client: {}", e));

	tokio::spawn(task);

	Ok(())
}
