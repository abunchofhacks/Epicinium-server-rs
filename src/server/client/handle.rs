/**/

use super::Update;

use crate::common::keycode::Keycode;
use crate::server::lobby;
use crate::server::message::Message;

use log::*;

use tokio::sync::mpsc;

#[derive(Debug)]
pub struct Poison {}

#[derive(Debug, Clone)]
pub enum Handle
{
	Connected
	{
		id: Keycode,
		sendbuffer: mpsc::Sender<Message>,
		update_callback: mpsc::Sender<Update>,
		poison_callback: mpsc::Sender<Poison>,
		salts: Option<lobby::Salts>,
	},
	Disconnected
	{
		id: Keycode
	},
}

impl Handle
{
	pub fn is_disconnected(&self) -> bool
	{
		match self
		{
			Handle::Connected { .. } => false,
			Handle::Disconnected { .. } => true,
		}
	}

	pub fn send(&mut self, message: Message)
	{
		match self
		{
			Handle::Connected {
				id,
				sendbuffer,
				update_callback: _,
				poison_callback: _,
				..
			} => match sendbuffer.try_send(message)
			{
				Ok(()) =>
				{}
				Err(error) =>
				{
					error!("Error sending to client {}: {:?}", id, error);
					self.poison();
				}
			},
			Handle::Disconnected { .. } =>
			{}
		}
	}

	pub fn notify(&mut self, update: Update)
	{
		match self
		{
			Handle::Connected {
				id,
				sendbuffer: _,
				update_callback,
				poison_callback: _,
				..
			} => match update_callback.try_send(update)
			{
				Ok(()) =>
				{}
				Err(error) =>
				{
					error!("Error notifying client {}: {:?}", id, error);
					self.poison();
				}
			},
			Handle::Disconnected { .. } =>
			{}
		}
	}

	pub fn generate_and_send_secrets(&mut self, lobby_id: Keycode)
	{
		match self
		{
			Handle::Connected {
				id,
				salts: saved_salts,
				..
			} =>
			{
				let salts = lobby::Salts::generate();
				*saved_salts = Some(salts.clone());
				let secrets = lobby::Secrets::create(lobby_id, *id, salts);
				self.send(Message::Secrets { secrets });
			}
			Handle::Disconnected { .. } =>
			{}
		}
	}

	pub fn take(&mut self) -> Handle
	{
		let id: Keycode = match self
		{
			Handle::Connected { id, .. } => *id,
			Handle::Disconnected { id } => *id,
		};
		let mut result = Handle::Disconnected { id };
		std::mem::swap(self, &mut result);
		result
	}

	fn poison(&mut self)
	{
		match self
		{
			Handle::Connected {
				id,
				sendbuffer: _,
				update_callback: _,
				poison_callback,
				salts: _,
			} =>
			{
				match poison_callback.try_send(Poison {})
				{
					Ok(()) =>
					{}
					Err(_error) =>
					{}
				}
				*self = Handle::Disconnected { id: *id };
			}
			Handle::Disconnected { .. } =>
			{}
		}
	}
}
