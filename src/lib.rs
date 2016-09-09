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
//! ## `Response<T>`
//!
//! Every method that calls Twitter and carries rate-limit information wraps its return value in a
//! [`Response`][] struct, that transmits this information to your app. From there, you can handle
//! the rate-limit information to hold off on that kind of request, or simply grab its `response`
//! field to get the output of whatever method you called.
//!
//! [`Response`]: struct.Response.html

#![warn(missing_docs)]
#![warn(unused_extern_crates)]
#![warn(unused_qualifications)]

#[macro_use] extern crate hyper;
extern crate url;
extern crate rand;
extern crate crypto;
extern crate rustc_serialize;
extern crate mime;

mod auth;
pub mod error;
pub mod user;
pub mod entities;
pub mod cursor;
pub mod tweet;
pub mod search;
pub mod place;
mod links;
mod common;

pub use auth::{Token, request_token, authorize_url, authenticate_url, access_token, verify_tokens};
pub use common::{Response, ResponseIter, WebResponse};
