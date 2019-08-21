/* Notice */

use server::message::*;

use std::io;

use tokio::prelude::*;

pub fn load() -> impl Future<Item = StampMetadata, Error = ()>
{
	load_from_file().map_err(|e| match e
	{
		LoadError::Read { error } =>
		{
			eprintln!("Failed to load stamp: {:?}", error);
		}
		LoadError::Utf8 { error } =>
		{
			eprintln!("Failed to interpret stamp as utf8: {:?}", error);
		}
		LoadError::Parse { error } =>
		{
			eprintln!("Failed to parse stamp: {:?}", error);
		}
	})
}

fn load_from_file() -> impl Future<Item = StampMetadata, Error = LoadError>
{
	tokio::fs::read("server-notice.json")
		.map_err(|error| LoadError::Read { error })
		.and_then(|buffer| {
			String::from_utf8(buffer).map_err(|error| LoadError::Utf8 { error })
		})
		.and_then(|raw| {
			serde_json::from_str::<StampMetadata>(&raw)
				.map_err(|error| LoadError::Parse { error })
		})
}

enum LoadError
{
	Read
	{
		error: io::Error
	},
	Utf8
	{
		error: std::string::FromUtf8Error
	},
	Parse
	{
		error: serde_json::Error
	},
}
