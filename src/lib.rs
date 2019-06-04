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
//! * This library provides several types which implement the `Future` trait, but does not describe
//!   how to interact with them. The examples use the `block_on_all` method from [tokio]'s runtime
//!   to show a synchronous interaction, but more advanced scenarios are beyond the scope of this
//!   documentation. See the [tokio] documentation for more information.
//! * Twitter tracks API use through "tokens" which are managed by Twitter and processed separately
//!   for each "authenticated user" you wish to connect to your app. egg-mode's [Token]
//!   documentation describes how you can obtain one of these, but each example outside of the
//!   authentication documentation brings in a `Token` "offscreen", to avoid distracting from the
//!   rest of the example.
//!
//! [Token]: enum.Token.html
//! [tokio]: https://tokio.rs
//!
//! To load the profile information of a single user:
//!
//! ```rust,no_run
//! # extern crate egg_mode; extern crate tokio;
//! # use egg_mode::Token;
//! use tokio::runtime::current_thread::block_on_all;
//! # fn main() {
//! # let token: Token = unimplemented!();
//! let rustlang = block_on_all(egg_mode::user::show("rustlang", &token)).unwrap();
//!
//! println!("{} (@{})", rustlang.name, rustlang.screen_name);
//! # }
//! ```
//!
//! To post a new tweet:
//!
//! ```rust,no_run
//! # extern crate egg_mode; extern crate tokio;
//! # use egg_mode::Token;
//! use tokio::runtime::current_thread::block_on_all;
//! use egg_mode::tweet::DraftTweet;
//! # fn main() {
//! # let token: Token = unimplemented!();
//!
//! let post = block_on_all(DraftTweet::new("Hey Twitter!").send(&token)).unwrap();
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
//! `Response` also has IntoIterator implementations and iterator creation methods that echo those
//! on `Vec<T>`, for methods that return Vecs. These methods and iterator types distribute the
//! rate-limit information across each iteration.
//!
//! [`Response`]: struct.Response.html
//!
//! ## `TwitterFuture<'a, T>`
//!
//! Any method that requires a network call will return a handle to the pending network call, in
//! most cases the type [`TwitterFuture`][]. This type (and any other `*Future` in this library)
//! implements the `Future` trait, for use as an asynchronous network call. All `Future`
//! implementations in this library use the `Error` enum as their Error value. For more information
//! on how to use the `Future` trait, check out the [Tokio documentation guides][].
//!
//! In addition, there is also a `FutureResponse` type alias, that corresponds to
//! `TwitterFuture<'a, Response<T>>`, for methods that return rate-limit information.
//!
//! [`TwitterFuture`]: struct.TwitterFuture.html
//! [Tokio documentation guides]: https://tokio.rs/docs/getting-started/tokio/
//!
//! ## Authentication Types/Functions
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

extern crate base64;
extern crate hyper;
#[macro_use]
extern crate lazy_static;
extern crate chrono;
extern crate futures;
extern crate hmac;
#[cfg(feature = "hyper-rustls")]
extern crate hyper_rustls;
#[cfg(feature = "native-tls")]
extern crate hyper_tls;
extern crate mime;
#[cfg(feature = "native-tls")]
extern crate native_tls;
extern crate rand;
extern crate regex;
extern crate serde;
extern crate sha1;
extern crate tokio;
extern crate url;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

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

pub use auth::{
    access_token, authenticate_url, authorize_url, bearer_token, invalidate_bearer, request_token,
    verify_tokens, AuthFuture, KeyPair, Token,
};
pub use common::{
    FutureResponse, Response, ResponseIter, ResponseIterMut, ResponseIterRef, TwitterFuture,
};
