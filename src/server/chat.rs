/* Server::Chat */

use common::keycode::*;
use server::message::*;

use std::collections::HashMap;

use futures::future;
use futures::future::Either;
use futures::future::Future;
use futures::stream::Stream;

use tokio::sync::mpsc;

use enumset::*;

pub fn start_chat_task(
	messages: mpsc::Receiver<Message>,
) -> impl Future<Item = (), Error = ()> + Send
{
	let mut clients: HashMap<Keycode, Client> = HashMap::new();

	messages
		.map_err(|error| eprintln!("Recv error in chat_task: {:?}", error))
		.for_each(move |message| handle_message(message, &mut clients))
}

fn handle_message(
	message: Message,
	clients: &mut HashMap<Keycode, Client>,
) -> impl Future<Item = (), Error = ()> + Send
{
	match message
	{
		Message::InitInternal { client_id } =>
		{
			match clients.remove(&client_id)
			{
				Some(mut found_client) =>
				{
					init_client(&mut found_client, clients);
					if !found_client.dead
					{
						clients.insert(client_id, found_client);
					}
				}
				None => eprintln!("Client {} not found", client_id),
			}
		}
		Message::JoiningServerInternal {
			client_id,
			username,
			unlocks,
			sendbuffer,
		} =>
		{
			return Either::A(joined_server(
				client_id, username, unlocks, sendbuffer, clients,
			));
		}
		Message::LeaveServerInternal { client_id } =>
		{
			match clients.remove(&client_id)
			{
				Some(removed_client) => leaving_server(removed_client, clients),
				None => eprintln!("Client {} not found", client_id),
			}
		}

		Message::Chat { .. } =>
		{
			for client in clients.values_mut()
			{
				client.send(message.clone());
			}
			clients.retain(|_id, client| !client.dead);
		}

		Message::Pulse
		| Message::Ping
		| Message::Pong
		| Message::Version { .. }
		| Message::JoinServer { .. }
		| Message::LeaveServer { .. }
		| Message::Init
		| Message::Closing
		| Message::Quit
		| Message::Stamp { .. }
		| Message::Download { .. }
		| Message::Request { .. }
		| Message::RequestDenied { .. }
		| Message::RequestFulfilled { .. }
		| Message::JoinedServerInternal { .. } =>
		{
			panic!("Misrouted message in chat_task: {:?}", message);
		}
	}

	Either::B(future::ok(()))
}

struct Client
{
	username: String,
	join_metadata: Option<JoinMetadata>,
	sendbuffer: mpsc::Sender<Message>,
	hidden: bool,
	dead: bool,
}

impl Client
{
	fn send(&mut self, message: Message)
	{
		match self.sendbuffer.try_send(message.clone())
		{
			Ok(()) => (),
			Err(_error) => self.dead = true,
		}
	}
}

fn joined_server(
	id: Keycode,
	username: String,
	unlocks: EnumSet<Unlock>,
	sendbuffer: mpsc::Sender<Message>,
	clients: &mut HashMap<Keycode, Client>,
) -> impl Future<Item = (), Error = ()> + Send
{
	// TODO ghostbusting

	let join_metadata = generate_join_metadata(&unlocks);
	let hidden = username.starts_with("#");

	let mut newcomer = Client {
		username,
		join_metadata,
		sendbuffer,
		hidden: hidden,
		dead: false,
	};

	// Confirm to the newcomer that they have joined.
	let message = Message::JoinServer {
		status: None,
		content: Some(newcomer.username.clone()),
		sender: None,
		metadata: newcomer.join_metadata,
	};
	newcomer.send(message.clone());

	// Tell the newcomer that they are online.
	// TODO this is weird (#)
	newcomer.send(message.clone());

	// Tell everyone who the newcomer is.
	if !newcomer.hidden
	{
		for other in clients.values_mut()
		{
			other.send(message.clone());
		}

		// Tell everyone the rating and stars of the newcomer.
		// TODO rating and stars
	}

	// Let the client know which lobbies there are.
	// TODO lobbies

	// Let the client know who else is online.
	for other in clients.values()
	{
		if !other.hidden
		{
			newcomer.send(Message::JoinServer {
				status: None,
				content: Some(other.username.clone()),
				sender: None,
				metadata: other.join_metadata,
			});

			// TODO rating
			// TODO stars
			// TODO join_lobby
			// TODO in_game
		}
	}

	// Let the client know we are done initializing.
	newcomer.send(Message::Init);

	// Show them a welcome message, if any.
	welcome_client(&mut newcomer);

	clients.retain(|_id, client| !client.dead);
	if !newcomer.dead
	{
		clients.insert(id, newcomer);
	}

	future::ok(())
}

fn welcome_client(_client: &mut Client)
{
	// No welcome message at the moment.
}

fn generate_join_metadata(unlocks: &EnumSet<Unlock>) -> Option<JoinMetadata>
{
	let mut metadata: JoinMetadata = Default::default();
	if unlocks.contains(Unlock::Dev)
	{
		metadata.dev = true;
	}
	if unlocks.contains(Unlock::Guest)
	{
		metadata.guest = true;
	}

	if metadata == Default::default()
	{
		None
	}
	else
	{
		Some(metadata)
	}
}

fn init_client(
	found_client: &mut Client,
	clients: &mut HashMap<Keycode, Client>,
)
{
	// Let the client know which lobbies there are.
	// TODO lobbies

	// Tell the client that they are online.
	found_client.send(Message::JoinServer {
		status: None,
		content: Some(found_client.username.clone()),
		sender: None,
		metadata: found_client.join_metadata,
	});

	// TODO rating
	// TODO stars
	// TODO join_lobby
	// TODO in_game

	// Let the client know who else is online.
	for other in clients.values()
	{
		if !other.hidden
		{
			found_client.send(Message::JoinServer {
				status: None,
				content: Some(other.username.clone()),
				sender: None,
				metadata: other.join_metadata,
			});

			// TODO rating
			// TODO stars
			// TODO join_lobby
			// TODO in_game
		}
	}

	// Let the client know we are done initializing.
	found_client.send(Message::Init);

	clients.retain(|_id, client| !client.dead);
}

fn leaving_server(
	mut removed_client: Client,
	clients: &mut HashMap<Keycode, Client>,
)
{
	let message = Message::LeaveServer {
		content: Some(removed_client.username.clone()),
	};
	for client in clients.values_mut()
	{
		client.send(message.clone());
	}
	clients.retain(|_id, client| !client.dead);
	removed_client.send(message);
}
