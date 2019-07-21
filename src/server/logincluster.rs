/* LoginCluster */

pub struct LoginCluster
{
	login_server: String,
	closing: bool,

	clients: Vec<Client>,
}

struct Client;

impl LoginCluster
{
	pub fn create() -> LoginCluster
	{
		LoginCluster {
			login_server: String::from(""),
			closing: false,
			clients: Vec::new(),
		}
	}

	pub fn close(&mut self)
	{
		self.closing = true;
	}

	pub fn closed(&self) -> bool
	{
		self.closing && self.clients.is_empty()
	}
}
