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

pub fn start() -> mpsc::Sender<Message>
{
	let (general_in, general_out) = mpsc::channel::<Message>(10000);

	let task = start_chat_task(general_out);

	tokio::spawn(task);

	general_in
}

fn start_chat_task(
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
	let mut to_be_removed: Vec<Keycode> = Vec::new();

	match message
	{
		Message::InitInternal { client_id } =>
		{
			// TODO init
			let _ = client_id;
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
			to_be_removed.push(client_id);
		}

		Message::Chat { .. } =>
		{
			for (&id, client) in clients.iter_mut()
			{
				client
					.sendbuffer
					.try_send(message.clone())
					.map_err(|_| to_be_removed.push(id));
			}
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

	for id in to_be_removed
	{
		clients.remove(&id);
	}

	Either::B(future::ok(()))
}

struct Client
{
	pub username: String,
	pub join_metadata: Option<JoinMetadata>,
	pub sendbuffer: mpsc::Sender<Message>,
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

	let client = Client {
		username,
		join_metadata,
		sendbuffer,
	};
	clients.insert(id, client);

	future::ok(())
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
