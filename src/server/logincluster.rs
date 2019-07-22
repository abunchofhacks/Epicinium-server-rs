/* LoginCluster */

use server::serverclient::*;

use std::io;
use std::net;

pub struct LoginCluster
{
	closing: bool,

	clients: Vec<ServerClient>,

	listener: net::TcpListener,
}

impl LoginCluster
{
	pub fn create() -> io::Result<LoginCluster>
	{
		let listener = net::TcpListener::bind("127.0.0.1:9999")?;
		listener.set_nonblocking(true)?;

		Ok(LoginCluster {
			closing: false,
			clients: Vec::new(),
			listener: listener,
		})
	}

	pub fn close(&mut self)
	{
		self.closing = true;
	}

	pub fn closed(&self) -> bool
	{
		self.closing && self.clients.is_empty()
	}

	pub fn update(&mut self)
	{
		for stream in self.listener.incoming()
		{
			match ServerClient::create(stream)
			{
				Ok(client) =>
				{
					self.clients.push(client);
				}
				Err(ref e) if e.kind() == io::ErrorKind::WouldBlock =>
				{
					// There are no more incoming connections.
					break;
				}
				Err(e) =>
				{
					eprintln!("Incoming connection failed: {}", e);
					break;
				}
			}
		}

		for client in &mut self.clients
		{
			// TODO replace loop with for loop to avoid client from DOSing?
			loop
			{
				match client.receive()
				{
					Ok(message) =>
					{
						println!("Received message: {:?}", message);
					}
					Err(ref e) if e.kind() == io::ErrorKind::WouldBlock =>
					{
						// There are no more incoming messages from this client.
						break;
					}
					Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof =>
					{
						// The client has disconnected.
						println!("Client disconnected.");
						client.killed = true;
						break;
					}
					Err(e) =>
					{
						eprintln!("Client connection failed: {:?}", e);
						client.killed = true;
						break;
					}
				}
			}
		}

		self.clients.retain(|client| !client.killed);
	}
}
