/* ClientCluster */

use common::keycode::*;
use server::message::*;
use server::serverclient::*;

use std::fs::File;
use std::io;
use std::io::Read;
use std::sync;
use vec_drain_where::VecDrainWhereExt;

pub struct ClientCluster
{
	clients: Vec<ServerClient>,

	outgoing_clients: sync::mpsc::Sender<ServerClient>,
	incoming_clients: sync::mpsc::Receiver<ServerClient>,

	privatekey: openssl::rsa::Rsa<openssl::pkey::Private>,

	closing: bool,
}

impl ClientCluster
{
	pub fn create(
		incoming: sync::mpsc::Receiver<ServerClient>,
		outgoing: sync::mpsc::Sender<ServerClient>,
	) -> io::Result<ClientCluster>
	{
		let mut pem: Vec<u8> = Vec::new();
		let mut file = File::open("keys/private.pem")?;
		file.read_to_end(&mut pem)?;
		let privatekey = openssl::rsa::Rsa::private_key_from_pem(&pem)?;

		Ok(ClientCluster {
			clients: Vec::new(),
			outgoing_clients: outgoing,
			incoming_clients: incoming,
			privatekey: privatekey,
			closing: false,
		})
	}

	pub fn close(&mut self)
	{
		self.closing = true;

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
		self.closing && self.clients.is_empty()
	}

	pub fn update(&mut self)
	{
		let mut actions: Vec<(Keycode, Message)> = Vec::new();
		let mut requests: Vec<(Keycode, String)> = Vec::new();
		let mut broadcasts: Vec<(Message, Option<String>)> = Vec::new();

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
							Message::Quit =>
							{
								println!("Client gracefully disconnected.");
								client.stop_receiving();
							}
							Message::JoinServer { .. } =>
							{
								println!(
									"Ignoring message from client: {:?}",
									message
								);
							}
							Message::LeaveServer { .. } =>
							{
								client.send(Message::LeaveServer {
									content: Some(client.username.clone()),
								});
								broadcasts.push((
									Message::LeaveServer {
										content: Some(client.username.clone()),
									},
									None,
								));

								client.online = false;

								// Stop receiving until we move this client
								// from our list to somewhere else.
								break;
							}
							Message::Init =>
							{
								actions.push((client.id, message));
							}
							Message::Chat {
								content,
								sender: None,
								target: ChatTarget::General,
							} =>
							{
								println!(
									"Client {} sent chat message: {}",
									client.id_and_username, content
								);
								broadcasts.push((
									Message::Chat {
										content: content,
										sender: Some(client.username.clone()),
										target: ChatTarget::General,
									},
									None,
								));
							}
							Message::Chat {
								content,
								sender: None,
								target: ChatTarget::Lobby,
							} => match client.lobby
							{
								Some(ref lobbyid) =>
								{
									println!(
										"Client {} sent chat message to \
										 lobby {}: {}",
										client.id_and_username,
										lobbyid,
										content
									);
									broadcasts.push((
										Message::Chat {
											content: content,
											sender: Some(
												client.username.clone(),
											),
											target: ChatTarget::Lobby,
										},
										Some(lobbyid.clone()),
									));
								}
								None =>
								{
									println!(
										"Invalid lobby chat message from \
										 client not in a lobby"
									);
									client.kill();
								}
							},
							Message::Chat { .. } =>
							{
								println!(
									"Invalid message from client: {:?}",
									message
								);
								client.kill();
							}
							Message::Request { content: name } =>
							{
								requests.push((client.id, name));
							}
							Message::Version { .. } =>
							{
								println!(
									"Invalid message from online client: {:?}",
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

		for (cid, message) in actions
		{
			match message
			{
				Message::Init =>
				{
					// The client just finished a game; tell everyone their
					// rating and stars.
					match self.find_client(cid)
					{
						Some(client) =>
						{
							if !client.hidden
							{
								// TODO add rating and stars to broadcasts
							}
						}
						None =>
						{
							// If we cannot find the client, there is no need
							// to send the messages.
							continue;
						}
					}

					let mut messages: Vec<Message> = Vec::new();

					// Let the client know which lobbies there are.
					// TODO lobbies

					// Let the client know who else is online.
					for other in self.clients.iter().filter(|x| !x.hidden)
					{
						// TODO dev?
						// TODO guest?
						messages.push(Message::JoinServer {
							status: None,
							content: Some(other.username.clone()),
							sender: None,
							metadata: None,
						});
						// TODO rating
						// TODO stars
						// TODO join_lobby
						// TODO in_game
					}

					// Let the client know the rankings.
					// TODO rankings

					// Let the client know what the current challenge is called.
					// TODO challenge

					// Let the client know how many stars they have for the
					// current challenge.
					// TODO recent_stars

					// Let the client know we are done initializing.
					match self.find_client(cid)
					{
						Some(client) =>
						{
							for message in messages
							{
								client.send(message);
							}
							client.send(Message::Init);
						}
						None =>
						{}
					}
				}

				Message::Pulse
				| Message::Ping
				| Message::Pong
				| Message::Version { .. }
				| Message::JoinServer { .. }
				| Message::LeaveServer { .. }
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

		for x in broadcasts
		{
			match x
			{
				(message, None) =>
				{
					for client in &mut self.clients
					{
						if client.online
						{
							client.send(message.clone());
						}
					}
				}
				(message, lobbyid @ Some(_)) =>
				{
					for client in &mut self.clients
					{
						if client.lobby == lobbyid
						{
							client.send(message.clone());
						}
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

		for client in self.clients.e_drain_where(|x| !x.online)
		{
			match self.outgoing_clients.send(client)
			{
				Ok(_) =>
				{}
				Err(_) =>
				{
					panic!("The LoginCluster should outlast me.");
				}
			}
		}

		loop
		{
			match self.incoming_clients.try_recv()
			{
				Ok(mut client) =>
				{
					// Confirm to the newcomer that they have joined.
					client.send(Message::JoinServer {
						status: None,
						content: Some(client.username.clone()),
						sender: None,
						metadata: None,
					});

					// Tell the newcomer that they are online.
					// TODO this is weird
					client.send(Message::JoinServer {
						status: None,
						content: Some(client.username.clone()),
						sender: None,
						metadata: None,
					});

					// Tell everyone who the newcomer is.
					if !client.hidden
					{
						for otherclient in &mut self.clients
						{
							otherclient.send(Message::JoinServer {
								status: None,
								content: Some(client.username.clone()),
								sender: None,
								metadata: None,
							});
						}

						// Tell everyone the rating and stars of the newcomer.
						// TODO rating and stars
					}

					// Let the client know which lobbies there are.
					// TODO lobbies

					// Let the client know who else is online.
					for otherclient in &mut self.clients
					{
						if !otherclient.hidden
						{
							client.send(Message::JoinServer {
								status: None,
								content: Some(otherclient.username.clone()),
								sender: None,
								metadata: None,
							});

							// TODO rating
							// TODO stars
							// TODO join_lobby
							// TODO in_game
						}
					}

					// Let the client know the rankings.
					// TODO rankings

					// Let the client know what the current challenge is called.
					// TODO challenge

					// Let the client know how many stars they have for the
					// current challenge.
					// TODO recent_stars

					// Let the client know we are done initializing.
					client.send(Message::Init);

					// TODO join lobby if still in progress

					welcome_client(&mut client);

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

	fn find_client(&mut self, cid: Keycode) -> Option<&mut ServerClient>
	{
		for client in &mut self.clients
		{
			if client.id == cid
			{
				return Some(client);
			}
		}

		None
	}
}

fn welcome_client(_client: &mut ServerClient)
{
	// No welcome message at the moment.
}
