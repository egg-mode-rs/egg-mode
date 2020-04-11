// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! A library for interacting with Twitter.
//!
//! [Repository](https://github.com/QuietMisdreavus/twitter-rs)
//!
//! egg-mode is a Twitter library that aims to make as few assumptions about the user's codebase as
//! possible. Endpoints are exposed as bare functions where authentication details are passed in as
//! arguments, rather than as builder functions of a root "service" manager. The only exceptions to
//! this guideline are endpoints with many optional parameters, like posting a status update or
//! updating the metadata of a list.
//!
//! # About the examples in this documentation
//!
//! There are a couple prerequisites to using egg-mode, which its examples also assume:
//!
//! * All methods that hit the twitter API are `async` and should be awaited with the `.await` syntax.
//!   All such calls return a result type with the `Error` enum as their Error value.
//!   The resulting future must be executed on a `tokio` executor.
//!   For more information, check out the [Rust `async` book][rust-futures] and the
//!   [Tokio documentation guides][].
//!
//! * Twitter tracks API use through "tokens" which are managed by Twitter and processed separately
//!   for each "authenticated user" you wish to connect to your app. egg-mode's [Token]
//!   documentation describes how you can obtain one of these, but each example outside of the
//!   authentication documentation brings in a `Token` "offscreen", to avoid distracting from the
//!   rest of the example.
//!
//! [Token]: enum.Token.html
//! [tokio]: https://tokio.rs
//! [rust-futures]: https://rust-lang.github.io/async-book/
//! [Tokio documentation guides]: https://tokio.rs/docs/overview
//!
//! To load the profile information of a single user:
//!
//! ```rust,no_run
//! # use egg_mode::Token;
//! # #[tokio::main]
//! # async fn main() {
//! # let token: Token = unimplemented!();
//! let rustlang = egg_mode::user::show("rustlang", &token).await.unwrap();
//!
//! println!("{} (@{})", rustlang.name, rustlang.screen_name);
//! # }
//! ```
//!
//! To post a new tweet:
//!
//! ```rust,no_run
//! # use egg_mode::Token;
//! use egg_mode::tweet::DraftTweet;
//! # #[tokio::main]
//! # async fn main() {
//! # let token: Token = unimplemented!();
//!
//! let post = DraftTweet::new("Hey Twitter!").send(&token).await.unwrap();
//! # }
//! ```
//!
//! # Types and Functions
//!
//! All of the main content of egg-mode is in submodules, but there are a few things here in the
//! crate root. To wit, it contains items related to authentication and a couple items that all the
//! submodules use.
//!
//! ## `Response<T>`
//!
//! Every method that calls Twitter and carries rate-limit information wraps its return value in a
//! [`Response`][] struct, that transmits this information to your app. From there, you can handle
//! the rate-limit information to hold off on that kind of request, or simply grab its `response`
//! field to get the output of whatever method you called. `Response` also implements `Deref`, so
//! for the most part you can access fields of the final result without having to grab the
//! `response` field directly.
//!
//! [`Response`]: struct.Response.html
//!
//! ## Authentication
//!
//! The remaining types and methods are explained as part of the [authentication overview][Token],
//! with the exception of `verify_tokens`, which is a simple method to ensure a given token is
//! still valid.
//!
//! # Modules
//!
//! As there are many actions available in the Twitter API, egg-mode divides them roughly into
//! several modules by their shared purpose. Here's a sort of high-level overview, in rough order
//! from "most important" to "less directly used":
//!
//! ## Primary actions
//!
//! These could be considered the "core" actions within the Twitter API that egg-mode has made
//! available.
//!
//! * `tweet`: This module lets you act on tweets. Here you can find actions to load a user's
//!   timeline, post a new tweet, or like and retweet individual posts.
//! * `user`: This module lets you act on users, be it by following or unfollowing them, loading
//!   their profile information, blocking or muting them, or showing the relationship between two
//!   users.
//! * `search`: Due to the complexity of searching for tweets, it gets its own module.
//! * `direct`: Here you can work with a user's Direct Messages, either by loading DMs they've sent
//!   or received, or by sending new ones.
//! * `list`: This module lets you act on lists, from creating and deleting them, adding and
//!   removing users, or loading the posts made by their members.
//! * `media`: This module lets you upload images, GIFs, and videos to Twitter so you can attach
//!   them to tweets.
//!
//! ## Secondary actions
//!
//! These modules still contain direct actions for Twitter, but they can be considered as having
//! more of a helper role than something you might use directly.
//!
//! * `place`: Here are actions that look up physical locations that can be attached to tweets, as
//!   well at the `Place` struct that appears on tweets with locations attached.
//! * `service`: These are some miscellaneous methods that show information about the Twitter
//!   service as a whole, like loading the maximum length of t.co URLs or loading the current Terms
//!   of Service or Privacy Policy.
//!
//! ## Helper structs
//!
//! These modules contain some implementations that wrap some pattern seen in multiple "action"
//! modules.
//!
//! * `cursor`: This contains a helper trait and some helper structs that allow effective cursoring
//!   through certain collections of results from Twitter.
//! * `entities`: Whenever some text can be returned that may contain links, hashtags, media, or
//!   user mentions, its metadata is parsed into something that lives in this module.
//! * `error`: Any interaction with Twitter may result in an error condition, be it from finding a
//!   tweet or user that doesn't exist or the network connection being unavailable. All the error
//!   types are aggregated into an enum in this module.

#![warn(missing_docs)]
#![warn(unused_extern_crates)]
#![warn(unused_qualifications)]

#[macro_use]
mod common;
mod auth;
pub mod cursor;
pub mod direct;
pub mod entities;
pub mod error;
mod links;
pub mod list;
pub mod media;
pub mod place;
pub mod search;
pub mod service;
pub mod stream;
pub mod tweet;
pub mod user;

pub use crate::auth::{
    access_token, authenticate_url, authorize_url, bearer_token, invalidate_bearer, request_token,
    verify_tokens, KeyPair, Token,
};
pub use crate::common::Response;
