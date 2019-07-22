/* LoginCluster */

use server::message::Message::*;
use server::message::*;
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
			// TODO add counter to prevent one client DOSing us?
			while !client.killed
			{
				match client.receive()
				{
					Ok(message) => match message
					{
						Pulse =>
						{
							// TODO
						}
						Ping =>
						{
							// TODO write response
						}
						Pong =>
						{
							// TODO
						}
						Version {
							version,
							metadata:
								PlatformMetadata {
									platform,
									patchmode,
								},
						} =>
						{
							client.version = version;
							println!(
								"Client has version {}",
								version.to_string()
							);
							client.platform = platform;
							println!("Client has platform {:?}", platform);
							client.patchmode = patchmode;
							println!("Client has patchmode {:?}", patchmode);
						}
						Quit =>
						{
							println!("Client has gracefully disconnected.");
							client.killed = true;
						}
						Closing =>
						{
							println!(
								"Invalid message from client: {:?}",
								message
							);
							client.killed = true;
						}
					},
					Err(ref e) if e.kind() == io::ErrorKind::WouldBlock =>
					{
						// There are no more incoming messages from this client.
						break;
					}
					Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof =>
					{
						// The client has disconnected.
						if !client.killed
						{
							println!("Client has ungracefully disconnected.");
							client.killed = true;
						}
					}
					Err(e) =>
					{
						eprintln!("Client connection failed: {:?}", e);
						client.killed = true;
					}
				}
			}
		}

		self.clients.retain(|client| !client.killed);
	}
}
