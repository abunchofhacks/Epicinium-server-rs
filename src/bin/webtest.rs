/* Web Integration Tests */

extern crate epicinium;
extern crate reqwest;
extern crate serde;

use epicinium::server::message::ResponseStatus;

use reqwest as http;
use serde::Deserialize;

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
	let mut url = base_url;
	url.set_path("top_ratings.php");
	return http
		.request(http::Method::POST, url)
		// TODO useragent
		.send()
		.map_err(|error| error.into())
		.and_then(|response| {
			response.error_for_status().map_err(|error| error.into())
		})
		.and_then(|mut response| response.json().map_err(|error| error.into()))
		//.map(|response| {
		//	()
		//})
		.and_then(|response: TopRatingsResponse| {
			if response.status == ResponseStatus::Success
			{
				Ok(())
			}
			else
			{
				unreachable!("bad response: {:?}", response)
			}
		})
		.map(|()| println!("top_ratings: ok"));
}

#[derive(Clone, Deserialize, Debug)]
struct TopRatingsResponse
{
	status: ResponseStatus,
}
