/* LoginCluster */

use common::version::*;
use server::message::*;
use server::serverclient::*;

use std::io;
use std::net;

pub struct LoginCluster
{
	clients: Vec<ServerClient>,

	listener: net::TcpListener,

	welcome_party: WelcomeParty,
	closing: bool,
}

impl LoginCluster
{
	pub fn create() -> io::Result<LoginCluster>
	{
		let listener = net::TcpListener::bind("127.0.0.1:9999")?;
		listener.set_nonblocking(true)?;

		Ok(LoginCluster {
			clients: Vec::new(),
			listener: listener,
			welcome_party: WelcomeParty { closing: false },
			closing: false,
		})
	}

	pub fn close(&mut self)
	{
		self.closing = true;
		self.welcome_party.closing = true;
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
						match message
						{
							Message::Pulse =>
							{
								// The client just let us know that it is
								// still breathing.
							}
							Message::Ping =>
							{
								// Pings must always be responded with pongs.
								client.send(Message::Pong);
							}
							Message::Pong =>
							{
								// Remember the ping time.
								client.handle_pong();
							}
							Message::Version { .. } =>
							{
								self.welcome_party.handle(client, message);
							}
							Message::Quit =>
							{
								println!("Client gracefully disconnected.");
								client.stop_receiving();
							}
							Message::Closing =>
							{
								println!(
									"Invalid message from client: {:?}",
									message
								);
								client.stop_receiving();
							}
							Message::Chat { .. } =>
							{
								// TODO handle
							}
						}
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
						client.kill();
					}
					Err(e) =>
					{
						eprintln!("Client connection failed: {:?}", e);
						client.kill();
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
						client.kill();
					}
					Err(e) =>
					{
						eprintln!("Client connection failed: {:?}", e);
						client.kill();
					}
				}
			}
		}

		for client in &mut self.clients
		{
			if !client.dead()
			{
				client.check_vitals();
			}
		}

		self.clients.retain(|client| !client.dead());
	}
}

pub struct WelcomeParty
{
	pub closing: bool,
}

impl WelcomeParty
{
	pub fn handle(&mut self, client: &mut ServerClient, message: Message)
	{
		match message
		{
			Message::Version {
				version,
				metadata:
					Some(PlatformMetadata {
						platform,
						patchmode,
					}),
			} =>
			{
				client.version = version;
				println!("Client has version {}", version.to_string());

				client.platform = platform;
				println!("Client has platform {:?}", platform);
				client.patchmode = patchmode;
				println!("Client has patchmode {:?}", patchmode);

				self.greet(client);
			}

			Message::Version {
				version,
				metadata: None,
			} =>
			{
				client.version = version;
				println!("Client has version {}", version.to_string());

				self.greet(client);
			}

			Message::Pulse
			| Message::Ping
			| Message::Pong
			| Message::Chat { .. }
			| Message::Closing
			| Message::Quit =>
			{
				panic!("Message misrouted");
			}
		}
	}

	fn greet(&mut self, client: &mut ServerClient)
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
		else if self.closing
		{
			client.send(Message::Closing);

			client.stop_receiving();
			return;
		}

		// TODO load notice

		// TODO change state to VERSIONED

		// TODO enable pulses enzo

		// Send a ping message, just to get an estimated ping.
		client.ping();
	}
}
