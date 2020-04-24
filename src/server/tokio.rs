/* Server::Tokio */

use crate::common::coredump::enable_coredumps;
use crate::common::keycode::*;
//use crate::server::chat;
use crate::server::client;
use crate::server::killer;
use crate::server::login;
use crate::server::portal;
use crate::server::settings::*;

use std::error;
use std::net::SocketAddr;
use std::sync;

use futures::FutureExt;
use futures::TryFutureExt;
use futures::{select, try_join};

use tokio::net::TcpListener;
use tokio::sync::watch;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum State
{
	Open,
	Closing,
	Closed,
}

#[tokio::main]
pub async fn run_server(
	settings: &Settings,
) -> Result<(), Box<dyn error::Error>>
{
	enable_coredumps()?;
	increase_sockets()?;

	let login_server = login::connect(settings)?;
	let login = sync::Arc::new(login_server);

	let (state_in, state_out) = watch::channel(State::Open);
	let acceptance_task = accept_clients(settings, login, state_out);

	let (killcount_in, killcount_out) = watch::channel(0u8);
	let killer_task = killer::run(killcount_in).map_err(|e| e.into());

	try_join!(acceptance_task, killer_task).map(|((), ())| ())
}

async fn accept_clients(
	settings: &Settings,
	login: sync::Arc<login::Server>,
	mut server_state: watch::Receiver<State>,
) -> Result<(), Box<dyn error::Error>>
{
	let server = settings.get_server()?;
	let ipaddress = server.to_string();
	let binding: portal::Binding = portal::bind(settings).await?;
	let port = binding.port;
	let address: SocketAddr = format!("{}:{}", ipaddress, port).parse()?;
	let mut listener = TcpListener::bind(&address).await?;
	binding.confirm().await?;

	println!("Listening on {}:{}", ipaddress, port);

	let mut ticker: u64 = rand::random();

	loop
	{
		let socket = select! {
			listened = listener.accept().fuse() => match listened
			{
				Ok((socket, _addr)) =>
				{
					socket
				},
				Err(error) =>
				{
					println!("Failed to connect client: {:?}", error);
					continue
				}
			},
			state = server_state.recv().fuse() => match state
			{
				Some(State::Open) => continue,
				Some(State::Closing) => break,
				Some(State::Closed) => break,
				None => break,
			}
		};

		let serial = ticker;
		ticker += 1;
		let key: u16 = rand::random();
		let id = keycode(key, serial);

		client::accept(socket, id, login.clone(), server_state.clone());
	}

	binding.unbind().await
}

fn increase_sockets() -> std::io::Result<()>
{
	const MAX_SOCKETS: rlimit::rlim = 16384;
	rlimit::Resource::NOFILE.set(MAX_SOCKETS, MAX_SOCKETS)
}
