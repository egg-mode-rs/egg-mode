// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Internal mechanisms for the `auth` module.

use std::borrow::Cow;
use std::collections::BTreeMap;
use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use base64;
use hmac::{Hmac, Mac, NewMac};
use hyper::header::{AUTHORIZATION, CONTENT_TYPE};
use hyper::{Body, Method, Request};
use rand::{self, Rng};
use sha1::Sha1;

use crate::common::*;

use super::{Token, KeyPair};

// n.b. this type is exported in `raw::auth` - these docs are public!
/// Builder struct to assemble and sign an API request.
///
/// For more information about how to use this type and about building requests manually, see [the
/// module docs](index.html).
pub struct RequestBuilder<'a> {
    base_uri: &'a str,
    method: Method,
    params: Option<ParamList>,
    query: Option<String>,
    body: Option<(Body, &'static str)>,
    addon: OAuthAddOn,
}

impl<'a> RequestBuilder<'a> {
    /// Creates a new `RequestBuilder` with the given HTTP method and base URL.
    pub fn new(method: Method, base_uri: &'a str) -> Self {
        RequestBuilder {
            base_uri,
            method,
            params: None,
            query: None,
            body: None,
            addon: OAuthAddOn::None,
        }
    }

    /// Adds the given parameters as a query string. Parameters given this way will be included in
    /// the OAuth signature.
    ///
    /// Note that functions that take a `ParamList` accumulate parameters as part of the OAuth
    /// signature. If you call both `with_query_params` and `with_body_params`, both sets of
    /// parameters will be used as part of the OAuth signature.
    ///
    /// On the other hand, the query string is not cumulative. If you call `with_query_params`
    /// multiple times, only the last set of parameters will actually be considered part of the
    /// query string.
    pub fn with_query_params(self, params: &ParamList) -> Self {
        let total_params = if let Some(mut my_params) = self.params {
            my_params.combine(params.clone());
            my_params
        } else {
            params.clone()
        };
        RequestBuilder {
            query: Some(params.to_urlencoded()),
            params: Some(total_params),
            ..self
        }
    }

    /// Adds the given params as a request body, formatted as `application/x-www-form-urlencoded`.
    /// Parameters given this way will be included in the OAuth signature.
    ///
    /// Note that functions that take a `ParamList` accumulate parameters as part of the OAuth
    /// signature. If you call both `with_query_params` and `with_body_params`, both sets of
    /// parameters will be used as part of the OAuth signature.
    ///
    /// Note that the functions that specify a request body each overwrite the body. For example,
    /// if you specify `with_body_params` and also `with_body_json`, only the one you call last
    /// will be sent with the request.
    pub fn with_body_params(self, params: &ParamList) -> Self {
        let total_params = if let Some(mut my_params) = self.params {
            my_params.combine(params.clone());
            my_params
        } else {
            params.clone()
        };
        RequestBuilder {
            body: Some((Body::from(params.to_urlencoded()), "application/x-www-form-urlencoded")),
            params: Some(total_params),
            ..self
        }
    }

    /// Includes the given data as the request body, formatted as JSON. Data given this way will
    /// *not* be included in the OAuth signature.
    ///
    /// Note that the functions that specify a request body each overwrite the body. For example,
    /// if you specify `with_body_params` and also `with_body_json`, only the one you call last
    /// will be sent with the request.
    pub fn with_body_json(self, body: impl serde::Serialize) -> Self {
        self.with_body(serde_json::to_string(&body).unwrap(), "application/json; charset=UTF-8")
    }

    /// Includes the given data as the request body, with the given content type. Data given this
    /// way will *not* be included in the OAuth signature.
    ///
    /// Note that the functions that specify a request body each overwrite the body. For example,
    /// if you specify `with_body_params` and also `with_body`, only the one you call last will be
    /// sent with the request.
    pub fn with_body(self, body: impl Into<Body>, content: &'static str) -> Self {
        RequestBuilder {
            body: Some((body.into(), content)),
            ..self
        }
    }

    /// Includes the given OAuth Callback into the OAuth parameters.
    ///
    /// Note that `oauth_callback` and `oauth_verifier` are mutually exclusive. If you specify both
    /// on the same request, only the last one will be sent.
    pub fn oauth_callback(self, callback: impl Into<String>) -> Self {
        RequestBuilder {
            addon: OAuthAddOn::Callback(callback.into()),
            ..self
        }
    }

    /// Includes the given OAuth Verifier into the OAuth parameters.
    ///
    /// Note that `oauth_callback` and `oauth_verifier` are mutually exclusive. If you specify both
    /// on the same request, only the last one will be sent.
    pub fn oauth_verifier(self, verifier: impl Into<String>) -> Self {
        RequestBuilder {
            addon: OAuthAddOn::Verifier(verifier.into()),
            ..self
        }
    }

    /// Formats this `RequestBuilder` into a complete `Request`, signing it with the given keys.
    ///
    /// While the `token` parameter is an Option here, it should only be `None` when generating a
    /// request token; all other calls must have two sets of keys (or be authenticated in a
    /// different way, i.e. a Bearer token).
    pub fn request_keys(self, consumer_key: &KeyPair, token: Option<&KeyPair>) -> Request<Body> {
        let oauth = OAuthParams::from_keys(consumer_key.clone(), token.cloned())
            .with_addon(self.addon.clone())
            .sign_request(self.method.clone(), self.base_uri, self.params.as_ref());
        self.request_authorization(oauth.to_string())
    }

    /// Formats this `RequestBuilder` into a complete `Request`, signing it with the given token.
    ///
    /// If the given `Token` is an Access token, the request will be signed using OAuth 1.0a, using
    /// the given URI, HTTP method, and parameters to create a signature.
    ///
    /// If the given `Token` is a Bearer token, the request will be authenticated using OAuth 2.0,
    /// specifying the given Bearer token as authorization.
    pub fn request_token(self, token: &Token) -> Request<Body> {
        match token {
            Token::Access { consumer, access } => self.request_keys(consumer, Some(access)),
            Token::Bearer(bearer) => self.request_authorization(format!("Bearer {}", bearer)),
        }
    }

    /// Formats this `RequestBuilder` into a complete `Request`, with an Authorization header
    /// formatted using HTTP Basic authentication using the given consumer key, as expected by the
    /// `POST oauth2/token` endpoint.
    ///
    /// This Authorization should only be used when requesting a Bearer token; other requests need
    /// to be signed with multiple keys (as with `request_keys` or giving an Access token to
    /// `request_token`) or with a proper Bearer token given to `request_token`.
    ///
    /// This authorization can also be used to access Enterprise API endpoints that require Basic
    /// authentication, using a `KeyPair` with the email address and password that would ordinarily
    /// be used to access the Enterprise API Console.
    pub fn request_consumer_bearer(self, consumer_key: &KeyPair) -> Request<Body> {
        self.request_authorization(bearer_request(consumer_key))
    }

    /// Assembles the final `Request` with the given Authorization header. This is private to
    /// require that a well-formed header is constructed given, as constructed from the other
    /// `request_*` methods.
    fn request_authorization(self, authorization: String) -> Request<Body> {
        let full_url = if let Some(query) = self.query {
            format!("{}?{}", self.base_uri, query)
        } else {
            self.base_uri.to_string()
        };
        let request = Request::builder()
            .method(self.method)
            .uri(full_url)
            .header(AUTHORIZATION, authorization);

        if let Some((body, content)) = self.body {
            request.header(CONTENT_TYPE, content)
                .body(body).unwrap()
        } else {
            request.body(Body::empty()).unwrap()
        }
    }
}

/// OAuth header set used to create an OAuth signature.
#[derive(Clone, Debug)]
struct OAuthParams {
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
            .map(char::from)
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
    fn from_keys(consumer_key: KeyPair, token: Option<KeyPair>) -> OAuthParams {
        OAuthParams {
            consumer_key,
            token,
            ..OAuthParams::empty()
        }
    }

    /// Adds the given callback or verifier to this `OAuthParams` header.
    fn with_addon(self, addon: OAuthAddOn) -> OAuthParams {
        OAuthParams {
            addon,
            ..self
        }
    }

    /// Uses the parameters in this `OAuthParams` instance to generate a signature for the given
    /// request, returning it as a `SignedHeader`.
    fn sign_request(self, method: Method, uri: &str, params: Option<&ParamList>) -> SignedHeader {
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
        let mut digest = Hmac::<Sha1>::new_from_slice(key.as_bytes()).expect("Wrong key length");
        digest.update(base_str.as_bytes());

        let mut params: BTreeMap<&'static str, Cow<'static, str>> = BTreeMap::new();
        params.insert("oauth_signature_method", "HMAC-SHA1".into());
        params.insert("oauth_version", "1.0".into());

        params.insert("oauth_consumer_key", self.consumer_key.key);
        if let Some(token) = self.token {
            params.insert("oauth_token", token.key);
        }

        params.insert("oauth_nonce", self.nonce.into());
        params.insert("oauth_timestamp", self.timestamp.to_string().into());

        match self.addon {
            OAuthAddOn::Callback(c) => {
                params.insert("oauth_callback", c.into());
            }
            OAuthAddOn::Verifier(v) => {
                params.insert("oauth_verifier", v.into());
            }
            OAuthAddOn::None => (),
        }

        params.insert("oauth_signature", base64::encode(&digest.finalize().into_bytes()).into());

        SignedHeader {
            params,
        }
    }
}

/// Represents an "addon" to an OAuth header.
#[derive(Clone, Debug)]
enum OAuthAddOn {
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
struct SignedHeader {
    /// The OAuth parameters used to create the signature.
    params: BTreeMap<&'static str, Cow<'static, str>>,
}

/// The `Display` impl for `SignedHeader` formats it as an `Authorization` header for an HTTP
/// request.
impl fmt::Display for SignedHeader {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // authorization scheme
        write!(f, "OAuth ")?;

        // authorization data

        let mut first = true;
        for (k, v) in &self.params {
            if first {
                first = false;
            } else {
                write!(f, ", ")?;
            }

            write!(f, "{}=\"{}\"", k, percent_encode(v))?;
        }

        Ok(())
    }
}

/// Creates a basic `Authorization` header based on the given consumer token.
///
/// The authorization created by this function can only be used with requests to generate or
/// invalidate a bearer token. Using this authorization with any other endpoint will result in an
/// invalid request.
fn bearer_request(con_token: &KeyPair) -> String {
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
    let mut request = RequestBuilder::new(Method::GET, uri);
    if let Some(params) = params {
        request = request.with_query_params(params);
    }
    request.request_token(token)
}

// n.b. this function is re-exported in the `raw` module - these docs are public!
/// Assemble a signed DELETE request to the given URL with the given parameters.
///
/// The given parameters, if present, will be appended to the given `uri` as a percent-encoded
/// query string. If the given `token` is not a Bearer token, the parameters will also be used to
/// create the OAuth signature.
pub fn delete(uri: &str, token: &Token, params: Option<&ParamList>) -> Request<Body> {
    let mut request = RequestBuilder::new(Method::DELETE, uri);
    if let Some(params) = params {
        request = request.with_query_params(params);
    }
    request.request_token(token)
}

// n.b. this function is re-exported in the `raw` module - these docs are public!
/// Assemble a signed POST request to the given URL with the given parameters.
///
/// The given parameters, if present, will be percent-encoded and included in the POST body
/// formatted with a content-type of `application/x-www-form-urlencoded`. If the given `token` is
/// not a Bearer token, the parameters will also be used to create the OAuth signature.
pub fn post(uri: &str, token: &Token, params: Option<&ParamList>) -> Request<Body> {
    let mut request = RequestBuilder::new(Method::POST, uri);
    if let Some(params) = params {
        request = request.with_body_params(params);
    }
    request.request_token(token)
}

// n.b. this function is re-exported in the `raw` module - these docs are public!
/// Assemble a signed POST request to the given URL with the given JSON body.
///
/// This method of building requests allows you to use endpoints that require a request body of
/// plain text or JSON, like `POST media/metadata/create`. Note that this function does not encode
/// its parameters into the OAuth signature, so take care if the endpoint you're using lists
/// parameters as part of its specification.
pub fn post_json<B: serde::Serialize>(uri: &str, token: &Token, body: B) -> Request<Body> {
    RequestBuilder::new(Method::POST, uri)
        .with_body_json(body)
        .request_token(token)
}

#[cfg(test)]
mod tests {
    use super::bearer_request;

    #[test]
    fn bearer_header() {
        let con_key = "xvz1evFS4wEEPTGEFPHBog";
        let con_secret = "L8qq9PZyRg6ieKGEKhZolGC0vJWLw8iEJ88DRdyOg";
        let con_token = super::KeyPair::new(con_key, con_secret);

        let output = bearer_request(&con_token);

        assert_eq!(output, "Basic eHZ6MWV2RlM0d0VFUFRHRUZQSEJvZzpMOHFxOVBaeVJnNmllS0dFS2hab2xHQzB2SldMdzhpRUo4OERSZHlPZw==");
    }
}
