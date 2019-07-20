/* Epicinium Rust Lib */

extern crate serde;
extern crate serde_json;

#[macro_use]
extern crate serde_derive;

extern crate enum_map;

#[macro_use]
extern crate enum_map_derive;


pub mod common;
pub mod logic;

pub use self::common::*;
pub use self::logic::*;
