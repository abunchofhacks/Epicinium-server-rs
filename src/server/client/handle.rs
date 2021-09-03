/*
 * Part of epicinium_server
 * developed by A Bunch of Hacks.
 *
 * Copyright (c) 2018-2021 A Bunch of Hacks
 *
 * This library is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This library is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 * [authors:]
 * Sander in 't Veld (sander@abunchofhacks.coop)
 */

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

	pub fn verify_invite(&self, invite: &lobby::Invite) -> bool
	{
		match self
		{
			Handle::Connected {
				id,
				salts: Some(salts),
				..
			} => match invite
			{
				lobby::Invite::JoinSecret(secret) =>
				{
					secret.client_id == *id
						&& secret.salt == salts.join_secret_salt
				}
				lobby::Invite::SpectateSecret(secret) =>
				{
					secret.client_id == *id
						&& secret.salt == salts.spectate_secret_salt
				}
			},
			Handle::Connected { salts: None, .. } => false,
			Handle::Disconnected { .. } => false,
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
