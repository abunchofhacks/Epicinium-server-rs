/* Server::Lobby */

use crate::common::keycode::*;
use crate::server::message::*;

use futures::future::Future;
use futures::stream::Stream;

use tokio::sync::mpsc;
use tokio::sync::oneshot;

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
	Closing,

	Msg(Message),
}

pub fn create(
	creator_id: Keycode,
	updates: mpsc::Receiver<Update>,
	closing: oneshot::Receiver<()>,
	closed: oneshot::Sender<()>,
) -> Keycode
{
	// TODO data from timestamp
	let key = rand::random();
	let data = rand::random();
	let lobby_id = keycode(key, data);

	let task = start_task(lobby_id, updates, closing, closed);
	tokio::spawn(task);

	lobby_id
}

fn start_task(
	lobby_id: Keycode,
	updates: mpsc::Receiver<Update>,
	closing: oneshot::Receiver<()>,
	closed: oneshot::Sender<()>,
) -> impl Future<Item = (), Error = ()> + Send
{
	let mut clients: Vec<Client> = Vec::new();
	let mut close = Close {
		is_closing: false,
		is_closed: false,
		watcher: Some(closed),
	};

	let closing_updates = closing
		.map(|()| Update::Closing)
		.map_err(move |error| {
			eprintln!("Closing error in lobby {}: {:?}", lobby_id, error)
		})
		.into_stream();

	updates
		.map_err(move |error| {
			eprintln!("Recv error in lobby {}: {:?}", lobby_id, error)
		})
		.select(closing_updates)
		.for_each(move |update| {
			handle_update(update, lobby_id, &mut clients, &mut close);
			Ok(())
		})
}

struct Close
{
	is_closing: bool,
	is_closed: bool,
	watcher: Option<oneshot::Sender<()>>,
}

fn handle_update(
	update: Update,
	lobby_id: Keycode,
	clients: &mut Vec<Client>,
	close: &mut Close,
)
{
	match update
	{
		Update::Join { .. } | Update::Leave { .. } if close.is_closed =>
		{}
		Update::Join {
			client_id,
			username,
			sendbuffer,
		} => handle_join(lobby_id, client_id, username, sendbuffer, clients),
		Update::Leave { client_id } =>
		{
			handle_leave(lobby_id, client_id, clients, close)
		}
		Update::Closing =>
		{
			close.is_closing = true;
			if clients.is_empty()
			{
				do_close(close);
			}
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
	close: &mut Close,
)
{
	do_leave(lobby_id, client_id, clients);

	if close.is_closing && clients.is_empty()
	{
		do_close(close);
	}
}

fn do_close(close: &mut Close)
{
	if let Some(watcher) = close.watcher.take()
	{
		match watcher.send(())
		{
			Ok(()) => (),
			Err(_error) => println!("Lobby force-closed."),
		}
	}
	close.is_closed = true;
}

fn do_leave(lobby_id: Keycode, client_id: Keycode, clients: &mut Vec<Client>)
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
