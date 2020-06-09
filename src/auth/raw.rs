// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Internal mechanisms for the `auth` module.

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use base64;
use hmac::{Hmac, Mac};
use hyper::header::{AUTHORIZATION, CONTENT_TYPE};
use hyper::{Body, Method, Request};
use percent_encoding::{utf8_percent_encode, AsciiSet, PercentEncode};
use rand::{self, Rng};
use sha1::Sha1;

use crate::common::*;

use super::{Token, KeyPair};

/// Percent-encodes the given string based on the Twitter API specification.
///
/// Twitter bases its encoding scheme on RFC 3986, Section 2.1. They describe the process in full
/// [in their documentation][twitter-percent], but the process can be summarized by saying that
/// every *byte* that is not an ASCII number or letter, or the ASCII characters `-`, `.`, `_`, or
/// `~` must be replaced with a percent sign (`%`) and the byte value in hexadecimal.
///
/// [twitter-percent]: https://developer.twitter.com/en/docs/basics/authentication/oauth-1-0a/percent-encoding-parameters
///
/// When this function was originally implemented, the `percent_encoding` crate did not have an
/// encoding set that matched this, so it was recreated here.
pub fn percent_encode(src: &str) -> PercentEncode {
    lazy_static::lazy_static! {
        static ref ENCODER: AsciiSet = percent_encoding::NON_ALPHANUMERIC.remove(b'-').remove(b'.').remove(b'_').remove(b'~');
    }
    utf8_percent_encode(src, &*ENCODER)
}

/// OAuth header set used to create an OAuth signature.
#[derive(Clone, Debug)]
pub struct OAuthParams {
    /// The consumer key that represents the app making the API request.
    consumer_key: KeyPair,
    /// The token that represents the user authorizing the request (or the access request
    /// representing a user authorizing the app).
    token: Option<KeyPair>,
    /// A random token representing the request itself. Used to de-duplicate requests on Twitter's
    /// end.
    nonce: String,
    /// A Unix timestamp for when the request was created.
    timestamp: u64,
    /// A callback or verifier parameter, if necessary.
    addon: OAuthAddOn,
}

impl OAuthParams {
    /// Creates an empty `OAuthParams` header with a new `timestamp` and `nonce`.
    ///
    /// **Note**: This should only be used as part of another constructor that populates the tokens!
    /// Attempting to sign a request with an empty consumer and access token will result in an
    /// invalid request.
    fn empty() -> OAuthParams {
        let timestamp = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(dur) => dur,
            Err(err) => err.duration(),
        }
        .as_secs();
        let mut rng = rand::thread_rng();
        let nonce = ::std::iter::repeat(())
            .map(|()| rng.sample(rand::distributions::Alphanumeric))
            .take(32)
            .collect::<String>();
        OAuthParams {
            consumer_key: KeyPair::empty(),
            token: None,
            nonce,
            timestamp,
            addon: OAuthAddOn::None,
        }
    }

    /// Creates a new `OAuthParams` header with the given keys. The `token` is optional
    /// specifically for when you're generating a request token; otherwise it should be the request
    /// token (for when you're generating an access token) or an access token (for when you're
    /// requesting a regular API function).
    pub fn from_keys(consumer_key: KeyPair, token: Option<KeyPair>) -> OAuthParams {
        OAuthParams {
            consumer_key,
            token,
            ..OAuthParams::empty()
        }
    }

    /// Adds the given callback to this `OAuthParams` header.
    ///
    /// Note that the `callback` and `verifier` parameters are mutually exclusive. If you call this
    /// function after setting a verifier with `with_verifier`, it will overwrite the verifier.
    pub fn with_callback(self, callback: String) -> OAuthParams {
        OAuthParams {
            addon: OAuthAddOn::Callback(callback),
            ..self
        }
    }

    /// Adds the given verifier to this `OAuthParams` header.
    ///
    /// Note that the `callback` and `verifier` parameters are mutually exclusive. If you call this
    /// function after setting a callback with `with_callback`, it will overwrite the callback.
    pub fn with_verifier(self, verifier: String) -> OAuthParams {
        OAuthParams {
            addon: OAuthAddOn::Verifier(verifier),
            ..self
        }
    }

    /// Uses the parameters in this `OAuthParams` instance to generate a signature for the given
    /// request, returning it as a `SignedHeader`.
    pub(crate) fn sign_request(self, method: Method, uri: &str, params: Option<&ParamList>) -> SignedHeader {
        let query_string = {
            let sig_params = params
                .cloned()
                .unwrap_or_default()
                .add_param("oauth_consumer_key", self.consumer_key.key.clone())
                .add_param("oauth_nonce", self.nonce.clone())
                .add_param("oauth_signature_method", "HMAC-SHA1")
                .add_param("oauth_timestamp", format!("{}", self.timestamp.clone()))
                .add_param("oauth_version", "1.0")
                .add_opt_param("oauth_token", self.token.clone().map(|k| k.key))
                .add_opt_param("oauth_callback", self.addon.as_callback().map(|s| s.to_string()))
                .add_opt_param("oauth_verifier", self.addon.as_verifier().map(|s| s.to_string()));

            let mut query = sig_params
                .iter()
                .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
                .collect::<Vec<_>>();
            query.sort();

            query.join("&")
        };

        let base_str = format!(
            "{}&{}&{}",
            percent_encode(method.as_str()),
            percent_encode(uri),
            percent_encode(&query_string)
        );
        let key = format!(
            "{}&{}",
            percent_encode(&self.consumer_key.secret),
            percent_encode(&self.token.as_ref().unwrap_or(&KeyPair::new("", "")).secret)
        );

        // TODO check if key is correct length? Can this fail?
        let mut digest = Hmac::<Sha1>::new_varkey(key.as_bytes()).expect("Wrong key length");
        digest.input(base_str.as_bytes());

        SignedHeader {
            params: self,
            signature: base64::encode(&digest.result().code()),
        }
    }
}

/// Represents an "addon" to an OAuth header.
#[derive(Clone, Debug)]
pub enum OAuthAddOn {
    /// An `oauth_callback` parameter, used when generating a request token.
    Callback(String),
    /// An `oauth_verifier` parameter, used when generating an access token.
    Verifier(String),
    /// Neither an `oauth_callback` nor an `oauth_verifier` parameter are present in this header.
    /// This is the default used when signing a regular API request.
    None,
}

impl OAuthAddOn {
    /// Returns the `oauth_callback` parameter, if present.
    fn as_callback(&self) -> Option<&str> {
        match self {
            OAuthAddOn::Callback(c) => Some(c),
            _ => None,
        }
    }

    /// Returns the `oauth_verifier` parameter, if present.
    fn as_verifier(&self) -> Option<&str> {
        match self {
            OAuthAddOn::Verifier(v) => Some(v),
            _ => None,
        }
    }
}

/// A set of `OAuthParams` parameters combined with a request signature, ready to be attached to a
/// request.
pub struct SignedHeader {
    /// The OAuth parameters used to create the signature.
    params: OAuthParams,
    /// The signature for an associated request.
    signature: String,
}

/// The `Display` impl for `SignedHeader` formats it as an `Authorization` header for an HTTP
/// request.
impl fmt::Display for SignedHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // authorization scheme
        write!(f, "OAuth ")?;

        // authorization data
        write!(
            f,
            "oauth_consumer_key=\"{}\"",
            percent_encode(&self.params.consumer_key.key)
        )?;

        write!(f, ", oauth_nonce=\"{}\"", percent_encode(&self.params.nonce))?;

        write!(f, ", oauth_signature=\"{}\"", percent_encode(&self.signature))?;

        write!(
            f,
            ", oauth_signature_method=\"{}\"",
            percent_encode("HMAC-SHA1")
        )?;

        write!(f, ", oauth_timestamp=\"{}\"", self.params.timestamp)?;

        if let Some(ref token) = self.params.token {
            write!(f, ", oauth_token=\"{}\"", percent_encode(&token.key))?;
        }

        write!(f, ", oauth_version=\"{}\"", "1.0")?;

        match self.params.addon {
            OAuthAddOn::Callback(ref callback) => {
                write!(f, ", oauth_callback=\"{}\"", percent_encode(callback))?;
            }
            OAuthAddOn::Verifier(ref verifier) => {
                write!(f, ", oauth_verifier=\"{}\"", percent_encode(verifier))?;
            }
            OAuthAddOn::None => (),
        }

        Ok(())
    }
}

/// An abstracted set of authorization credentials.
///
/// This enum is constructed from a `Token` and either constructs an OAuth signature or a Bearer
/// signature based on what kind of token is given. This allows the `auth` entry points to not need
/// to match on the structure of a `Token` and instead just focus on signing the request.
pub enum AuthHeader {
    /// A set of OAuth parameters based on a consumer/access token combo.
    AccessToken(OAuthParams),
    /// A Bearer token.
    Bearer(String),
}

impl From<Token> for AuthHeader {
    fn from(token: Token) -> AuthHeader {
        match token {
            Token::Access { consumer, access } => {
                AuthHeader::AccessToken(OAuthParams::from_keys(consumer, Some(access)))
            }
            Token::Bearer(b) => {
                AuthHeader::Bearer(b)
            }
        }
    }
}

impl AuthHeader {
    /// With the given parameters, create an `Authorization` header that matches the `Token` that
    /// was used to create this `AuthHeader`. The resulting string can be passed as an
    /// `Authorization` header to an API request.
    ///
    /// If the source `Token` was a bearer token, this function ignores the parameters and gives a
    /// Bearer authorization based on the original token.
    pub(crate) fn sign_request(self, method: Method, uri: &str, params: Option<&ParamList>) -> String {
        match self {
            AuthHeader::AccessToken(oauth) => {
                oauth.sign_request(method, uri, params).to_string()
            }
            AuthHeader::Bearer(b) => {
                format!("Bearer {}", b)
            }
        }
    }
}

/// Creates a basic `Authorization` header based on the given consumer token.
///
/// The authorization created by this function can only be used with requests to generate or
/// invalidate a bearer token. Using this authorization with any other endpoint will result in an
/// invalid request.
pub fn bearer_request(con_token: &KeyPair) -> String {
    let text = format!("{}:{}", con_token.key, con_token.secret);
    format!("Basic {}", base64::encode(&text))
}

// n.b. this function is re-exported in the `raw` module - these docs are public!
/// Assemble a signed GET request to the given URL with the given parameters.
///
/// The given parameters, if present, will be appended to the given `uri` as a percent-encoded
/// query string. If the given `token` is not a Bearer token, the parameters will also be used to
/// create the OAuth signature.
pub fn get(uri: &str, token: &Token, params: Option<&ParamList>) -> Request<Body> {
    let full_url = if let Some(p) = params {
        let query = p
            .iter()
            .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
            .collect::<Vec<_>>()
            .join("&");

        format!("{}?{}", uri, query)
    } else {
        uri.to_string()
    };

    let request = Request::get(full_url)
        .header(AUTHORIZATION,
                AuthHeader::from(token.clone()).sign_request(Method::GET, uri, params));

    request.body(Body::empty()).unwrap()
}

// n.b. this function is re-exported in the `raw` module - these docs are public!
/// Assemble a signed POST request to the given URL with the given parameters.
///
/// The given parameters, if present, will be percent-encoded and included in the POST body
/// formatted with a content-type of `application/x-www-form-urlencoded`. If the given `token` is
/// not a Bearer token, the parameters will also be used to create the OAuth signature.
pub fn post(uri: &str, token: &Token, params: Option<&ParamList>) -> Request<Body> {
    let content = "application/x-www-form-urlencoded";
    let body = if let Some(p) = params {
        Body::from(
            p.iter()
                .map(|(k, v)| format!("{}={}", k, percent_encode(v)))
                .collect::<Vec<_>>()
                .join("&"),
        )
    } else {
        Body::empty()
    };

    let request =
        Request::post(uri)
            .header(CONTENT_TYPE, content)
            .header(AUTHORIZATION,
                    AuthHeader::from(token.clone()).sign_request(Method::POST, uri, params));

    request.body(body).unwrap()
}

// n.b. this function is re-exported in the `raw` module - these docs are public!
/// Assemble a signed POST request to the given URL with the given JSON body.
///
/// This method of building requests allows you to use endpoints that require a request body of
/// plain text or JSON, like `POST media/metadata/create`. Note that this function does not encode
/// its parameters into the OAuth signature, so take care if the endpoint you're using lists
/// parameters as part of its specification.
pub fn post_json<B: serde::Serialize>(uri: &str, token: &Token, body: B) -> Request<Body> {
    let content = "application/json; charset=UTF-8";
    let body = Body::from(serde_json::to_string(&body).unwrap()); // TODO rewrite

    let request =
        Request::post(uri)
            .header(CONTENT_TYPE, content)
            .header(AUTHORIZATION,
                    AuthHeader::from(token.clone()).sign_request(Method::POST, uri, None));

    request.body(body).unwrap()
}
