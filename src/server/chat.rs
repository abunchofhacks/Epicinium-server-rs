/* Server::Chat */

use crate::common::keycode::*;
use crate::server::message::*;

use futures::future::Future;
use futures::stream::Stream;

use tokio::sync::mpsc;

use enumset::*;
use vec_drain_where::VecDrainWhereExt;

#[derive(Debug)]
pub enum Update
{
	Join
	{
		client_id: Keycode,
		username: String,
		unlocks: EnumSet<Unlock>,
		sendbuffer: mpsc::Sender<Message>,
	},
	Init
	{
		sendbuffer: mpsc::Sender<Message>,
	},
	Leave
	{
		client_id: Keycode,
	},

	Msg(Message),
}

pub fn start_task(
	updates: mpsc::Receiver<Update>,
) -> impl Future<Item = (), Error = ()> + Send
{
	let mut clients: Vec<Client> = Vec::new();

	updates
		.map_err(|error| eprintln!("Recv error in general chat: {:?}", error))
		.for_each(move |update| {
			handle_update(update, &mut clients);
			Ok(())
		})
}

fn handle_update(update: Update, clients: &mut Vec<Client>)
{
	match update
	{
		Update::Join {
			client_id,
			username,
			unlocks,
			sendbuffer,
		} => handle_join(client_id, username, unlocks, sendbuffer, clients),
		Update::Init { sendbuffer } => handle_init(sendbuffer, clients),
		Update::Leave { client_id } => handle_leave(client_id, clients),

		Update::Msg(message) =>
		{
			for client in clients.iter_mut()
			{
				client.send(message.clone());
			}
		}
	}
}

struct Client
{
	id: Keycode,
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
		match self.sendbuffer.try_send(message)
		{
			Ok(()) => (),
			Err(_error) => self.dead = true,
		}
	}
}

fn handle_join(
	id: Keycode,
	username: String,
	unlocks: EnumSet<Unlock>,
	sendbuffer: mpsc::Sender<Message>,
	clients: &mut Vec<Client>,
)
{
	// TODO ghostbusting

	let join_metadata = generate_join_metadata(&unlocks);
	let hidden = username.starts_with("#");

	let mut newcomer = Client {
		id: id,
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

	// Tell everyone who the newcomer is.
	if !newcomer.hidden
	{
		for other in clients.iter_mut()
		{
			other.send(message.clone());
		}

		// Tell everyone the rating and stars of the newcomer.
		// TODO rating and stars
	}

	// Let the client know which lobbies there are.
	// TODO lobbies

	// Let the client know who else is online.
	for other in clients.iter()
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

	// Tell the newcomer that they are online.
	// TODO this is weird (#1411)
	newcomer.send(message);

	// Let the client know we are done initializing.
	newcomer.send(Message::Init);

	// Show them a welcome message, if any.
	welcome_client(&mut newcomer);

	clients.push(newcomer);
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

fn handle_init(sendbuffer: mpsc::Sender<Message>, clients: &Vec<Client>)
{
	match do_init(sendbuffer, clients)
	{
		Ok(()) => (),
		Err(e) => eprintln!("Send error while processing init: {:?}", e),
	}
}

fn do_init(
	mut sendbuffer: mpsc::Sender<Message>,
	clients: &Vec<Client>,
) -> Result<(), mpsc::error::TrySendError<Message>>
{
	// Let the client know which lobbies there are.
	// TODO lobbies

	// Let the client know who else is online.
	for client in clients
	{
		if !client.hidden
		{
			sendbuffer.try_send(Message::JoinServer {
				status: None,
				content: Some(client.username.clone()),
				sender: None,
				metadata: client.join_metadata,
			})?;

			// TODO rating
			// TODO stars
			// TODO join_lobby
			// TODO in_game
		}
	}

	// Let the client know we are done initializing.
	sendbuffer.try_send(Message::Init)
}

fn handle_leave(client_id: Keycode, clients: &mut Vec<Client>)
{
	let removed: Vec<Client> = clients
		.e_drain_where(|client| client.id == client_id)
		.collect();

	for removed_client in removed
	{
		let Client {
			id: _,
			username,
			join_metadata: _,
			mut sendbuffer,
			hidden,
			dead: _,
		} = removed_client;

		let message = Message::LeaveServer {
			content: Some(username),
		};

		if !hidden
		{
			for client in clients.iter_mut()
			{
				client.send(message.clone());
			}
		}

		match sendbuffer.try_send(message)
		{
			Ok(()) => (),
			Err(e) => eprintln!("Send error while processing leave: {:?}", e),
		}
	}
}
