/* LoginServer */

use server::message::LoginResponseData;

use futures::future::Future;

use tokio::sync::mpsc;

use reqwest::async as http;

pub struct LoginRequest
{
	token: String,
	account_id: String,
	client: mpsc::Sender<LoginResponseData>,
}

pub fn start_login_task(
	requests: mpsc::Receiver<LoginRequest>,
) -> impl Future<Item = (), Error = ()>
{
	// TODO
}
