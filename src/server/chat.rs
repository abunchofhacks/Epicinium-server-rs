/* Server::Chat */

use common::keycode::*;
use server::message::*;

use std::collections::HashMap;

use futures::future::Future;
use futures::stream::Stream;

use tokio::sync::mpsc;

use enumset::*;

pub struct Server
{
	general: mpsc::Sender<Message>,
}

impl Server
{
	pub fn start() -> Self
	{
		let (general_in, general_out) = mpsc::channel::<Message>(10000);

		let task = start_chat_task(general_out);

		tokio::spawn(task);

		Server {
			general: general_in,
		}
	}

	pub fn join(
		&self,
		client_id: Keycode,
		logindata: LoginData,
		mut sendbuffer: mpsc::Sender<Message>,
	) -> Result<Option<LoginData>, mpsc::error::TrySendError<Message>>
	{
		let mut unlocks = EnumSet::<Unlock>::empty();
		for &x in &logindata.unlocks
		{
			unlocks.insert(unlock_from_unlock_id(x));
		}

		if !unlocks.contains(Unlock::Access)
		{
			println!("Login failed due to insufficient access");
			return sendbuffer
				.try_send(Message::JoinServer {
					status: Some(ResponseStatus::KeyRequired),
					content: None,
					sender: None,
					metadata: None,
				})
				.map(|()| None);
		}

		// TODO ghostbusting

		self.general
			.try_send(Message::JoinServerInternal {
				client_id: client_id,
				login_data: logindata.clone(),
				sendbuffer: sendbuffer,
			})
			.map(|()| Some(logindata))
	}
}

fn start_chat_task(
	messages: mpsc::Receiver<Message>,
) -> impl Future<Item = (), Error = ()> + Send
{
	let mut clients: HashMap<Keycode, Client> = HashMap::new();

	messages
		.map_err(|error| eprintln!("Recv error in chat_task: {:?}", error))
		.for_each(move |message| {
			let mut to_be_removed: Vec<Keycode> = Vec::new();

			match message
			{
				Message::InitInternal { client_id } =>
				{
					// TODO init
					let _ = client_id;
				}
				Message::JoinServerInternal {
					client_id: id,
					login_data,
					sendbuffer,
				} =>
				{
					clients.insert(id, Client::new(login_data, sendbuffer));
				}
				Message::LeaveServerInternal { client_id } =>
				{
					to_be_removed.push(client_id);
				}

				Message::Chat { .. } =>
				{
					for (&id, client) in &mut clients
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
				| Message::SetUsernameInternal { .. } =>
				{
					panic!("Misrouted message in chat_task: {:?}", message);
				}
			}

			for id in to_be_removed
			{
				clients.remove(&id);
			}

			Ok(())
		})
}

struct Client
{
	pub username: String,
	pub join_metadata: Option<JoinMetadata>,
	pub sendbuffer: mpsc::Sender<Message>,
}

impl Client
{
	fn new(logindata: LoginData, sendbuffer: mpsc::Sender<Message>) -> Self
	{
		let join_metadata = generate_join_metadata(&logindata);

		Client {
			username: logindata.username,
			join_metadata,
			sendbuffer,
		}
	}
}

fn generate_join_metadata(logindata: &LoginData) -> Option<JoinMetadata>
{
	let mut unlocks = EnumSet::<Unlock>::empty();
	for &x in &logindata.unlocks
	{
		unlocks.insert(unlock_from_unlock_id(x));
	}

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
