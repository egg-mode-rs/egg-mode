// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Types and methods used to authenticate calls to Twitter.
//!
//! This module is meant to be internal, since the OAuth mechanisms are fairly specific to Twitter.
//! Any relevant items for obtaining a Token are re-exported in the crate root. As such, the
//! authentication overview is written on the Token type, rather than in this module docs.

use std;
use std::error::Error;
use std::borrow::Cow;
use std::time::{UNIX_EPOCH, SystemTime};

use base64;
use futures::{Future, Poll, Async};
use hmac::{Hmac, Mac};
use hyper::header::{Authorization, Scheme, ContentType, Basic, Bearer, Headers};
use hyper::{Method, Request};
use mime::Mime;
use rand::{self, Rng};
use serde_json;
use sha_1::Sha1;
use tokio_core::reactor::Handle;
use url::percent_encoding::{EncodeSet, utf8_percent_encode};

use links;
use error;
use common::*;

//the encode sets in the url crate don't quite match what twitter wants, so i'll make up my own
#[derive(Copy, Clone)]
struct TwitterEncodeSet;

impl EncodeSet for TwitterEncodeSet {
    fn contains(&self, byte: u8) -> bool {
        match byte {
            b'a' ... b'z' | b'A' ... b'Z' | b'0' ... b'9'
                | b'-' | b'.' | b'_' | b'~' => false,
            _ => true
        }
    }
}

///Encodes the given string slice for transmission to Twitter.
fn percent_encode(src: &str) -> String {
    utf8_percent_encode(src, TwitterEncodeSet).collect::<String>()
}

///OAuth header set given to Twitter calls.
///
///Since different authorization/authentication calls have various parameters that go into this
///header, they're optionally placed at the end of this header.  On the other hand, `signature` is
///optional so a structured header can be passed to `sign()` for signature.
#[derive(Clone, Debug)]
struct TwitterOAuth {
    consumer_key: String,
    nonce: String,
    signature: Option<String>,
    timestamp: u64,
    token: Option<String>,
    callback: Option<String>,
    verifier: Option<String>,
}

impl std::str::FromStr for TwitterOAuth {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut consumer_key: Option<String> = None;
        let mut nonce: Option<String> = None;
        let mut signature: Option<String> = None;
        let mut timestamp: Option<u64> = None;
        let mut token: Option<String> = None;
        let mut callback: Option<String> = None;
        let mut verifier: Option<String> = None;

        for substr in s.split(',') {
            let mut parts = substr.trim().split('=');
            match parts.next() {
                Some("oauth_consumer_key") => consumer_key = parts.next().map(str::to_string),
                Some("oauth_nonce") => nonce = parts.next().map(str::to_string),
                Some("oauth_signature") => signature = parts.next().map(str::to_string),
                Some("oauth_timestamp") => match parts.next().map(<u64 as std::str::FromStr>::from_str) {
                    Some(Ok(n)) => timestamp = Some(n),
                    Some(Err(e)) => return Err(e.description().to_string()),
                    None => timestamp = None,
                },
                Some("oauth_token") => token = parts.next().map(str::to_string),
                Some("oauth_callback") => callback = parts.next().map(str::to_string),
                Some("oauth_verifier") => verifier = parts.next().map(str::to_string),
                Some(_) => return Err("unexpected OAuth Authorization header field".to_string()),
                None => return Err("unexpected header format".to_string()),
            }
        }

        Ok(TwitterOAuth {
            consumer_key: try!(consumer_key.ok_or("no oauth_consumer_key")),
            nonce: try!(nonce.ok_or("no oauth_nonce")),
            signature: signature,
            timestamp: try!(timestamp.ok_or("no oauth_timestamp")),
            token: token,
            callback: callback,
            verifier: verifier,
        })
    }
}

impl Scheme for TwitterOAuth {
    fn scheme() -> Option<&'static str> {
        Some("OAuth")
    }

    fn fmt_scheme(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        try!(write!(f, "oauth_consumer_key=\"{}\"", percent_encode(&self.consumer_key)));

        try!(write!(f, ", oauth_nonce=\"{}\"", percent_encode(&self.nonce)));

        if let Some(ref signature) = self.signature {
            try!(write!(f, ", oauth_signature=\"{}\"", percent_encode(signature)));
        }

        try!(write!(f, ", oauth_signature_method=\"{}\"", percent_encode("HMAC-SHA1")));

        try!(write!(f, ", oauth_timestamp=\"{}\"", self.timestamp));

        if let Some(ref token) = self.token {
            try!(write!(f, ", oauth_token=\"{}\"", percent_encode(token)));
        }

        try!(write!(f, ", oauth_version=\"{}\"", "1.0"));

        if let Some(ref callback) = self.callback {
            try!(write!(f, ", oauth_callback=\"{}\"", percent_encode(callback)));
        }

        if let Some(ref verifier) = self.verifier {
            try!(write!(f, ", oauth_verifier=\"{}\"", percent_encode(verifier)));
        }

        Ok(())
    }
}

/// A key/secret pair representing an OAuth token.
///
/// This struct is used as part of the authentication process. You'll need to manually create at
/// least one of these, to hold onto your consumer token.
///
/// For more information, see the documentation for [Tokens][].
///
/// [Tokens]: enum.Token.html
///
/// # Example
///
/// ```rust
/// let con_token = egg_mode::KeyPair::new("consumer key", "consumer token");
/// ```
#[derive(Debug, Clone)]
pub struct KeyPair {
    ///A key used to identify an application or user.
    pub key: Cow<'static, str>,
    ///A private key used to sign messages from an application or user.
    pub secret: Cow<'static, str>,
}

impl KeyPair {
    ///Creates a KeyPair with the given key and secret.
    ///
    ///This can be called with either `&'static str` (a string literal) or `String` for either
    ///parameter.
    pub fn new<K, S>(key: K, secret: S) -> KeyPair
        where K: Into<Cow<'static, str>>,
              S: Into<Cow<'static, str>>
    {
        KeyPair {
            key: key.into(),
            secret: secret.into(),
        }
    }
}

/// A token that can be used to sign requests to Twitter.
///
/// # Authenticating Requests With Twitter
///
/// A Token is given at the end of the authentication process, and is what you use to authenticate
/// every other call you make with Twitter. The process is different depending on whether you're
/// wanting to operate on behalf of a user, and on how easily your application can open a web
/// browser and/or redirect web requests to and from Twitter. For more information, see Twitter's
/// [OAuth documentation overview][OAuth].
///
/// [OAuth]: https://dev.twitter.com/oauth/overview
///
/// The very first thing you'll need to do to get access to the Twitter API is to head to
/// [Twitter's Application Manager][twitter-apps] and create an app. Once you've done that, there
/// are two sets of keys immediately available to you. First are the "consumer key" and "consumer
/// secret", that are used to represent you as the application author when signing requests. These
/// keys are given to every single API call regardless of permission level. Related are an "access
/// token" and "access token secret", that can be used to skip the authentication steps if you're
/// only interacting with your own account or with no account in particular. Generally, if you want
/// to read or write to a particular user's stream, you'll need to request authorization and get an
/// access token to work on their behalf.
///
/// [twitter-apps]: https://apps.twitter.com/
///
/// ## Access Tokens
///
/// Access tokens are for when you want to perform your requests on behalf of a specific user. This
/// could be for something like posting to their account, reading their home timeline, viewing
/// protected accounts they follow, and other actions that only make sense from the perspective
/// from a specific user. Because of the two-fold nature of making sure your requests are signed
/// from your specific *app* and from that specific *user*, the authentication process for access
/// tokens is fairly complicated.
///
/// The process to get an access token for a specific user (with this library) has three basic
/// steps:
///
/// 1. Log your request with Twitter by getting a [request token][].
/// 2. Direct the user to grant permission to your application by sending them to an
///    [authenticate][] or [authorize][] URL, depending on the nature of your app.
/// 3. Convert the verifier given by the permission request into an [access token][].
///
/// [request token]: fn.request_token.html
/// [authorize]: fn.authorize_url.html
/// [authenticate]: fn.authenticate_url.html
/// [access token]: fn.access_token.html
///
/// Before you get too deep into the authentication process, it helps to know a couple things about
/// the app you're writing:
///
/// * Is your app in an environment where directing users to and from a web page is easy? (e.g. a
///   website, or a mobile app)
/// * Are you using Twitter authentication as a substitute for user accounts, instead of just to
///   interact with their Twitter account?
///
/// Depending on your answer to the first question, you may need to use "PIN-Based Authorization",
/// where the user completes the authentication/authorization process in a separate window and
/// receives a numeric PIN in response that your app can use to complete the authentication
/// process. The alternative to that is the standard OAuth flow, where a web browser is directed to
/// Twitter to complete the login and authorization, then redirected back to your app to receive
/// the access token proper. The way to signal one method or another is by the `callback` parameter
/// to the [access token] request.
///
/// The second question informs *where* you send the user to authorize your app. Using the "Sign In
/// With Twitter" flow, your app could be able to transparently request another access token
/// without the user needing to accept the connection every time. This is ideal for websites where
/// a "Sign In With Twitter" button could replace a regular login button, instead using a user's
/// Twitter account in place for regular username/password credentials. To be able to use the "Sign
/// In With Twitter" flow, you must first enable it for your app on Twitter's Application Manager.
/// Then, for Step 2 of the authentication process, send the user to an [authenticate] URL.
///
/// The primary difference between the different URLs for Step 2 is that an [authenticate] URL
/// allows the above behavior, whereas an [authorize] URL does not require the extra setting in the
/// app manager and always requires the user to re-authorize the app every time they're sent
/// through the authentication process. As access tokens can be cached indefinitely until the app's
/// access is revoked, this is not necessarily as onerous as it sounds.
///
/// The end result of Step 2 is that your app receives a "verifier" to vouch for the user's
/// acceptance of your app. With PIN-Based Authorization, the user receives a PIN from Twitter that
/// acts as the verifier. With "Sign In With Twitter" and its counterpart, "3-Legged
/// Authorization", the verifier is given as a query parameter to the callback URL given back in
/// Step 1. With this verifier and the original request token, you can combine them with your app's
/// consumer token to get the [access token] that opens up the rest of the Twitter API.
///
/// ### Example (Access Token)
///
/// For "PIN-Based Authorization":
///
/// ```rust,no_run
/// # extern crate egg_mode; extern crate tokio_core;
/// # use tokio_core::reactor::{Core, Handle};
/// # fn main() {
/// # let (mut core, handle): (Core, Handle) = unimplemented!();
/// let con_token = egg_mode::KeyPair::new("consumer key", "consumer secret");
/// // "oob" is needed for PIN-based auth; see docs for `request_token` for more info
/// let request_token = core.run(egg_mode::request_token(&con_token, "oob", &handle)).unwrap();
/// let auth_url = egg_mode::authorize_url(&request_token);
///
/// // give auth_url to the user, they can sign in to Twitter and accept your app's permissions.
/// // they'll receive a PIN in return, they need to give this to your application
///
/// let verifier = "123456"; //read the PIN from the user here
///
/// // note this consumes con_token; if you want to sign in multiple accounts, clone it here
/// let (token, user_id, screen_name) =
///     core.run(egg_mode::access_token(con_token, &request_token, verifier, &handle)).unwrap();
///
/// // token can be given to any egg_mode method that asks for a token
/// // user_id and screen_name refer to the user who signed in
/// # }
/// ```
///
/// **WARNING**: The consumer token and preset access token mentioned below are as privileged as
/// passwords! If your consumer key pair leaks or is visible to the public, anyone can impersonate
/// your app! If you use a fixed token for your app, it's recommended to set them in separate files
/// and use `include_str!()` (from the standard library) to load them in, so you can safely exclude
/// them from source control.
///
/// ### Shortcut: Pre-Generated Access Token
///
/// If you only want to sign in as yourself, you can skip the request token authentication flow
/// entirely and instead use the access token key pair given alongside your app keys:
///
/// ```rust
/// let con_token = egg_mode::KeyPair::new("consumer key", "consumer secret");
/// let access_token = egg_mode::KeyPair::new("access token key", "access token secret");
/// let token = egg_mode::Token::Access {
///     consumer: con_token,
///     access: access_token,
/// };
///
/// // token can be given to any egg_mode method that asks for a token
/// ```
///
/// ## Bearer Tokens
///
/// Bearer tokens are for when you want to perform requests on behalf of your app itself, instead
/// of a specific user. Bearer tokens are the API equivalent of viewing Twitter from a logged-out
/// session. Anything that's already public can be viewed, but things like protected users or the
/// home timeline can't be accessed with bearer tokens. On the other hand, because you don't need
/// to authenticate a user, obtaining a bearer token is relatively simple.
///
/// If a Bearer token will work for your purposes, use the following steps to get a Token:
///
/// 1. With the consumer key/secret obtained the same way as above, ask Twitter for the current
///    [Bearer token] for your application.
///
/// [Bearer token]: fn.bearer_token.html
///
/// And... that's it! This Bearer token can be cached and saved for future use. It will not expire
/// until you ask Twitter to [invalidate] the token for you. Otherwise, this token can be used the
/// same way as the [access token] from earlier, but with the restrictions mentioned above.
///
/// [invalidate]: fn.invalidate_bearer.html
///
/// ### Example (Bearer Token)
///
/// ```rust,no_run
/// # extern crate egg_mode; extern crate tokio_core;
/// # use tokio_core::reactor::{Core, Handle};
/// # fn main() {
/// # let (mut core, handle): (Core, Handle) = unimplemented!();
/// let con_token = egg_mode::KeyPair::new("consumer key", "consumer secret");
/// let token = core.run(egg_mode::bearer_token(&con_token, &handle)).unwrap();
///
/// // token can be given to *most* egg_mode methods that ask for a token
/// // for restrictions, see docs for bearer_token
/// # }
/// ```
#[derive(Debug, Clone)]
pub enum Token {
    ///An OAuth Access token indicating the request is coming from a specific user.
    Access {
        ///A "consumer" key/secret that represents the application sending the request.
        consumer: KeyPair,
        ///An "access" key/secret that represents the user's authorization of the application.
        access: KeyPair,
    },
    ///An OAuth Bearer token indicating the request is coming from the application itself, not a
    ///particular user.
    Bearer(String),
}

///With the given OAuth header and method parameters, create an OAuth signature and return the
///header with the signature inline.
fn sign(header: TwitterOAuth,
        method: Method,
        uri: &str,
        params: Option<&ParamList>,
        con_token: &KeyPair,
        access_token: Option<&KeyPair>) -> TwitterOAuth {
    let query_string = {
        let mut sig_params = params.cloned().unwrap_or_default();

        add_param(&mut sig_params, "oauth_consumer_key", header.consumer_key.as_str());
        add_param(&mut sig_params, "oauth_nonce", header.nonce.as_str());
        add_param(&mut sig_params, "oauth_signature_method", "HMAC-SHA1");
        add_param(&mut sig_params, "oauth_timestamp", format!("{}", header.timestamp));
        add_param(&mut sig_params, "oauth_version", "1.0");

        if let Some(ref token) = header.token {
            add_param(&mut sig_params, "oauth_token", token.as_str());
        }

        if let Some(ref callback) = header.callback {
            add_param(&mut sig_params, "oauth_callback", callback.as_str());
        }

        if let Some(ref verifier) = header.verifier {
            add_param(&mut sig_params, "oauth_verifier", verifier.as_str());
        }

        let mut query = sig_params.iter()
                                  .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
                                  .collect::<Vec<_>>();
        query.sort();

        query.join("&")
    };

    let base_str = format!("{}&{}&{}",
                           percent_encode(method.as_ref()),
                           percent_encode(uri),
                           percent_encode(&query_string));
    let key = format!("{}&{}",
                      percent_encode(&con_token.secret),
                      percent_encode(&access_token.unwrap_or(&KeyPair::new("", "")).secret));

    let mut digest = Hmac::<Sha1>::new(key.as_bytes());
    digest.input(base_str.as_bytes());

    let config = base64::Config::new(
        base64::CharacterSet::Standard,
        true,
        true,
        // TODO do we want line-wrapping?
        base64::LineWrap::NoWrap
    );

    let signature = Some(base64::encode_config(digest.result().code(), config));

    TwitterOAuth {signature, ..header}
}

///With the given method parameters, return a signed OAuth header.
fn get_header(method: Method,
              uri: &str,
              con_token: &KeyPair,
              access_token: Option<&KeyPair>,
              callback: Option<String>,
              verifier: Option<String>,
              params: Option<&ParamList>) -> TwitterOAuth {
    let now_s = match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(dur) => dur,
        Err(err) => err.duration(),
    }.as_secs();
    let header = TwitterOAuth {
        consumer_key: con_token.key.to_string(),
        nonce: rand::thread_rng().gen_ascii_chars().take(32).collect::<String>(),
        signature: None,
        timestamp: now_s,
        token: access_token.map(|tok| tok.key.to_string()),
        callback: callback,
        verifier: verifier,
    };

    sign(header, method, uri, params, con_token, access_token)
}

fn bearer_request(con_token: &KeyPair) -> Basic {
    Basic {
        username: percent_encode(&con_token.key),
        password: Some(percent_encode(&con_token.secret)),
    }
}

/// Assemble a signed GET request to the given URL with the given parameters.
pub fn get(uri: &str,
           token: &Token,
           params: Option<&ParamList>) -> Request {
    let full_url = if let Some(p) = params {
        let query = p.iter()
                     .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
                     .collect::<Vec<_>>()
                     .join("&");

        format!("{}?{}", uri, query)
    } else {
        uri.to_string()
    };

    let mut request: Request = Request::new(Method::Get, full_url.parse().unwrap());
    match *token {
        Token::Access {
            consumer: ref con_token,
            access: ref access_token,
        } => {
            let header = get_header(Method::Get, uri, con_token, Some(access_token),
                                    None, None, params);
            request.headers_mut().set(Authorization(header));
        },
        Token::Bearer(ref token) => {
            request.headers_mut().set(Authorization(Bearer { token: token.clone() }));
        },
    }

    request
}

/// Assemble a signed POST request to the given URL with the given parameters.
pub fn post(uri: &str,
            token: &Token,
            params: Option<&ParamList>) -> Request {
    let content: Mime = "application/x-www-form-urlencoded".parse().unwrap();
    let body = if let Some(p) = params {
        p.iter()
         .map(|(k, v)| format!("{}={}", k, percent_encode(v)))
         .collect::<Vec<_>>()
         .join("&")
    } else {
        "".to_string()
    };

    let mut request: Request = Request::new(Method::Post, uri.parse().unwrap());
    request.set_body(body);
    request.headers_mut().set(ContentType(content));

    match *token {
        Token::Access {
            consumer: ref con_token,
            access: ref access_token,
        } => {
            let header = get_header(Method::Post, uri, con_token, Some(access_token),
                                    None, None, params);

            request.headers_mut().set(Authorization(header));
        },
        Token::Bearer(ref token) => {
            request.headers_mut().set(Authorization(Bearer { token: token.clone() }));
        },
    }

    request
}

/// Assemble a signed POST request to the given URL with the given JSON body.
pub fn post_json(uri: &str, token: &Token, body: &serde_json::Value) -> Request {
    let content: Mime = "application/json; charset=UTF-8".parse().unwrap();
    let body = body.to_string();

    let mut request: Request = Request::new(Method::Post, uri.parse().unwrap());
    request.set_body(body);
    request.headers_mut().set(ContentType(content));

    match *token {
        Token::Access {
            consumer: ref con_token,
            access: ref access_token,
        } => {
            let header = get_header(Method::Post, uri, con_token, Some(access_token),
                                    None, None, None);

            request.headers_mut().set(Authorization(header));
        },
        Token::Bearer(ref token) => {
            request.headers_mut().set(Authorization(Bearer { token: token.clone() }));
        },
    }

    request
}

/// With the given consumer KeyPair, ask Twitter for a request KeyPair that can be used to request
/// access to the user's account.
///
/// # Access Token Authentication
///
/// [Authentication overview](enum.Token.html)
///
/// 1. **Request Token**: Authenticate your application
/// 2. [Authorize]/[Authenticate]: Authenticate the user
/// 3. [Access Token]: Combine the authentication
///
/// [Authorize]: fn.authorize_url.html
/// [Authenticate]: fn.authenticate_url.html
/// [Access Token]: fn.access_token.html
///
/// # Request Token: Authenticate your application
///
/// To begin the authentication process, first log your request with Twitter by authenticating your
/// application by itself. This "request token" is used in later steps to match all the requests to
/// the same authentication attempt.
///
/// The parameter `callback` is used differently based on how your program is set up, and which
/// authentication process you'd like to use. For applications where directing users to and from
/// another web page is difficult, you can use the special value `"oob"` to indicate that you would
/// like to use PIN-Based Authentication.
///
/// Web-based applications and those that can handle web redirects transparently can instead supply
/// a callback URL for that parameter. When the user completes the sign-in and authentication
/// process, they will be directed to the provided URL with the information necessary to complete
/// the authentication process. Depending on which Step 2 URL you use and whether you've enabled it
/// for your app, this is called "Sign In With Twitter" or "3-Legged Authorization".
///
/// With this Request Token, you can assemble an [Authorize] or [Authenticate] URL that will allow
/// the user to log in with Twitter and allow your app access to their account. See the
/// Authentication Overview for more details, but the short version is that you want to use
/// [Authenticate] for "Sign In With Twitter" functionality, and [Authorize] if not.
///
/// # Examples
///
/// ```rust,no_run
/// # extern crate egg_mode; extern crate tokio_core;
/// # use tokio_core::reactor::{Core, Handle};
/// # fn main() {
/// # let (mut core, handle): (Core, Handle) = unimplemented!();
/// let con_token = egg_mode::KeyPair::new("consumer key", "consumer token");
/// // for PIN-Based Auth
/// let req_token = core.run(egg_mode::request_token(&con_token, "oob", &handle)).unwrap();
/// // for Sign In With Twitter/3-Legged Auth
/// let req_token = core.run(egg_mode::request_token(&con_token,
///                                                  "https://myapp.io/auth",
///                                                  &handle)).unwrap();
/// # }
/// ```
pub fn request_token<S: Into<String>>(con_token: &KeyPair, callback: S, handle: &Handle)
    -> TwitterFuture<KeyPair>
{
    let header = get_header(Method::Post, links::auth::REQUEST_TOKEN,
                            con_token, None, Some(callback.into()), None, None);

    let mut request = Request::new(Method::Post, links::auth::REQUEST_TOKEN.parse().unwrap());
    request.headers_mut().set(Authorization(header));

    fn parse_tok(full_resp: String, _: &Headers) -> Result<KeyPair, error::Error> {
        let mut key: Option<String> = None;
        let mut secret: Option<String> = None;

        for elem in full_resp.split('&') {
            let mut kv = elem.splitn(2, '=');
            match kv.next() {
                Some("oauth_token") => key = kv.next().map(|s| s.to_string()),
                Some("oauth_token_secret") => secret = kv.next().map(|s| s.to_string()),
                Some(_) => (),
                None =>
                    return Err(
                        error::Error::InvalidResponse(
                            "unexpected end of request_token response", None
                        )
                    ),
            }
        }

        Ok(KeyPair::new(try!(key.ok_or(error::Error::MissingValue("oauth_token"))),
                        try!(secret.ok_or(error::Error::MissingValue("oauth_token_secret")))))
    }

    make_future(handle, request, parse_tok)
}

/// With the given request KeyPair, return a URL that a user can access to accept or reject an
/// authorization request.
///
/// # Access Token Authentication
///
/// [Authentication overview](enum.Token.html)
///
/// 1. [Request Token]: Authenticate your application
/// 2. **Authorize**/[Authenticate]: Authenticate the user
/// 3. [Access Token]: Combine the authentication
///
/// [Request Token]: fn.request_token.html
/// [Authenticate]: fn.authenticate_url.html
/// [Access Token]: fn.access_token.html
///
/// # Authorize: Authenticate the user
///
/// This function is part of the step of authenticating a user with Twitter so they can authorize
/// your application to access their account. This function generates a URL with the given request
/// token that you must give to the user. What happens with this URL depends on what you used as
/// the `callback` parameter for `request_token`.
///
/// If you gave a callback URL to `request_token`, Twitter will redirect the user to that URL after
/// they log in and accept your app's permissions. There will be two query string parameters added
/// to the URL for this redirect: `oauth_token`, which contains the `key` from the [request token]
/// used here, and `oauth_verifier`, which contains a verifier string that can be used to create
/// the final [access token]. Note that if this URL is used instead of [Authenticate], the user
/// will need to accept the app's connection each time, even if they have connected the app
/// previously and have not revoked the app's permissions. This process is called [3-legged
/// authorization]. If you would like the user to transparently be redirected without confirmation
/// if they've already accepted the connection, see the docs for [Authenticate] to read about "Sign
/// In With Twitter".
///
/// [3-legged authorization]: https://dev.twitter.com/oauth/3-legged
///
/// If you gave the special value `"oob"` to `request_token`, this URL can be directly shown to the
/// user, who can enter it into a separate web browser to complete the authorization. This is
/// called [PIN-based authorization] and it's required for applications that cannot be reached by
/// redirecting a URL from a web browser. When the user loads this URL, they can sign in with
/// Twitter and grant your app access to their account. If they grant this access, they are given a
/// numeric PIN that your app can use as the "verifier" to create the final [access token].
///
/// [Pin-Based authorization]: https://dev.twitter.com/oauth/pin-based
pub fn authorize_url(request_token: &KeyPair) -> String {
    format!("{}?oauth_token={}", links::auth::AUTHORIZE, request_token.key)
}

/// With the given request KeyPair, return a URL to redirect a user to so they can accept or reject
/// an authorization request.
///
/// # Access Token Authentication
///
/// [Authentication overview](enum.Token.html)
///
/// 1. [Request Token]: Authenticate your application
/// 2. [Authorize]/ **Authenticate**: Authenticate the user
/// 3. [Access Token]: Combine the authentication
///
/// [Request Token]: fn.request_token.html
/// [Authorize]: fn.authorize_url.html
/// [Access Token]: fn.access_token.html
///
/// # Authenticate: Authenticate the user (with Sign In To Twitter)
///
/// This function is part of the step of authenticating a user with Twitter so they can authorize
/// your application to access their account. This function generates a URL with the given request
/// token that you must give to the user.
///
/// The URL returned by this function acts the same as the [Authorize] URL, with one exception: If
/// you have "[Sign In With Twitter]" enabled for your app, the user does not need to re-accept the
/// app's connection if they've accepted it previously. If they're already logged in to Twitter,
/// and have already accepted your app's access, they won't even see the redirect through Twitter.
/// Twitter will immediately redirect the user to the `callback` URL given to the [request token].
///
/// [Sign In With Twitter]: https://dev.twitter.com/web/sign-in/implementing
///
/// If the user is redirected to a callback URL, Twitter will add two query string parameters:
/// `oauth_token`, which contains the `key` from the [request token] used here, and
/// `oauth_verifier`, which contains a verifier string that can be used to create the final [access
/// token].
pub fn authenticate_url(request_token: &KeyPair) -> String {
    format!("{}?oauth_token={}", links::auth::AUTHENTICATE, request_token.key)
}

/// With the given OAuth tokens and verifier, ask Twitter for an access KeyPair that can be used to
/// sign further requests to the Twitter API.
///
/// # Access Token Authentication
///
/// [Authentication overview](enum.Token.html)
///
/// 1. [Request Token]: Authenticate your application
/// 2. [Authorize]/[Authenticate]: Authenticate the user
/// 3. **Access Token**: Combine the authentication
///
/// [Request Token]: fn.request_token.html
/// [Authorize]: fn.authorize_url.html
/// [Authenticate]: fn.authenticate_url.html
///
/// # Access Token: Combine the app and user authentication
///
/// This is the final step in authenticating a user account to use your app. With this method, you
/// combine the consumer `KeyPair` that represents your app, the [request token] that represents
/// the session, and the "verifier" that represents the user's credentials and their acceptance of
/// your app's access.
///
/// The `verifier` parameter comes from the Step 2 process you used. For PIN-Based Authorization,
/// the verifier is the PIN returned to the user after they sign in. For "Sign In With Twitter" and
/// 3-Legged Authorization, the verifier is the string passed by twitter to your app through the
/// `oauth_verifier` query string parameter. For more information, see the documentation for the
/// [Authorize] URL function.
///
/// Note that this function consumes `con_token`, because it is inserted into the `Token` that is
/// returned. If you would like to use the consumer token to authenticate multiple accounts in the
/// same session, clone the `KeyPair` when passing it into this function.
///
/// The `AuthFuture` returned by this function, on success, yields a tuple of three items: The
/// final access token, the ID of the authenticated user, and the screen name of the authenticated
/// user.
pub fn access_token<S: Into<String>>(con_token: KeyPair,
                                     request_token: &KeyPair,
                                     verifier: S,
                                     handle: &Handle)
    -> AuthFuture
{
    let header = get_header(Method::Post, links::auth::ACCESS_TOKEN,
                            &con_token, Some(request_token), None, Some(verifier.into()), None);
    let mut request = Request::new(Method::Post, links::auth::ACCESS_TOKEN.parse().unwrap());
    request.headers_mut().set(Authorization(header));

    AuthFuture {
        con_token: Some(con_token),
        loader: make_raw_future(handle, request),
    }
}

/// `Future` which yields an access token when it finishes.
///
/// See the docs for [`access_token`][] for more details.
///
/// [`access_token`]: fn.access_token.html
///
/// The `Future` implementation yields a tuple of three items upon success: The final access token,
/// the ID of the authenticated user, and the screen name of the authenticated user.
#[must_use = "futures do nothing unless polled"]
pub struct AuthFuture {
    con_token: Option<KeyPair>,
    loader: RawFuture,
}

impl Future for AuthFuture {
    type Item = (Token, u64, String);
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let full_resp = match self.loader.poll() {
            Err(e) => return Err(e),
            Ok(Async::NotReady) => return Ok(Async::NotReady),
            Ok(Async::Ready(resp)) => resp,
        };

        if let Some(con_token) = self.con_token.take() {
            let mut key: Option<String> = None;
            let mut secret: Option<String> = None;
            let mut id: Option<u64> = None;
            let mut username: Option<String> = None;

            for elem in full_resp.split('&') {
                let mut kv = elem.splitn(2, '=');
                match kv.next() {
                    Some("oauth_token") => key = kv.next().map(|s| s.to_string()),
                    Some("oauth_token_secret") => secret = kv.next().map(|s| s.to_string()),
                    Some("user_id") => id = kv.next().and_then(|s| u64::from_str_radix(s, 10).ok()),
                    Some("screen_name") => username = kv.next().map(|s| s.to_string()),
                    Some(_) => (),
                    None => return Err(
                        error::Error::InvalidResponse(
                            "unexpected end of response in access_token", None)
                    ),
                }
            }

            let access_key = try!(key.ok_or(error::Error::MissingValue("oauth_token")));
            let access_secret = try!(secret.ok_or(error::Error::MissingValue("oauth_token_secret")));

            Ok(Async::Ready((Token::Access {
                    consumer: con_token,
                    access: KeyPair::new(access_key, access_secret),
                },
                try!(id.ok_or(error::Error::MissingValue("user_id"))),
                try!(username.ok_or(error::Error::MissingValue("screen_name"))))))
        } else {
            Err(error::Error::FutureAlreadyCompleted)
        }
    }
}

/// With the given consumer KeyPair, request the current Bearer token to perform Application-only
/// authentication.
///
/// If you don't need to use the Twitter API to perform actions on or with specific users, app-only
/// auth provides a much easier way to authenticate with the Twitter API. The Token given by this
/// function can be used to authenticate requests as if there were coming from your app itself.
/// This comes with an important restriction, though: any request that requires a user context -
/// direct messages, viewing protected user profiles, functions like `tweet::home_timeline` that
/// operate in terms of the authenticated user - will not work with just a Bearer token. Attempts
/// to perform those actions will return an authentication error.
///
/// Other things to note about Bearer tokens:
///
/// - Bearer tokens have a higher rate limit for the methods they can be used on, compared to
///   regular Access tokens.
/// - The bearer token returned by Twitter is the same token each time you call it. It can be
///   cached and reused as long as you need it.
/// - Since a Bearer token can be used to directly authenticate calls to Twitter, it should be
///   treated with the same sensitivity as a password. If you believe your Bearer token to be
///   compromised, call [`invalidate_bearer`] with your consumer KeyPair and the Bearer token you
///   need to invalidate.  This will cause Twitter to generate a new Bearer token for your
///   application, which will be returned the next time you call this function.
///
/// [`invalidate_bearer`]: fn.invalidate_bearer.html
///
/// For more information, see the Twitter documentation on [Application-only authentication][auth].
///
/// [auth]: https://dev.twitter.com/oauth/application-only
pub fn bearer_token(con_token: &KeyPair, handle: &Handle)
    -> TwitterFuture<Token>
{
    let content: Mime = "application/x-www-form-urlencoded;charset=UTF-8".parse().unwrap();

    let auth_header = bearer_request(con_token);
    let mut request = Request::new(Method::Post, links::auth::BEARER_TOKEN.parse().unwrap());
    request.headers_mut().set(Authorization(auth_header));
    request.headers_mut().set(ContentType(content));
    request.set_body("grant_type=client_credentials");

    fn parse_tok(full_resp: String, _: &Headers) -> Result<Token, error::Error> {
        let decoded: serde_json::Value = try!(serde_json::from_str(&full_resp));
        let result = try!(decoded.get("access_token")
                                 .and_then(|s| s.as_str())
                                 .ok_or(error::Error::MissingValue("access_token")));

        Ok(Token::Bearer(result.to_owned()))
    }

    make_future(handle, request, parse_tok)
}

/// Invalidate the given Bearer token using the given consumer KeyPair. Upon success, the future
/// returned by this function yields the Token that was just invalidated.
///
/// # Panics
///
/// If this function is handed a `Token` that is not a Bearer token, this function will panic.
pub fn invalidate_bearer(handle: &Handle, con_token: &KeyPair, token: &Token)
    -> TwitterFuture<Token>
{
    let token = if let Token::Bearer(ref token) = *token {
        token
    } else {
        panic!("non-bearer token passed to invalidate_bearer");
    };

    let content: Mime = "application/x-www-form-urlencoded;charset=UTF-8".parse().unwrap();

    let auth_header = bearer_request(con_token);
    let mut request = Request::new(Method::Post, links::auth::INVALIDATE_BEARER.parse().unwrap());
    request.headers_mut().set(Authorization(auth_header));
    request.headers_mut().set(ContentType(content));
    request.set_body(format!("access_token={}", token));

    fn parse_tok(full_resp: String, _: &Headers) -> Result<Token, error::Error> {
        let decoded: serde_json::Value = try!(serde_json::from_str(&full_resp));
        let result = try!(decoded.get("access_token")
                                 .and_then(|s| s.as_str())
                                 .ok_or(error::Error::MissingValue("access_token")));

        Ok(Token::Bearer(result.to_owned()))
    }

    make_future(handle, request, parse_tok)
}

/// If the given tokens are valid, return the user information for the authenticated user.
///
/// If you have cached access tokens, using this method is a convenient way to make sure they're
/// still valid. If the user has revoked access from your app, this function will return an error
/// from Twitter indicating that you don't have access to the user.
pub fn verify_tokens(token: &Token, handle: &Handle)
    -> FutureResponse<::user::TwitterUser>
{
    let req = get(links::auth::VERIFY_CREDENTIALS, token, None);

    make_parsed_future_serde(handle, req)
}

#[cfg(test)]
mod tests {
    use super::bearer_request;
    use hyper::header::Authorization;

    #[test]
    fn bearer_header() {
        let con_key = "xvz1evFS4wEEPTGEFPHBog";
        let con_secret = "L8qq9PZyRg6ieKGEKhZolGC0vJWLw8iEJ88DRdyOg";
        let con_token = super::KeyPair::new(con_key, con_secret);

        let header = Authorization(bearer_request(&con_token));

        let output = header.to_string();

        assert_eq!(output, "Basic eHZ6MWV2RlM0d0VFUFRHRUZQSEJvZzpMOHFxOVBaeVJnNmllS0dFS2hab2xHQzB2SldMdzhpRUo4OERSZHlPZw==");
    }
}
