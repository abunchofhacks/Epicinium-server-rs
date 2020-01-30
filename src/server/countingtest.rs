/* Server::Test::Counting */

use common::coredump;
use common::version::*;
use server::message::*;
use server::settings::*;

use std::error;
use std::io;
use std::io::ErrorKind;
use std::net::SocketAddr;

use futures::future::Either;

use rand::seq::SliceRandom;
use tokio::io::{ReadHalf, WriteHalf};
use tokio::net::TcpStream;
use tokio::prelude::*;

pub fn run(settings: &Settings) -> Result<(), Box<dyn error::Error>>
{
	coredump::enable_coredumps()?;

	let mut ntests: usize = 2;
	let mut fakeversion: Version = Version::current();

	for arg in std::env::args().skip(1)
	{
		if arg.starts_with("-")
		{
			// Setting argument, will be handled by Settings.
		}
		else if arg.starts_with("v")
		{
			fakeversion = arg.parse()?;
		}
		else
		{
			ntests = arg.parse()?;
		}
	}

	let server = settings.get_server()?;
	let port = settings.get_port()?;

	println!(
		"ntests = {}, fakeversion = v{}, server = {}, port = {}",
		ntests,
		fakeversion.to_string(),
		server,
		port,
	);

	let serveraddress: SocketAddr = format!("{}:{}", server, port).parse()?;

	// TODO seed

	let mut numbers: Vec<usize> = (0..ntests).collect();
	let mut rng = rand::thread_rng();
	numbers.shuffle(&mut rng);

	let tests = numbers
		.iter()
		.map(|&number| start_test(number, fakeversion, &serveraddress));
	let all_tests = stream::futures_unordered(tests).fold((), |(), ()| Ok(()));

	tokio::run(all_tests);
	Ok(())
}

fn start_test(
	number: usize,
	fakeversion: Version,
	serveraddress: &SocketAddr,
) -> impl Future<Item = (), Error = ()> + Send
{
	println!("[{}] Connecting...", number);

	TcpStream::connect(&serveraddress)
		.map_err(move |error| {
			eprintln!("[{}] Failed to connect: {:?}", number, error)
		})
		.and_then(move |connection| run_test(connection, number, fakeversion))
}

fn run_test(
	socket: TcpStream,
	number: usize,
	fakeversion: Version,
) -> impl Future<Item = (), Error = ()> + Send
{
	println!("[{}] Connected.", number);

	let initialmessages = vec![Message::Version {
		version: fakeversion,
	}];

	let (reader, writer) = socket.split();
	stream::unfold(reader, move |socket| {
		let lengthbuffer = [0u8; 4];
		let future_length = tokio_io::io::read_exact(socket, lengthbuffer)
			.and_then(move |(socket, lengthbuffer)| {
				let length = u32::from_le_bytes(lengthbuffer);
				receive_message(socket, number, length)
			});

		Some(future_length)
	})
	.map_err(move |error| {
		eprintln!("[{}] Failed to receive: {:?}", number, error)
	})
	.and_then(move |message| handle_message(number, message))
	.map(|responses| stream::iter_ok(responses))
	.flatten()
	.select(stream::iter_ok(initialmessages))
	.map(move |message| prepare_message(number, message))
	.fold(writer, move |writer, bytes| {
		send_bytes(writer, number, bytes)
	})
	.map(|_writer| ())
}

fn receive_message(
	socket: ReadHalf<TcpStream>,
	number: usize,
	length: u32,
) -> impl Future<Item = (Message, ReadHalf<TcpStream>), Error = io::Error>
{
	if length == 0
	{
		println!("[{}] Received pulse.", number);
		return Either::A(future::ok((Message::Pulse, socket)));
	}

	println!("[{}] Receiving message of length {}...", number, length);
	let buffer = vec![0; length as usize];
	Either::B(tokio_io::io::read_exact(socket, buffer).and_then(
		move |(socket, buffer)| {
			println!(
				"[{}] Received message of length {}.",
				number,
				buffer.len()
			);
			let message = parse_message(number, buffer)?;

			// Unfold expects the value first and the state second.
			Ok((message, socket))
		},
	))
}

fn parse_message(number: usize, buffer: Vec<u8>) -> io::Result<Message>
{
	let jsonstr = match String::from_utf8(buffer)
	{
		Ok(x) => x,
		Err(e) =>
		{
			return Err(io::Error::new(ErrorKind::InvalidData, e));
		}
	};

	if jsonstr.len() < 200
	{
		println!("[{}] Received message: {}", number, jsonstr);
	}

	let message: Message = serde_json::from_str(&jsonstr)?;

	Ok(message)
}

fn handle_message(number: usize, message: Message) -> Result<Vec<Message>, ()>
{
	match message
	{
		Message::Ping => Ok(vec![Message::Pong]),
		Message::Pong | Message::Pulse => Ok(Vec::new()),
		Message::Version { version: _ } => Ok(vec![Message::JoinServer {
			status: None,
			content: Some("".to_string()),
			sender: Some((1000 + number).to_string()),
			metadata: None,
		}]),
		Message::JoinServer { content: None, .. } =>
		{
			eprintln!("[{}] Failed to join chat", number);
			Err(())
		}
		Message::JoinServer {
			content: Some(_name),
			..
		} => Ok(Vec::new()),
		Message::Init =>
		{
			// TODO send number 0

			// TODO remove this
			Ok(vec![Message::Quit])
		}
		Message::Chat {
			content: _,
			sender: _,
			target: _,
		} =>
		{
			// TODO send other numbers
			Ok(Vec::new())
		}
		Message::LeaveServer { .. } => Ok(Vec::new()),
		Message::Closing =>
		{
			eprintln!("[{}] Server closing unexpectedly", number);
			Err(())
		}
		Message::Quit =>
		{
			eprintln!("[{}] Server closed unexpectedly", number);
			Err(())
		}
	}
}

fn prepare_message(number: usize, message: Message) -> Vec<u8>
{
	if let Message::Pulse = message
	{
		println!("[{}] Sending pulse...", number);

		let zeroes = [0u8; 4];
		return zeroes.to_vec();
	}

	let (jsonstr, length) = prepare_message_data(number, message);

	let mut buffer = length.to_le_bytes().to_vec();
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

	println!("[{}] Sending message of length {}...", number, length);

	if length < 200
	{
		println!("[{}] Sending message: {}", number, jsonstr);
	}

	(jsonstr, length)
}

fn send_bytes(
	socket: WriteHalf<TcpStream>,
	number: usize,
	buffer: Vec<u8>,
) -> impl Future<Item = WriteHalf<TcpStream>, Error = ()> + Send
{
	tokio_io::io::write_all(socket, buffer)
		.map(move |(socket, buffer)| {
			/*verbose*/
			println!("[{}] Sent {} bytes.", number, buffer.len());
			socket
		})
		.map_err(move |error| {
			eprintln!("[{}] Failed to send: {:?}", number, error);
		})
}
