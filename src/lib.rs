//! A library for interacting with Twitter.
//!
//! Please see [the repository][] and its enclosed examples for tips on working with this library.
//!
//! [the repository]: https://github.com/QuietMisdreavus/twitter-rs
//!
//! # Quick Start
//!
//! Before you write any code with this library, head over to the [Twitter Application
//! Manager][twitter-apps] to set up an app, so you can have your consumer key and consumer secret.
//! The complete authentication process is outlined on the [Token][] page and given in detail on
//! the individual method pages.
//!
//! [twitter-apps]: https://apps.twitter.com/
//! [Token]: enum.Token.html
//!
//! ## PIN-Based Authentication
//!
//! To sign in as a specific user:
//!
//! ```rust,no_run
//! let con_token = egg_mode::KeyPair::new("consumer key", "consumer secret");
//! // "oob" is needed for PIN-based auth; see docs for `request_token` for more info
//! let request_token = egg_mode::request_token(&con_token, "oob").unwrap();
//! let auth_url = egg_mode::authorize_url(&request_token);
//!
//! // give auth_url to the user, they can sign in to Twitter and accept your app's permissions.
//! // they'll receive a PIN in return, they need to give this to your application
//!
//! let verifier = "123456"; //read the PIN from the user here
//!
//! // note this consumes con_token; if you want to sign in multiple accounts, clone it here
//! let (token, user_id, screen_name) =
//!     egg_mode::access_token(con_token, &request_token, verifier).unwrap();
//!
//! // token can be given to any egg_mode method that asks for a token
//! // user_id and screen_name refer to the user who signed in
//! ```
//!
//! See the [request token][] docs page for more information, or for sign-in options that are
//! easier for websites or apps that can launch a web browser more seamlessly.
//!
//! [request token]: fn.request_token.html
//!
//! ### Shortcut: Pre-Generated Access Token
//!
//! If you only want to sign in as yourself, you can skip the request token authentication flow and
//! instead use the access token key pair given alongside your app keys:
//!
//! ```rust
//! let con_token = egg_mode::KeyPair::new("consumer key", "consumer secret");
//! let access_token = egg_mode::KeyPair::new("access token key", "access token secret");
//! let token = egg_mode::Token::Access {
//!     consumer: con_token,
//!     access: access_token,
//! };
//!
//! // token can be given to any egg_mode method that asks for a token
//! ```
//!
//! ## Bearer Tokens and App-Only Authentication
//!
//! If you can get away with performing requests on behalf of your application as a whole, and not
//! as a specific user, you can use a Bearer token instead. You still need to set up an application
//! as before, but you can use a simpler process instead:
//!
//! ```rust,no_run
//! let con_token = egg_mode::KeyPair::new("consumer key", "consumer secret");
//! let token = egg_mode::bearer_token(&con_token).unwrap();
//!
//! // token can be given to *most* egg_mode methods that ask for a token
//! // for restrictions, see docs for bearer_token
//! ```
//!
//! For more information, see the [bearer token][] docs page.
//!
//! [bearer token]: fn.bearer_token.html
//!
//! # `Response<T>`
//!
//! Every method that calls Twitter and carries rate-limit information wraps its return value in a
//! [`Response`][] struct, that transmits this information to your app. From there, you can handle
//! the rate-limit information to hold off on that kind of request, or simply grab its `response`
//! field to get the output of whatever method you called.
//!
//! [`Response`]: struct.Response.html
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
//! * `text`: Text processing functions to count characters in new tweets and extract links and
//!   hashtags for highlighting and linking.
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

#[macro_use] extern crate hyper;
#[macro_use] extern crate lazy_static;
extern crate hyper_native_tls;
extern crate native_tls;
extern crate url;
extern crate rand;
extern crate ring;
extern crate rustc_serialize;
extern crate mime;
extern crate chrono;
extern crate regex;
extern crate unicode_normalization;

#[macro_use] mod common;
mod auth;
pub mod error;
pub mod user;
pub mod entities;
pub mod cursor;
pub mod tweet;
pub mod search;
pub mod place;
pub mod direct;
pub mod service;
pub mod text;
mod links;

pub use auth::{KeyPair, Token, request_token, authorize_url, authenticate_url,
               access_token, verify_tokens, bearer_token, invalidate_bearer};
pub use common::{Response, ResponseIter, ResponseIterRef, ResponseIterMut, WebResponse};
