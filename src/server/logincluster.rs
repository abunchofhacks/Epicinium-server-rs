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
			match stream
			{
				Ok(stream) =>
				{
					self.clients.push(ServerClient::create(stream));
				}
				Err(ref e) if e.kind() == io::ErrorKind::WouldBlock =>
				{
					// There are no more incoming connections.
					break;
				}
				Err(e) =>
				{
					eprintln!("Incoming connection failed: {}", e);
				}
			}
		}
	}
}
