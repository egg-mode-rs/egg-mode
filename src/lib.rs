//! A library for interacting with Twitter.
//!
//! Please see [the repository][] and its enclosed examples for tips on working with this library
//! while it's still in progress.
//!
//! [the repository]: https://github.com/icesoldier/twitter-rs

#![warn(missing_docs)]
#![warn(unused_extern_crates)]
#![warn(unused_qualifications)]

#[macro_use] extern crate hyper;
extern crate url;
extern crate time;
extern crate rand;
extern crate crypto;
extern crate rustc_serialize;
extern crate mime;

mod auth;
pub mod error;
pub mod user;
mod links;
mod common;

pub use auth::{Token, request_token, authorize_url, access_token};
pub use common::{Response, TwitterErrors, TwitterErrorCode, UserID};
