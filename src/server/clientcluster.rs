/* ClientCluster */

use server::message::*;
use server::serverclient::*;

use std::io;
use vec_drain_where::VecDrainWhereExt;

pub struct ClientCluster
{
	clients: Vec<ServerClient>,

	pub outgoing_clients: Vec<ServerClient>,
	pub incoming_clients: Vec<ServerClient>,

	closing: bool,
}

impl ClientCluster
{
	pub fn create() -> io::Result<ClientCluster>
	{
		Ok(ClientCluster {
			clients: Vec::new(),
			outgoing_clients: Vec::new(),
			incoming_clients: Vec::new(),
			closing: false,
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
								// TODO init client
								client.send(Message::Init);
							}
							Message::Chat { .. } =>
							{
								// TODO chat
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

		{
			let mut drained: Vec<ServerClient> = self
				.clients
				.e_drain_where(|client| !client.online)
				.collect();
			for client in &mut drained
			{
				client.send(Message::LeaveServer { content: None });
			}

			{
				self.outgoing_clients = drained;
			}
		}

		{
			let mut added: Vec<ServerClient>;
			{
				added = self.incoming_clients.drain(..).collect();
			}
			added.retain(|client| !client.dead());
			self.clients.append(&mut added);
		}
	}
}
