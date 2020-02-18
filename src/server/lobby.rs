/* Server::Lobby */

use crate::common::keycode::*;
use crate::server::message::*;

use futures::future::Future;
use futures::stream::Stream;

use tokio::sync::mpsc;

use vec_drain_where::VecDrainWhereExt;

#[derive(Debug)]
pub enum Update
{
	Join
	{
		client_id: Keycode,
		username: String,
		sendbuffer: mpsc::Sender<Message>,
	},
	Leave
	{
		client_id: Keycode,
	},

	Msg(Message),
}

pub fn create(creator_id: Keycode, updates: mpsc::Receiver<Update>) -> Keycode
{
	// TODO data from timestamp
	let key = rand::random();
	let data = rand::random();
	let lobby_id = keycode(key, data);

	let task = start_task(lobby_id, updates);
	tokio::spawn(task);

	lobby_id
}

fn start_task(
	lobby_id: Keycode,
	updates: mpsc::Receiver<Update>,
) -> impl Future<Item = (), Error = ()> + Send
{
	let mut clients: Vec<Client> = Vec::new();

	updates
		.map_err(move |error| {
			eprintln!("Recv error in lobby {}: {:?}", lobby_id, error)
		})
		.for_each(move |update| {
			handle_update(update, lobby_id, &mut clients);
			Ok(())
		})
}

fn handle_update(update: Update, lobby_id: Keycode, clients: &mut Vec<Client>)
{
	// TODO is_closed (necessary?)
	let is_closed = false;

	match update
	{
		Update::Join { .. } | Update::Leave { .. } if is_closed =>
		{}
		Update::Join {
			client_id,
			username,
			sendbuffer,
		} => handle_join(lobby_id, client_id, username, sendbuffer, clients),
		Update::Leave { client_id } =>
		{
			handle_leave(lobby_id, client_id, clients)
		}

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
	sendbuffer: mpsc::Sender<Message>,
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
	lobby_id: Keycode,
	client_id: Keycode,
	username: String,
	sendbuffer: mpsc::Sender<Message>,
	clients: &mut Vec<Client>,
)
{
	// TODO joining might fail because it is full or locked etcetera

	let mut newcomer = Client {
		id: client_id,
		username,
		sendbuffer,
		dead: false,
	};

	// TODO max players
	newcomer.send(Message::MaxPlayers { lobby_id, value: 2 });

	for other in clients.into_iter()
	{
		newcomer.send(Message::JoinLobby {
			lobby_id: Some(lobby_id),
			username: Some(other.username.clone()),
			metadata: None,
		});

		// TODO roles
		// TODO colors
		// TODO vision types
	}

	// TODO AI pool
	// TODO bots

	// TODO map pool if this is not a replay lobby
	// TODO other map settings
	// TODO list all recordings if this is a replay lobby
	// TODO other replay settings

	clients.push(newcomer);
}

fn handle_leave(
	lobby_id: Keycode,
	client_id: Keycode,
	clients: &mut Vec<Client>,
)
{
	let removed: Vec<Client> = clients
		.e_drain_where(|client| client.id == client_id)
		.collect();

	for removed_client in removed
	{
		let Client {
			id: _,
			username,
			mut sendbuffer,
			dead: _,
		} = removed_client;

		let message = Message::LeaveLobby {
			lobby_id: Some(lobby_id),
			username: Some(username),
		};

		for client in clients.iter_mut()
		{
			client.send(message.clone());
		}

		match sendbuffer.try_send(message)
		{
			Ok(()) => (),
			Err(e) => eprintln!("Send error while processing leave: {:?}", e),
		}
	}
}
