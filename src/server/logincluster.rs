/* LoginCluster */

use common::version::*;
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
			while client.receiving()
			{
				match client.receive()
				{
					Ok(message) =>
					{
						LoginCluster::handle_message(
							client,
							message,
							self.closing,
						);
					}
					Err(ref e) if e.kind() == io::ErrorKind::WouldBlock =>
					{
						// There are no more incoming messages from this client.
						break;
					}
					Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof =>
					{
						// The client has disconnected.
						println!("Client ungracefully disconnected.");
						client.stop_receiving();
						client.stop_sending();
					}
					Err(e) =>
					{
						eprintln!("Client connection failed: {:?}", e);
						client.stop_receiving();
						client.stop_sending();
					}
				}
			}
		}

		for client in &mut self.clients
		{
			while client.has_queued()
			{
				match client.send_queued()
				{
					Ok(()) =>
					{}
					Err(ref e) if e.kind() == io::ErrorKind::WouldBlock =>
					{
						// The TCP buffers are blocked up for this client.
						break;
					}
					Err(ref e) if e.kind() == io::ErrorKind::UnexpectedEof =>
					{
						// The client has disconnected.
						println!("Client ungracefully disconnected.");
						client.stop_receiving();
						client.stop_sending();
					}
					Err(e) =>
					{
						eprintln!("Client connection failed: {:?}", e);
						client.stop_receiving();
						client.stop_sending();
					}
				}
			}
		}

		self.clients.retain(|client| !client.dead());
	}

	fn handle_message(
		client: &mut ServerClient,
		message: Message,
		closing: bool,
	)
	{
		match message
		{
			Message::Pulse =>
			{
				// TODO handle
			}
			Message::Ping =>
			{
				// Pings must always be responded with pongs.
				client.send(Message::Pong);
			}
			Message::Pong =>
			{
				// TODO handle
			}
			Message::Version { version, metadata } =>
			{
				client.version = version;
				println!("Client has version {}", version.to_string());

				match metadata
				{
					Some(PlatformMetadata {
						platform,
						patchmode,
					}) =>
					{
						client.platform = platform;
						println!("Client has platform {:?}", platform);
						client.patchmode = patchmode;
						println!("Client has patchmode {:?}", patchmode);
					}
					None =>
					{}
				}

				LoginCluster::welcome_client(client, closing);
			}
			Message::Quit =>
			{
				println!("Client gracefully disconnected.");
				client.stop_receiving();
			}
			Message::Closing =>
			{
				println!("Invalid message from client: {:?}", message);
				client.stop_receiving();
			}
			Message::Chat {
				content: _,
				sender: _,
				target: _,
			} =>
			{
				// TODO handle
			}
		}
	}

	fn welcome_client(client: &mut ServerClient, closing: bool)
	{
		let myversion = Version::current();
		client.send(Message::Version {
			version: myversion,
			metadata: None,
		});

		if client.version.major != myversion.major
			|| client.version == Version::undefined()
		{
			client.stop_receiving();
			return;
		}
		else if (client.patchmode == Patchmode::Itchio
			|| client.patchmode == Patchmode::Gamejolt)
			&& client.version < Version::exact(0, 29, 0, 0)
		{
			// Version 0.29.0 was the first closed beta
			// version, which means clients with non-server
			// patchmodes (itch or gamejolt) cannot patch.
			// It is also the first version with keys.
			// Older versions do not properly display the
			// warning that joining failed because of
			// ResponseStatus::KEY_REQUIRED. Instead, we
			// overwrite the 'Version mismatch' message.
			client.send(Message::Chat {
				content: "The Open Beta has ended. \
				          Join our Discord community at \
				          www.epicinium.nl/discord \
				          to qualify for access to the \
				          Closed Beta."
					.to_string(),
				sender: "server".to_string(),
				target: ChatTarget::General,
			});

			client.stop_receiving();
			return;
		}
		else if closing
		{
			client.send(Message::Closing);

			client.stop_receiving();
			return;
		}
		else
		{
			// TODO notice
		}

		// TODO change state to VERSIONED

		// TODO enable pulses enzo
	}
}
