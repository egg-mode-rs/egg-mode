// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Types and methods used to authenticate calls to Twitter.
//!
//! # Authenticating Requests With Twitter
//!
//! egg-mode uses a [`Token`] type to represent successfully authenticating with Twitter. The
//! process of obtaining a `Token` is somewhat complicated, and there are a few distinct routes you
//! can take to get one. Which route you take depends on whether you want to only access public
//! data versus wanting to act on behalf of a specific user, and whether you can open a web browser
//! and/or redirect web requests to and from Twitter. Twitter's [Authentication Overview][auth] has
//! the complete information.
//!
//! [`Token`]: enum.Token.html
//! [auth]: https://developer.twitter.com/en/docs/basics/authentication/overview
//!
//! Regardless of which route you take, you need to [register] with Twitter's Developer site and
//! [set up an app][apps] to get a set of keys that represent your code interacting with the
//! Twitter service. These are called the "consumer key" and "consumer secret", and you can store
//! them in a [`KeyPair`] to start the authentication process with egg-mode. At an HTTP request
//! level, these keys are sent with every API call regardless of how you choose to authenticate.
//! (There are other keys given when you register an app, but those will be mentioned below when
//! talking about Access tokens.)
//!
//! [register]: https://developer.twitter.com/en/apply-for-access
//! [apps]: https://developer.twitter.com/en/apps
//! [`KeyPair`]: struct.KeyPair.html
//!
//! There are two kinds of `Tokens` used within egg-mode, representing the two major ways to
//! interact with the Twitter API: Bearer tokens, for accessing public information on Twitter from
//! the point of view of your app itself, and Access tokens, for performing actions or requesting
//! data on behalf of a specific user.
//!
//! ## Bearer Tokens
//!
//! The simplest kind of `Token` you can get is called a "Bearer token".  Bearer tokens are for
//! when you want to perform requests on behalf of your app itself, instead of a specific user.
//! Bearer tokens are the API equivalent of viewing Twitter from a logged-out session. Anything
//! that's already public can be viewed, but things like protected users or the home timeline can't
//! be accessed with bearer tokens. On the other hand, because you don't need to authenticate a
//! user, obtaining a bearer token is relatively simple.
//!
//! If a Bearer token will work for your purposes, use the following steps to get a Token:
//!
//! 1. With the consumer key and secret from Twitter, ask Twitter for the current [Bearer token]
//!    for your application.
//!
//! [Bearer token]: fn.bearer_token.html
//!
//! And... that's it! This Bearer token can be cached and saved for future use. It will not expire
//! until you ask Twitter to [invalidate] the token for you. Otherwise, this token can be used the
//! same way as access tokens below, but with the restrictions mentioned earlier.
//!
//! [invalidate]: fn.invalidate_bearer.html
//!
//! ### Example (Bearer Token)
//!
//! ```rust,no_run
//! # #[tokio::main]
//! # async fn main() {
//! let con_token = egg_mode::KeyPair::new("consumer key", "consumer secret");
//! let token = egg_mode::auth::bearer_token(&con_token).await.unwrap();
//!
//! // token can be given to *most* egg_mode methods that ask for a token
//! // for restrictions, see docs for bearer_token
//! # }
//! ```
//!
//! ## Access Tokens
//!
//! Access tokens are for when you want to perform your requests on behalf of a specific user. This
//! could be for something like posting to their account, sending and receiving direct messages for
//! them, viewing protected accounts they follow, and other actions that only make sense from the
//! perspective from a specific user. Because of the two-fold nature of making sure your requests
//! are signed from your specific *app* and from that specific *user*, the authentication process
//! for access tokens is relatively complicated.
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
//! Before you get too deep into the authentication process, it helps to know a couple things about
//! the app you're writing:
//!
//! * Is your app in an environment where directing users to and from a web page is easy? (e.g. a
//!   website, or a mobile app)
//! * Are you using Twitter authentication as a substitute for user accounts in your own
//!   application, instead of wanting to interact with their Twitter account?
//!
//! Depending on your answer to the first question, you may need to use "PIN-Based Authorization",
//! where the user completes the authentication/authorization process in a separate window and
//! receives a numeric PIN in response that your app can use to complete the authentication
//! process. The alternative to that is the standard OAuth flow, where a web browser is directed to
//! Twitter to complete the login and authorization, then redirected back to your app to receive
//! the access token proper. The way to signal one method or another is by the `callback` parameter
//! to the [access token] request.
//!
//! The second question informs *where* you send the user to authorize your app. Using the "Sign In
//! With Twitter" flow, your app could be able to transparently request another access token
//! without the user needing to accept the connection every time. This is ideal for websites where
//! a "Sign In With Twitter" button could replace a regular login button, using a user's Twitter
//! account in place for regular username/password credentials. To be able to use the "Sign In With
//! Twitter" flow, you must first enable it for your app on Twitter's Application Manager.  Then,
//! for Step 2 of the authentication process, send the user to an [authenticate] URL. If you don't
//! need or want to use the "Sign In With Twitter" process, send the user to an [authorize] URL
//! instead.
//!
//! The primary difference between the different URLs for Step 2 is that an [authenticate] URL
//! allows the above behavior, whereas an [authorize] URL does not require the extra setting in the
//! app manager and always requires the user to re-authorize the app every time they're sent
//! through the authentication process. Since access tokens can be cached and reused indefinitely
//! until the app's access is revoked, you only really need to send the user through the
//! authentication process once.
//!
//! The end result of Step 2 is that your app receives a "verifier" to vouch for the user's
//! acceptance of your app. With PIN-Based Authorization, the user receives a PIN from Twitter that
//! acts as the verifier. With "Sign In With Twitter" and its counterpart, "3-Legged
//! Authorization", the verifier is given as a query parameter to the callback URL given back in
//! Step 1. With this verifier and the original request token, you can combine them with your app's
//! consumer token to get the [access token] that opens up the rest of the Twitter API.
//!
//! ### Example (Access Token)
//!
//! For "PIN-Based Authorization":
//!
//! ```rust,no_run
//! # #[tokio::main]
//! # async fn main() {
//! let con_token = egg_mode::KeyPair::new("consumer key", "consumer secret");
//! // "oob" is needed for PIN-based auth; see docs for `request_token` for more info
//! let request_token = egg_mode::auth::request_token(&con_token, "oob").await.unwrap();
//! let auth_url = egg_mode::auth::authorize_url(&request_token);
//!
//! // give auth_url to the user, they can sign in to Twitter and accept your app's permissions.
//! // they'll receive a PIN in return, they need to give this to your application
//!
//! let verifier = "123456"; //read the PIN from the user here
//!
//! // note this consumes con_token; if you want to sign in multiple accounts, clone it here
//! let (token, user_id, screen_name) =
//!     egg_mode::auth::access_token(con_token, &request_token, verifier).await.unwrap();
//!
//! // token can be given to any egg_mode method that asks for a token
//! // user_id and screen_name refer to the user who signed in
//! # }
//! ```
//!
//! **WARNING**: The consumer token and preset access token mentioned below are as privileged as
//! passwords! If your consumer key pair leaks or is visible to the public, anyone can impersonate
//! your app! If you use a fixed token for your app, it's recommended to set them in separate files
//! and use `include_str!()` (from the standard library), or save them in an environment file and
//! use a library like `dotenv` to load them in, so you can safely exclude them from source
//! control.
//!
//! ### Shortcut: Pre-Generated Access Token
//!
//! If you only want to sign in as yourself, there's a shortcut you can use to get an Access token.
//! When you sign up for an app and get your consumer token, a second key/secret pair are given to
//! you. This "access token" and "access token secret" act as authorization to access your own
//! account with your own code, and can be used to directly construct an egg-mode `Token`:
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
//! For more information on the individual steps of the authentication process, see the
//! documentation for the functions in this module.

use std::borrow::Cow;

use hyper::Method;
use serde::{Serialize, Deserialize};
use serde_json;

use crate::common::*;
use crate::{
    error::{self, Result},
    links,
};

pub(crate) mod raw;

use raw::RequestBuilder;

/// A key/secret pair representing the app that is sending a request or an authorization from a user.
///
/// This type is used as part of the authentication process and to sign API requests afterward. For
/// the most part it's used internally as part of a [`Token`], but at the very beginning of the
/// authentication process, you'll need to manually create one to hold onto your "consumer token"
/// and request a [request token].
///
/// [`Token`]: enum.Token.html
/// [request token]: fn.request_token.html
///
/// For more information, see the [authentication documentation][auth].
///
/// [auth]: index.html
///
/// # Example
///
/// ```rust
/// let con_token = egg_mode::KeyPair::new("consumer key", "consumer token");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPair {
    ///A key used to identify an application or user.
    pub key: Cow<'static, str>,
    ///A private key used to sign messages from an application or user.
    pub secret: Cow<'static, str>,
}

impl KeyPair {
    /// Creates a KeyPair with the given key and secret.
    ///
    /// This can be called with either `&'static str` (a string literal) or `String` for either
    /// parameter.
    pub fn new<K, S>(key: K, secret: S) -> KeyPair
    where
        K: Into<Cow<'static, str>>,
        S: Into<Cow<'static, str>>,
    {
        KeyPair {
            key: key.into(),
            secret: secret.into(),
        }
    }

    /// Internal function to create an empty KeyPair. Not meant to be used from user code.
    fn empty() -> KeyPair {
        KeyPair {
            key: "".into(),
            secret: "".into(),
        }
    }
}

/// A token that can be used to sign requests to Twitter.
///
/// Conceptually, a Token represents your authorization to call the Twitter API. It can either be a
/// [Bearer token], representing a "logged-out" view of Twitter coming from your app itself; or an
/// [Access token], representing a combination of your app's "consumer" key with a specific user
/// granting access for your app to use the Twitter API on their behalf. For more information, see
/// the [authentication documentation][auth].
///
/// [Bearer token]: index.html#bearer-tokens
/// [Access token]: index.html#access-tokens
/// [auth]: index.html
///
/// Once you have obtained a Token of either kind, the keys within may be saved and reused in the
/// future, as long as the access has not been revoked. **Note** that the keys saved in this type
/// work just like a password, and they should be handled with care when you save them! If you
/// believe your keys have been compromised, you can generate a new consumer token in [Twitter's
/// Apps Dashboard][apps], and if you've been using a Bearer token, you should [invalidate] it and
/// generate a new one.
///
/// [apps]: https://developer.twitter.com/en/apps
/// [invalidate]: fn.invalidate_bearer.html
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Token {
    /// An OAuth Access token indicating the request is coming from a specific user.
    Access {
        /// A "consumer" key/secret that represents the application sending the request.
        consumer: KeyPair,
        /// An "access" key/secret that represents the user's authorization of the application.
        access: KeyPair,
    },
    /// An OAuth Bearer token indicating the request is coming from the application itself, not a
    /// particular user.
    Bearer(String),
}

/// With the given consumer KeyPair, ask Twitter for a request KeyPair that can be used to request
/// access to the user's account.
///
/// # Access Token Authentication
///
/// [Authentication overview](index.html)
///
/// 1. **Request Token**: Authenticate your application
/// 2. [Authorize]/[Authenticate]: Authenticate the user
/// 3. [Access Token]: Confirm the authentication with Twitter
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
/// # #[tokio::main]
/// # async fn main() {
/// let con_token = egg_mode::KeyPair::new("consumer key", "consumer token");
/// // for PIN-Based Auth
/// let req_token = egg_mode::auth::request_token(&con_token, "oob").await.unwrap();
/// // for Sign In With Twitter/3-Legged Auth
/// let req_token = egg_mode::auth::request_token(&con_token, "https://myapp.io/auth")
///     .await
///     .unwrap();
/// # }
/// ```
pub async fn request_token<S: Into<String>>(con_token: &KeyPair, callback: S) -> Result<KeyPair> {
    let request = RequestBuilder::new(Method::POST, links::auth::REQUEST_TOKEN)
        .oauth_callback(callback.into())
        .request_keys(con_token, None);

    let (_, body) = raw_request(request).await?;

    let body = std::str::from_utf8(&body).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "stream did not contain valid UTF-8",
        )
    })?;
    let mut key: Option<String> = None;
    let mut secret: Option<String> = None;

    for elem in body.split('&') {
        let mut kv = elem.splitn(2, '=');
        match kv.next() {
            Some("oauth_token") => key = kv.next().map(|s| s.to_string()),
            Some("oauth_token_secret") => secret = kv.next().map(|s| s.to_string()),
            Some(_) => (),
            None => {
                return Err(error::Error::InvalidResponse(
                    "unexpected end of request_token response",
                    None,
                ))
            }
        }
    }

    Ok(KeyPair::new(
        key.ok_or(error::Error::MissingValue("oauth_token"))?,
        secret.ok_or(error::Error::MissingValue("oauth_token_secret"))?,
    ))
}

/// With the given request KeyPair, return a URL that a user can access to accept or reject an
/// authorization request.
///
/// # Access Token Authentication
///
/// [Authentication overview](index.html)
///
/// 1. [Request Token]: Authenticate your application
/// 2. **Authorize**/[Authenticate]: Authenticate the user
/// 3. [Access Token]: Confirm the authentication with Twitter
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
/// [3-legged authorization]: https://developer.twitter.com/en/docs/basics/authentication/oauth-1-0a/obtaining-user-access-tokens
///
/// If you gave the special value `"oob"` to `request_token`, this URL can be directly shown to the
/// user, who can enter it into a separate web browser to complete the authorization. This is
/// called [PIN-based authorization] and it's required for applications that cannot be reached by
/// redirecting a URL from a web browser. When the user loads this URL, they can sign in with
/// Twitter and grant your app access to their account. If they grant this access, they are given a
/// numeric PIN that your app can use as the "verifier" to create the final [access token].
///
/// [Pin-Based authorization]: https://developer.twitter.com/en/docs/basics/authentication/oauth-1-0a/pin-based-oauth
pub fn authorize_url(request_token: &KeyPair) -> String {
    format!(
        "{}?oauth_token={}",
        links::auth::AUTHORIZE,
        request_token.key
    )
}

/// With the given request KeyPair, return a URL to redirect a user to so they can accept or reject
/// an authorization request.
///
/// # Access Token Authentication
///
/// [Authentication overview](index.html)
///
/// 1. [Request Token]: Authenticate your application
/// 2. [Authorize]/ **Authenticate**: Authenticate the user
/// 3. [Access Token]: Confirm the authentication with Twitter
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
/// [Sign In With Twitter]: https://developer.twitter.com/en/docs/basics/authentication/guides/log-in-with-twitter
///
/// If the user is redirected to a callback URL, Twitter will add two query string parameters:
/// `oauth_token`, which contains the `key` from the [request token] used here, and
/// `oauth_verifier`, which contains a verifier string that can be used to create the final [access
/// token].
pub fn authenticate_url(request_token: &KeyPair) -> String {
    format!(
        "{}?oauth_token={}",
        links::auth::AUTHENTICATE,
        request_token.key
    )
}

/// With the given OAuth tokens and verifier, ask Twitter for an access KeyPair that can be used to
/// sign further requests to the Twitter API.
///
/// # Access Token Authentication
///
/// [Authentication overview](index.html)
///
/// 1. [Request Token]: Authenticate your application
/// 2. [Authorize]/[Authenticate]: Authenticate the user
/// 3. **Access Token**: Confirm the authentication with Twitter
///
/// [Request Token]: fn.request_token.html
/// [Authorize]: fn.authorize_url.html
/// [Authenticate]: fn.authenticate_url.html
///
/// # Access Token: Confirm the authentication with Twitter
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
/// The `Future` returned by this function, on success, yields a tuple of three items: The
/// final access token, the ID of the authenticated user, and the screen name of the authenticated
/// user.
pub async fn access_token<S: Into<String>>(
    con_token: KeyPair,
    request_token: &KeyPair,
    verifier: S,
) -> Result<(Token, u64, String)> {
    let request = RequestBuilder::new(Method::POST, links::auth::ACCESS_TOKEN)
        .oauth_verifier(verifier.into())
        .request_keys(&con_token, Some(request_token));

    let (_headers, urlencoded) = raw_request(request).await?;
    let urlencoded = std::str::from_utf8(&urlencoded).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "stream did not contain valid UTF-8",
        )
    })?;

    // TODO deserialize into a struct
    let mut key: Option<String> = None;
    let mut secret: Option<String> = None;
    let mut id: Option<u64> = None;
    let mut username: Option<String> = None;

    for elem in urlencoded.split('&') {
        let mut kv = elem.splitn(2, '=');
        match kv.next() {
            Some("oauth_token") => key = kv.next().map(|s| s.to_string()),
            Some("oauth_token_secret") => secret = kv.next().map(|s| s.to_string()),
            Some("user_id") => id = kv.next().and_then(|s| u64::from_str_radix(s, 10).ok()),
            Some("screen_name") => username = kv.next().map(|s| s.to_string()),
            Some(_) => (),
            None => {
                return Err(error::Error::InvalidResponse(
                    "unexpected end of response in access_token",
                    None,
                ))
            }
        }
    }

    let access_key = key.ok_or(error::Error::MissingValue("oauth_token"))?;
    let access_secret = secret.ok_or(error::Error::MissingValue("oauth_token_secret"))?;

    Ok((
        Token::Access {
            consumer: con_token,
            access: KeyPair::new(access_key, access_secret),
        },
        id.ok_or(error::Error::MissingValue("user_id"))?,
        username.ok_or(error::Error::MissingValue("screen_name"))?,
    ))
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
/// [auth]: https://developer.twitter.com/en/docs/basics/authentication/oauth-2-0/application-only
pub async fn bearer_token(con_token: &KeyPair) -> Result<Token> {
    let content = "application/x-www-form-urlencoded;charset=UTF-8";

    let request = RequestBuilder::new(Method::POST, links::auth::BEARER_TOKEN)
        .with_body("grant_type=client_credentials", content)
        .request_consumer_bearer(con_token);

    let decoded = request_with_json_response::<serde_json::Value>(request).await?;
    let result = decoded
        .get("access_token")
        .and_then(|s| s.as_str())
        .ok_or(error::Error::MissingValue("access_token"))?;

    Ok(Token::Bearer(result.to_owned()))
}

/// Invalidate the given Bearer token using the given consumer KeyPair. Upon success, the future
/// returned by this function yields the Token that was just invalidated.
///
/// For more information about Bearer tokens, see the [authentication overview][auth] and the
/// documentation for the [`bearer_token`] function.
///
/// [auth]: index.html#bearer-tokens
/// [`bearer_token`]: fn.bearer_token.html
///
/// # Panics
///
/// If this function is handed a `Token` that is not a Bearer token, this function will panic.
pub async fn invalidate_bearer(con_token: &KeyPair, token: &Token) -> Result<Token> {
    let token = if let Token::Bearer(ref token) = *token {
        token
    } else {
        panic!("non-bearer token passed to invalidate_bearer");
    };

    let content = "application/x-www-form-urlencoded;charset=UTF-8";

    let request = RequestBuilder::new(Method::POST, links::auth::INVALIDATE_BEARER)
        .with_body(format!("access_token={}", token), content)
        .request_consumer_bearer(con_token);

    let decoded = request_with_json_response::<serde_json::Value>(request).await?;
    let result = decoded
        .get("access_token")
        .and_then(|s| s.as_str())
        .ok_or(error::Error::MissingValue("access_token"))?;

    Ok(Token::Bearer(result.to_owned()))
}

/// If the given tokens are valid, return the user information for the authenticated user.
///
/// If you have cached access tokens, using this method is a convenient way to make sure they're
/// still valid. If the user has revoked access from your app, this function will return an error
/// from Twitter indicating that you don't have access to the user.
pub async fn verify_tokens(token: &Token) -> Result<Response<crate::user::TwitterUser>> {
    let req = get(links::auth::VERIFY_CREDENTIALS, token, None);
    request_with_json_response(req).await
}
