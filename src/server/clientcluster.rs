/* ClientCluster */

use server::message::*;
use server::serverclient::*;

use std::io;
use std::sync;
use vec_drain_where::VecDrainWhereExt;

pub struct ClientCluster
{
	clients: Vec<ServerClient>,

	outgoing_clients: sync::mpsc::Sender<ServerClient>,
	incoming_clients: sync::mpsc::Receiver<ServerClient>,

	broadcasts: Vec<(Message, Option<String>)>,

	closing: bool,
}

impl ClientCluster
{
	pub fn create(
		incoming: sync::mpsc::Receiver<ServerClient>,
		outgoing: sync::mpsc::Sender<ServerClient>,
	) -> io::Result<ClientCluster>
	{
		Ok(ClientCluster {
			clients: Vec::new(),
			outgoing_clients: outgoing,
			incoming_clients: incoming,
			broadcasts: Vec::new(),
			closing: false,
		})
	}

	pub fn close(&mut self)
	{
		if self.closing
		{
			return;
		}

		self.closing = true;

		for client in &mut self.clients
		{
			client.send(Message::Closing);
		}
	}

	pub fn closed(&self) -> bool
	{
		self.closing && self.clients.is_empty()
	}

	pub fn update(&mut self)
	{
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
							Message::LeaveServer { .. } =>
							{
								client.online = false;

								// Stop receiving until we move this client
								// from our list to somewhere else.
								break;
							}
							Message::Init =>
							{
								init_client(client);
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
								self.broadcasts.push((
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
									self.broadcasts.push((
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
							Message::Version { .. }
							| Message::JoinServer { .. } =>
							{
								println!(
									"Invalid message from online client: {:?}",
									message
								);
								client.kill();
							}
							Message::Closing =>
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

		for x in self.broadcasts.drain(..)
		{
			match x
			{
				(message, None) =>
				{
					for client in &mut self.clients
					{
						client.send(message.clone());
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

		for mut client in self.clients.e_drain_where(|x| !x.online)
		{
			client.send(Message::LeaveServer {
				content: Some(client.username.clone()),
			});
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
					joined_server(&mut client);
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

fn joined_server(client: &mut ServerClient)
{
	client.send(Message::JoinServer {
		status: None,
		content: Some(client.username.clone()),
		sender: None,
		metadata: None,
	});

	init_client(client);
}

fn init_client(client: &mut ServerClient)
{
	// TODO init client

	client.send(Message::Init);
}
