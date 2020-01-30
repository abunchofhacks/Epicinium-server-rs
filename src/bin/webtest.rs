/* Web Integration Tests */

extern crate epicinium;
extern crate reqwest;
extern crate serde;

use epicinium::server::message::ResponseStatus;

use reqwest as http;
use serde::Deserialize;
use serde::Serialize;

fn main() -> Result<(), Box<dyn std::error::Error>>
{
	// TODO setting or something
	let url = "http://epicinium.localhost";
	let base_url = http::Url::parse(url)?;

	let http = http::Client::new();

	test_top_ratings(http, base_url).expect("failed to get top ratings");

	return Ok(());
}

fn test_top_ratings(
	http: http::Client,
	base_url: http::Url,
) -> Result<(), Box<dyn std::error::Error>>
{
	let request = TopRatingsRequest { amount: 10 };
	let payload = serde_json::to_string(&request)?;

	let mut url = base_url;
	url.set_path("top_ratings.php");
	return http
		.request(http::Method::POST, url)
		.body(payload)
		// TODO useragent
		.send()
		.map_err(|error| error.into())
		.and_then(|response| {
			response.error_for_status().map_err(|error| error.into())
		})
		.and_then(|mut response| response.json().map_err(|error| error.into()))
		.and_then(|response: TopRatingsResponse| match response
		{
			TopRatingsResponse {
				status: ResponseStatus::Success,
				rankings: Some(rankings),
			} if rankings.len() == request.amount => Ok(rankings),
			_ => Err(TopRatingsBadResponseError { response }.into()),
		})
		.map(|rankings| println!("top_ratings: {:#?}", rankings));
}

#[derive(Clone, Serialize, Debug)]
struct TopRatingsRequest
{
	amount: usize,
}

#[derive(Clone, Deserialize, Debug)]
struct TopRatingsResponse
{
	status: ResponseStatus,
	rankings: Option<Vec<TopRatingsRanking>>,
}

#[derive(Clone, Deserialize, Debug)]
struct TopRatingsRanking
{
	username: String,
	rank: u32,
	rating: f64,
}

#[derive(Debug)]
struct TopRatingsBadResponseError
{
	response: TopRatingsResponse,
}

impl std::fmt::Display for TopRatingsBadResponseError
{
	fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result
	{
		write!(f, "bad response: {:?}", self.response)
	}
}
impl std::error::Error for TopRatingsBadResponseError {}
