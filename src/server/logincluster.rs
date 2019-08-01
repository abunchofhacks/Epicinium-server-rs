/* LoginCluster */

use common::keycode::*;
use common::version::*;
use server::message::*;
use server::serverclient::*;

use std::fs;
use std::fs::File;
use std::io;
use std::io::Read;
use std::net;
use std::sync;
use std::sync::atomic;
use vec_drain_where::VecDrainWhereExt;

pub struct LoginCluster
{
	clients: Vec<ServerClient>,

	outgoing_clients: sync::mpsc::Sender<ServerClient>,
	incoming_clients: sync::mpsc::Receiver<ServerClient>,
	close_dependency: sync::Arc<atomic::AtomicBool>,

	listener: net::TcpListener,
	privatekey: openssl::pkey::PKey<openssl::pkey::Private>,

	ticker: u64,
	welcome_party: WelcomeParty,
	closing: bool,
}

impl LoginCluster
{
	pub fn create(
		outgoing: sync::mpsc::Sender<ServerClient>,
		incoming: sync::mpsc::Receiver<ServerClient>,
		close_dep: sync::Arc<atomic::AtomicBool>,
	) -> io::Result<LoginCluster>
	{
		let listener = net::TcpListener::bind("127.0.0.1:9999")?;
		listener.set_nonblocking(true)?;

		let mut pem: Vec<u8> = Vec::new();
		let mut file = File::open("keys/dummy_private.pem")?;
		file.read_to_end(&mut pem)?;
		let privatekey = openssl::pkey::PKey::private_key_from_pem(&pem)?;

		Ok(LoginCluster {
			clients: Vec::new(),
			outgoing_clients: outgoing,
			incoming_clients: incoming,
			close_dependency: close_dep,
			listener: listener,
			privatekey: privatekey,
			ticker: rand::random(),
			welcome_party: WelcomeParty { closing: false },
			closing: false,
		})
	}

	pub fn close(&mut self)
	{
		self.closing = true;
		self.welcome_party.closing = true;

		for client in &mut self.clients
		{
			client.send(Message::Closing);
		}
	}

	pub fn close_and_kick(&mut self)
	{
		if !self.closing
		{
			self.close();
		}

		for client in &mut self.clients
		{
			client.send(Message::Quit);
			client.stop_receiving();
		}
	}

	pub fn close_and_terminate(&mut self)
	{
		if !self.closing
		{
			self.close();
		}

		for client in &mut self.clients
		{
			client.send(Message::Quit);
			client.kill();
		}
	}

	pub fn closed(&self) -> bool
	{
		self.closing
			&& self.clients.is_empty()
			&& self.close_dependency.load(atomic::Ordering::Relaxed)
	}

	pub fn update(&mut self)
	{
		let mut requests: Vec<(Keycode, String)> = Vec::new();

		for stream in self.listener.incoming()
		{
			match ServerClient::create(stream, self.ticker)
			{
				Ok(client) =>
				{
					self.clients.push(client);
					self.ticker += 1;
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
							Message::Request { .. }
							| Message::JoinServer { .. }
								if client.unversioned() =>
							{
								println!(
									"Invalid message from unversioned \
									 client: {:?}",
									message
								);
								client.kill();
							}
							Message::Request { .. }
								if client.platform == Platform::Unknown =>
							{
								println!(
									"Invalid message from client without \
									 platform: {:?}",
									message
								);
								client.kill();
							}
							Message::Request { content: name } =>
							{
								requests.push((client.id, name));
							}
							Message::JoinServer { .. } if self.closing =>
							{
								client.send(Message::Closing);
							}
							Message::JoinServer {
								status: None,
								content: Some(_),
								sender: Some(account_id),
								metadata: _,
							} =>
							{
								let curver = Version::current();
								if client.version.major != curver.major
									|| client.version.minor != curver.minor
								{
									// Why is this LEAVE_SERVER {} and not
									// JOIN_SERVER {}? Maybe it has something
									// to do with MainMenu. Well, let's leave
									// it until we do proper error handling.
									// TODO #962
									client.send(Message::LeaveServer {
										content: None,
									})
								}
								else
								{
									join_server(client, account_id);

									client.online = true;

									// Stop receiving until we move this client
									// from our list to somewhere else.
									break;
								}
							}
							Message::JoinServer { .. } =>
							{
								println!(
									"Invalid message from client: {:?}",
									message
								);
								client.kill();
							}
							Message::LeaveServer { .. } =>
							{
								println!(
									"Ignoring message from client: {:?}",
									message
								);
							}
							Message::Init | Message::Chat { .. } =>
							{
								println!(
									"Invalid message from offline client: {:?}",
									message
								);
								client.kill();
							}
							Message::Closing
							| Message::Stamp { .. }
							| Message::Download { .. }
							| Message::RequestDenied { .. }
							| Message::RequestFulfilled { .. } =>
							{
								println!(
									"Invalid message from client: {:?}",
									message
								);
								client.kill();
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

		for (cid, name) in requests
		{
			for client in &mut self.clients
			{
				if client.id == cid && !client.dead()
				{
					client.fulfil_request(name, &self.privatekey);
					break;
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

		let mut stranded = Vec::<ServerClient>::new();
		for client in self.clients.e_drain_where(|x| x.online)
		{
			match self.outgoing_clients.send(client)
			{
				Ok(_) =>
				{}
				Err(sync::mpsc::SendError(mut client)) =>
				{
					if self.closing
					{
						client.send(Message::Closing);
					}
					stranded.push(client);
				}
			}
		}
		if !stranded.is_empty()
		{
			stranded.retain(|client| !client.dead());
			self.clients.append(&mut stranded);
		}

		loop
		{
			match self.incoming_clients.try_recv()
			{
				Ok(client) =>
				{
					if !client.dead()
					{
						self.clients.push(client);
					}
				}
				Err(_) =>
				{
					// There are no more incoming clients at the moment.
					break;
				}
			}
		}
	}
}

fn join_server(client: &mut ServerClient, account_id: String)
{
	match account_id.parse::<u8>()
	{
		Ok(x) if x > 0 && x <= 8 =>
		{
			const NAMES: [&str; 8] = [
				"Alice", "Bob", "Carol", "Dave", "Emma", "Frank", "Gwen",
				"Harold",
			];
			client.username = NAMES[(x - 1) as usize].to_string();
		}
		_ =>
		{
			client.username = format!("{}", client.id);
		}
	}
	client.id_and_username = format!("{} '{}'", client.id, client.username);
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
			| Message::JoinServer { .. }
			| Message::LeaveServer { .. }
			| Message::Init
			| Message::Chat { .. }
			| Message::Stamp { .. }
			| Message::Download { .. }
			| Message::Request { .. }
			| Message::RequestDenied { .. }
			| Message::RequestFulfilled { .. }
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
				sender: Some("server".to_string()),
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

		match load_notice()
		{
			Some(notice) =>
			{
				client.send(Message::Stamp { metadata: notice });
			}
			None =>
			{}
		}

		if client.version >= Version::exact(0, 31, 1, 0)
		{
			client.supports_empty_pulses = true;
		}

		if client.version >= Version::exact(0, 32, 0, 0)
		{
			// TODO enable compression
		}

		if client.version >= Version::exact(0, 31, 1, 0)
		{
			match client.platform
			{
				Platform::Unknown
				| Platform::Windows32
				| Platform::Windows64 =>
				{}
				Platform::Osx32
				| Platform::Osx64
				| Platform::Debian32
				| Platform::Debian64 =>
				{
					// TODO client.supports_constructed_symlinks = true;
				}
			}
		}

		if client.version >= Version::exact(0, 31, 1, 0)
		{
			// TODO client.supports_gzipped_downloads = true;
		}

		if client.version >= Version::exact(0, 31, 1, 0)
		{
			// TODO client.supports_manifest_files = true;
		}

		// Send a ping message, just to get an estimated ping.
		client.ping();

		// TODO mention patches
	}
}

fn load_notice() -> Option<StampMetadata>
{
	match fs::read_to_string("server-notice.json")
	{
		Ok(raw) => match serde_json::from_str::<StampMetadata>(&raw)
		{
			Ok(value) => Some(value),
			Err(e) =>
			{
				eprintln!("Notice file could not be loaded: {:?}", e);
				None
			}
		},
		Err(e) =>
		{
			eprintln!("Notice file could not be loaded: {:?}", e);
			None
		}
	}
}
