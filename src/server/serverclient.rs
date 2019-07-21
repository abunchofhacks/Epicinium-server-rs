/* ServerClient */

use std::net;

pub struct ServerClient
{
	stream: net::TcpStream,
}

impl ServerClient
{
	pub fn create(stream: net::TcpStream) -> ServerClient
	{
		println!("Incoming connection: {:?}", stream.peer_addr());

		ServerClient { stream: stream }
	}
}
