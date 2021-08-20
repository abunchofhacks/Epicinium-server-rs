/* Server::Test::Counting */

use crate::common::coredump;
use crate::common::version::*;
use crate::server::message::*;

use std::net::SocketAddr;

use log::*;

use anyhow::anyhow;

use rand::seq::SliceRandom;

use tokio::io::{ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::prelude::*;

#[tokio::main]
pub async fn run(
	ntests: usize,
	fakeversion: Version,
	server: String,
	port: u16,
) -> Result<(), anyhow::Error>
{
	coredump::enable_coredumps()?;
	increase_sockets()?;

	info!(
		"Starting (ntests = {}, fakeversion = v{}, server = {}, port = {})...",
		ntests,
		fakeversion.to_string(),
		server,
		port,
	);

	let serveraddress: SocketAddr = format!("{}:{}", server, port).parse()?;

	let mut numbers: Vec<usize> = (0..ntests).collect();
	let mut rng = rand::thread_rng();
	numbers.shuffle(&mut rng);

	let tests = numbers
		.iter()
		.map(|&number| run_test(number, ntests, fakeversion, &serveraddress));
	futures::future::try_join_all(tests).await?;
	Ok(())
}

async fn run_test(
	number: usize,
	count: usize,
	fakeversion: Version,
	serveraddress: &SocketAddr,
) -> Result<(), anyhow::Error>
{
	match run_test_impl(number, count, fakeversion, serveraddress).await
	{
		Ok(()) => Ok(()),
		Err(error) =>
		{
			error!("[{}] {:?}", number, error);
			Err(error)
		}
	}
}

async fn run_test_impl(
	number: usize,
	count: usize,
	fakeversion: Version,
	serveraddress: &SocketAddr,
) -> Result<(), anyhow::Error>
{
	debug!("[{}] Connecting...", number);

	let connection = TcpStream::connect(&serveraddress).await?;

	debug!("[{}] Connected.", number);

	let initialmessage = Message::Version {
		version: fakeversion,
	};

	let mut has_quit = QuitStage::None;

	// Wait for count + 1 JoinServer messages because the first JoinServer is
	// information about us successfully joining.
	let mut waiting = if number == 0 { count + 1 } else { 0 };

	let (mut reader, mut writer) = tokio::io::split(connection);

	send_message(number, &mut writer, initialmessage).await?;

	let debugmessage = Message::Debug {
		content: format!("I am number {}", number),
	};
	send_message(number, &mut writer, debugmessage).await?;

	while let Some(message) = receive_message(number, &mut reader).await?
	{
		assert!(has_quit != QuitStage::Received);

		let responses =
			handle_message(number, &mut waiting, &mut has_quit, message)
				.map_err(|()| anyhow!("error while handling message"))?;

		for response in responses
		{
			send_message(number, &mut writer, response).await?;
		}
	}

	if has_quit != QuitStage::Received
	{
		error!("[{}] Stopped receiving unexpectedly", number);
	}

	Ok(())
}

#[derive(Debug, PartialEq, Eq)]
enum QuitStage
{
	None,
	Sent,
	Received,
}

async fn receive_message(
	number: usize,
	socket: &mut ReadHalf<TcpStream>,
) -> Result<Option<Message>, anyhow::Error>
{
	let length = match socket.read_u32().await
	{
		Ok(length) => length,
		Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof =>
		{
			return Ok(None);
		}
		Err(error) => return Err(error.into()),
	};
	if length == 0
	{
		trace!("[{}] Received pulse.", number);
		return Ok(Some(Message::Pulse));
	}

	trace!("[{}] Receiving message of length {}...", number, length);
	let mut buffer = vec![0u8; length as usize];
	socket.read_exact(&mut buffer).await?;

	trace!("[{}] Received message of length {}.", number, buffer.len());
	let message = parse_message(number, buffer)?;

	Ok(Some(message))
}

fn parse_message(
	number: usize,
	buffer: Vec<u8>,
) -> Result<Message, anyhow::Error>
{
	let jsonstr = String::from_utf8(buffer)?;

	if log_enabled!(log::Level::Trace)
	{
		trace!("[{}] Received message: {}", number, jsonstr);
	}

	let message: Message = serde_json::from_str(&jsonstr)?;

	Ok(message)
}

fn handle_message(
	number: usize,
	waiting: &mut usize,
	has_quit: &mut QuitStage,
	message: Message,
) -> Result<Vec<Message>, ()>
{
	match message
	{
		Message::Ping => Ok(vec![Message::Pong]),
		Message::Pong | Message::Pulse => Ok(Vec::new()),
		Message::Version { version: _ } => Ok(vec![Message::JoinServer {
			status: None,
			content: Some("".to_string()),
			sender: Some((1000 + number).to_string()),
			metadata: Default::default(),
		}]),
		Message::JoinServer { content: None, .. } =>
		{
			error!("[{}] Failed to join chat", number);
			Err(())
		}
		Message::JoinServer {
			content: Some(_name),
			..
		} =>
		{
			if number == 0
			{
				println!("{}...", waiting);
				info!("{}...", waiting);

				if *waiting > 1
				{
					*waiting -= 1;
				}
				else if *waiting == 1
				{
					println!("{}!", number);
					info!("{}!", number);
					*has_quit = QuitStage::Sent;
					return Ok(vec![
						Message::Chat {
							content: number.to_string(),
							sender: None,
							target: ChatTarget::General,
						},
						Message::Quit,
					]);
				}
			}

			Ok(Vec::new())
		}
		Message::Init => Ok(Vec::new()),
		Message::Chat {
			content,
			sender: _,
			target: _,
		} =>
		{
			let x: usize = content.parse().map_err(|error| {
				error!("[{}] Failed to parse {}: {}", number, content, error);
			})?;

			if x + 1 == number
			{
				println!("{}!", number);
				info!("{}!", number);
				*has_quit = QuitStage::Sent;
				return Ok(vec![
					Message::Chat {
						content: number.to_string(),
						sender: None,
						target: ChatTarget::General,
					},
					Message::Quit,
				]);
			}

			Ok(Vec::new())
		}
		Message::LeaveServer { content } =>
		{
			if content.is_none()
			{
				error!("[{}] Server refusing access", number);
				Err(())
			}
			else
			{
				Ok(Vec::new())
			}
		}
		Message::Closing =>
		{
			error!("[{}] Server closing unexpectedly", number);
			Err(())
		}
		Message::Closed =>
		{
			error!("[{}] Server closed unexpectedly", number);
			Err(())
		}
		Message::Quit => match *has_quit
		{
			QuitStage::Sent =>
			{
				*has_quit = QuitStage::Received;
				Ok(Vec::new())
			}
			QuitStage::None | QuitStage::Received =>
			{
				error!("[{}] Server closed unexpectedly", number);
				Err(())
			}
		},
		Message::ListChallenge { .. } => Ok(Vec::new()),
		Message::RatingAndStars { .. } => Ok(Vec::new()),
		Message::RecentStars { .. } => Ok(Vec::new()),
		_ =>
		{
			unreachable!();
		}
	}
}

async fn send_message(
	number: usize,
	socket: &mut WriteHalf<TcpStream>,
	message: Message,
) -> Result<(), io::Error>
{
	let buffer = prepare_message(number, message);

	socket.write_all(&buffer).await?;

	trace!("[{}] Sent {} bytes.", number, buffer.len());
	Ok(())
}

fn prepare_message(number: usize, message: Message) -> Vec<u8>
{
	if let Message::Pulse = message
	{
		trace!("[{}] Sending pulse...", number);

		let zeroes = [0u8; 4];
		return zeroes.to_vec();
	}

	let (jsonstr, length) = prepare_message_data(number, message);

	let mut buffer = length.to_be_bytes().to_vec();
	buffer.append(&mut jsonstr.into_bytes());

	buffer
}

fn prepare_message_data(number: usize, message: Message) -> (String, u32)
{
	let jsonstr = match serde_json::to_string(&message)
	{
		Ok(data) => data,
		Err(e) =>
		{
			panic!("Invalid message: {:?}", e);
		}
	};

	let length = jsonstr.len() as u32;

	trace!("[{}] Sending message of length {}...", number, length);

	if log_enabled!(log::Level::Trace)
	{
		trace!("[{}] Sending message: {}", number, jsonstr);
	}

	(jsonstr, length)
}

fn increase_sockets() -> std::io::Result<()>
{
	const MAX_SOCKETS: rlimit::rlim = 16384;
	rlimit::Resource::NOFILE.set(MAX_SOCKETS, MAX_SOCKETS)
}
