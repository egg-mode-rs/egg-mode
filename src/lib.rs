//! A library for interacting with Twitter.
//!
//! Please see [the repository][] and its enclosed examples for tips on working with this library
//! while it's still in progress.
//!
//! [the repository]: https://github.com/QuietMisdreavus/twitter-rs
//!
//! ## Getting Started
//!
//! The very first thing you'll need to do to get access to the Twitter API is to head to
//! [Twitter's Application Manager][twitter-apps] and create an app. Once you've done that, there
//! are two sets of keys immediately available to you. First are the "consumer key" and "consumer
//! secret", that are used to represent you as the application author when signing requests. These
//! keys are given to every single API call regardless of permission level. Related are an "access
//! token" and "access token secret", that can be used to skip the authentication steps if you're
//! only interacting with your own account or with no account in particular. Generally, if you want
//! to read or write to a particular user's stream, you'll need to request authorization and get an
//! access token to work on their behalf.
//!
//! [twitter-apps]: https://apps.twitter.com/
//!
//! The process to get an access token for a specific user (with this library) has three basic
//! steps:
//!
//! 1. Log your request with Twitter by getting a [request token][].
//! 2. Direct the user to grant permission to your application by sending them to an
//!    [authenticate][] or [authorize][] URL, depending on the nature of your app.
//! 3. Convert the verifier given by the permission request into an [access token][].
//!
//! [request token]: fn.request_token.html
//! [authorize]: fn.authorize_url.html
//! [authenticate]: fn.authenticate_url.html
//! [access token]: fn.access_token.html
//!
//! The process changes some smaller details depending on whether your application is web- or
//! mobile-based, as opposed to desktop-based or in another situation where redirecting users to a
//! web browser and back automatically is difficult or impossible. In the former case where URLs
//! can be used to direct the user into and out of your app, give Twitter a "callback URL" when
//! setting up your request token, and redirect the user to an [authenticate][] URL to grant your
//! app permissions. When the user accepts the request, they will be redirected back to this URL to
//! get back to your app, with the original request token and a verifier given to your app to
//! signify their acceptance.
//!
//! On the other hand, if you can't use the callback URL in this fashion, you can instead use the
//! "PIN-based Auth" version of the flow. In this version, give a "callback URL" of "oob" when
//! setting up your request token, and use an [authorize][] URL. When the user grants the
//! permissions request in this fashion, they are given a numeric PIN that can be given back to
//! your app to use as a verifier.
//!
//! Either way, when you have a "verifier" from either of these methods, you can use your Tokens
//! from earlier in the process with that verifier to request an [access token][]. This access
//! token can then be saved and cached for future use.
//!
//! ### Alternate method: Bearer tokens
//!
//! If your only use for the Twitter API is to look at public data - things like public tweet
//! searches, or viewing public user profiles - you may be interested in Application-only
//! authentication. App-only auth provides a much easier way to authenticate API calls, with the
//! restriction that the Bearer token it provides can only be used for calls that do not need a
//! user context. For example, calls like `tweet::home_timeline`, which act on behalf of the
//! authenticated user, will return an error if used with a Bearer token.
//!
//! If a Bearer token will work for your purposes, use the following steps to get a Token:
//!
//! 1. With the consumer key/secret obtained the same way as above, ask Twitter for the current
//!    [Bearer token] for your application.
//!
//! [Bearer token]: fn.bearer_token.html
//!
//! And... that's it! This Bearer token can be cached and saved for future use. It will not expire
//! until you ask Twitter to [invalidate] the token for you. Otherwise, this token can be used the
//! same way as the [access token] from earlier, but with the restrictions mentioned above.
//!
//! [invalidate]: fn.invalidate_bearer.html
//!
//! ## `Response<T>`
//!
//! Every method that calls Twitter and carries rate-limit information wraps its return value in a
//! [`Response`][] struct, that transmits this information to your app. From there, you can handle
//! the rate-limit information to hold off on that kind of request, or simply grab its `response`
//! field to get the output of whatever method you called.
//!
//! [`Response`]: struct.Response.html
//!
//! ## Modules
//!
//! As there are many actions available in the Twitter API, egg-mode divides them roughly into
//! several modules by their shared purpose. Here's a sort of high-level overview, in rough order
//! from "most important" to "less directly used":
//!
//! ### Primary actions
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
//! ### Secondary actions
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
//! ### Helper structs
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
extern crate url;
extern crate rand;
extern crate crypto;
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
