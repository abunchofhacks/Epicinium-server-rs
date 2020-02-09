/* Server::Lobby */

use crate::common::keycode::*;
use crate::server::message::*;

use futures::future;
use futures::future::Either;
use futures::future::Future;
use futures::stream::Stream;

use tokio::sync::mpsc;
use tokio::sync::oneshot;

use enumset::*;
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

pub fn start_task(
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
		.map_err(|error| {
			eprintln!("Closing error in lobby {}: {:?}", lobby_id, error)
		})
		.into_stream();

	updates
		.map_err(|error| {
			eprintln!("Recv error in lobby {}: {:?}", lobby_id, error)
		})
		.select(closing_updates)
		.for_each(move |update| {
			handle_update(update, lobby_id, &mut clients, &mut close)
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
) -> impl Future<Item = (), Error = ()> + Send
{
	match update
	{
		Update::Join { .. } | Update::Leave { .. } if close.is_closed =>
		{}
		Update::Join {
			client_id,
			username,
			sendbuffer,
		} =>
		{
			return Either::A(handle_join(
				client_id, username, sendbuffer, clients,
			));
		}
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

	Either::B(future::ok(()))
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
	id: Keycode,
	username: String,
	sendbuffer: mpsc::Sender<Message>,
	clients: &mut Vec<Client>,
) -> impl Future<Item = (), Error = ()> + Send
{
	let mut newcomer = Client {
		id: id,
		username,
		sendbuffer,
		dead: false,
	};

	// TODO implement

	clients.push(newcomer);

	future::ok(())
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
