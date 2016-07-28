use std;
use std::error::Error;
use std::borrow::Cow;
use std::collections::HashMap;
use hyper;
use hyper::client::response::Response as HyperResponse;
use hyper::header::{Authorization, Scheme, ContentType};
use hyper::method::Method;
use mime::Mime;
use time;
use rand::{self, Rng};
use crypto::hmac::Hmac;
use crypto::mac::Mac;
use crypto::sha1::Sha1;
use rustc_serialize::base64::{self, ToBase64};
use super::{links, error};
use super::common::*;

///OAuth header set given to Twitter calls.
///
///Since different authorization/authentication calls have various parameters
///that go into this header, they're optionally placed at the end of this header.
///On the other hand, `signature` is optional so a structured header can be
///passed to `sign()` for signature.
#[derive(Clone, Debug)]
struct TwitterOAuth {
    consumer_key: String,
    nonce: String,
    signature: Option<String>,
    timestamp: i64,
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
        let mut timestamp: Option<i64> = None;
        let mut token: Option<String> = None;
        let mut callback: Option<String> = None;
        let mut verifier: Option<String> = None;

        for substr in s.split(',') {
            let mut parts = substr.trim().split('=');
            match parts.next() {
                Some("oauth_consumer_key") => consumer_key = parts.next().map(str::to_string),
                Some("oauth_nonce") => nonce = parts.next().map(str::to_string),
                Some("oauth_signature") => signature = parts.next().map(str::to_string),
                Some("oauth_timestamp") => match parts.next().map(i64::from_str) {
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

///A key/secret pair representing an OAuth token.
pub struct Token<'a> {
    pub key: Cow<'a, str>,
    pub secret: Cow<'a, str>,
}

impl<'a> Token<'a> {
    ///Creates a Token with the given key and secret.
    ///
    ///This can be called with either `&str` or `String`. In the former
    ///case the resulting Token will have the same lifetime as the given
    ///reference. If two Strings are given, the Token effectively has
    ///lifetime `'static`.
    pub fn new<K, S>(key: K, secret: S) -> Token<'a>
        where K: Into<Cow<'a, str>>,
              S: Into<Cow<'a, str>>
    {
        Token {
            key: key.into(),
            secret: secret.into(),
        }
    }
}

///With the given OAuth header and method parameters, create an OAuth
///signature and return the header with the signature in line.
fn sign(header: TwitterOAuth,
        method: Method,
        uri: &str,
        params: Option<&ParamList>,
        con_token: &Token,
        access_token: Option<&Token>) -> TwitterOAuth {
    let query_string = {
        let mut sig_params = params.map(|p| p.clone()).unwrap_or(HashMap::new());

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
                      percent_encode(&access_token.unwrap_or(&Token::new("", "")).secret));

    let mut sig = Hmac::new(Sha1::new(), key.as_bytes());
    sig.input(base_str.as_bytes());

    let config = base64::Config {
        char_set: base64::CharacterSet::Standard,
        newline: base64::Newline::LF,
        pad: true,
        line_length: None,
    };

    TwitterOAuth {
        consumer_key: header.consumer_key,
        nonce: header.nonce,
        signature: Some(sig.result().code().to_base64(config)),
        timestamp: header.timestamp,
        token: header.token,
        callback: header.callback,
        verifier: header.verifier,
    }
}

///With the given method parameters, return a signed OAuth header.
fn get_header(method: Method,
              uri: &str,
              con_token: &Token,
              access_token: Option<&Token>,
              callback: Option<String>,
              verifier: Option<String>,
              params: Option<&ParamList>) -> TwitterOAuth {
    let header = TwitterOAuth {
        consumer_key: con_token.key.to_string(),
        nonce: rand::thread_rng().gen_ascii_chars().take(32).collect::<String>(),
        signature: None,
        timestamp: time::now_utc().to_timespec().sec,
        token: access_token.map(|tok| tok.key.to_string()),
        callback: callback,
        verifier: verifier,
    };

    sign(header, method, uri, params, con_token, access_token)
}

pub fn get(uri: &str,
           con_token: &Token,
           access_token: &Token,
           params: Option<&ParamList>) -> Result<HyperResponse, error::Error> {
    let header = get_header(Method::Get, uri, con_token, Some(access_token),
                            None, None, params);

    let full_url = if let Some(p) = params {
        let query = p.iter()
                     .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
                     .collect::<Vec<_>>()
                     .join("&");

        format!("{}?{}", uri, query)
    }
    else { uri.to_string() };
    let client = hyper::Client::new();
    Ok(try!(client.get(&full_url).header(Authorization(header)).send()))
}

pub fn post(uri: &str,
            con_token: &Token,
            access_token: &Token,
            params: Option<&ParamList>) -> Result<HyperResponse, error::Error> {
    let header = get_header(Method::Post, uri, con_token, Some(access_token),
                            None, None, params);

    let content: Mime = "application/x-www-form-urlencoded".parse().unwrap();
    let body = if let Some(p) = params {
        p.iter()
         .map(|(k, v)| format!("{}={}", k, percent_encode(v)))
         .collect::<Vec<_>>()
         .join("&")
    }
    else { "".to_string() };
    let client = hyper::Client::new();
    Ok(try!(client.post(uri).body(body.as_bytes())
                  .header(Authorization(header))
                  .header(ContentType(content))
                  .send()))
}

///With the given consumer Token, ask Twitter for a request Token that can be
///used to request access to the user's account.
///
///This can be considered Step 1 in obtaining access to a user's account. With
///this Token, a web-based application can use `authenticate_url` (currently
///unimplemented), and a desktop-based application can use `authorize_url` to
///perform the authorization request.
///
///The parameter `callback` is used to provide an OAuth Callback URL for a web-
///or mobile-based application to receive the results of the authorization request.
///To use the PIN-Based Auth request, this must be set to `"oob"`. The resulting
///Token can be passed to `authorize_url` to give the user a means to accept the
///request.
pub fn request_token<S: Into<String>>(con_token: &Token, callback: S) -> Result<Token<'static>, error::Error> {
    let header = get_header(Method::Post, links::auth::REQUEST_TOKEN,
                            con_token, None, Some(callback.into()), None, None);

    let client = hyper::Client::new();
    let mut resp = try!(client.post(links::auth::REQUEST_TOKEN)
                          .header(Authorization(header))
                          .send());

    let full_resp = try!(response_raw(&mut resp));

    let mut key: Option<String> = None;
    let mut secret: Option<String> = None;

    for elem in full_resp.split('&') {
        let mut kv = elem.splitn(2, '=');
        match kv.next() {
            Some("oauth_token") => key = kv.next().map(|s| s.to_string()),
            Some("oauth_token_secret") => secret = kv.next().map(|s| s.to_string()),
            Some(_) => (),
            None => return Err(error::Error::InvalidResponse),
        }
    }

    Ok(Token::new(try!(key.ok_or(error::Error::MissingValue("oauth_token"))),
                  try!(secret.ok_or(error::Error::MissingValue("oauth_token_secret")))))
}

///With the given request Token, return a URL that a user can access to
///accept or reject an authorization request.
///
///This can be considered Step 2 in obtaining access to a user's account.
///Using PIN-Based Auth, give the URL that this function returns to the
///user so they can process the authorization request. They will receive
///a PIN in return, that can be given as the Verifier to `access_token`.
pub fn authorize_url(request_token: &Token) -> String {
    format!("{}?oauth_token={}", links::auth::AUTHORIZE, request_token.key)
}

///With the given OAuth tokens and verifier, ask Twitter for an access
///Token that can be used to sign further requests to the Twitter API.
///
///This can be considered Step 3 in obtaining access to a user's account.
///The Token this function returns represents the user's authorization
///that your app can use their account, and needs to be given to all other
///functions in the Twitter API.
///
///The OAuth Verifier this function takes is either given as a result of
///the OAuth Callback given to `request_token`, or the PIN given to the
///user as a result of their access of the `authorize_url`.
///
///This function also returns the User ID and Username of the authenticated
///user.
pub fn access_token<S: Into<String>>(con_token: &Token,
                                     request_token: &Token,
                                     verifier: S) -> Result<(Token<'static>, i64, String), error::Error> {
    let header = get_header(Method::Post, links::auth::ACCESS_TOKEN,
                            con_token, Some(request_token), None, Some(verifier.into()), None);

    let client = hyper::Client::new();
    let mut resp = try!(client.post(links::auth::ACCESS_TOKEN)
                          .header(Authorization(header))
                          .send());

    let full_resp = try!(response_raw(&mut resp));

    let mut key: Option<String> = None;
    let mut secret: Option<String> = None;
    let mut id: Option<i64> = None;
    let mut username: Option<String> = None;

    for elem in full_resp.split('&') {
        let mut kv = elem.splitn(2, '=');
        match kv.next() {
            Some("oauth_token") => key = kv.next().map(|s| s.to_string()),
            Some("oauth_token_secret") => secret = kv.next().map(|s| s.to_string()),
            Some("user_id") => id = kv.next().and_then(|s| i64::from_str_radix(s, 10).ok()),
            Some("screen_name") => username = kv.next().map(|s| s.to_string()),
            Some(_) => (),
            None => return Err(error::Error::InvalidResponse),
        }
    }

    Ok((Token::new(try!(key.ok_or(error::Error::MissingValue("oauth_token"))),
                  try!(secret.ok_or(error::Error::MissingValue("oauth_token_secret")))),
        try!(id.ok_or(error::Error::MissingValue("user_id"))),
        try!(username.ok_or(error::Error::MissingValue("screen_name")))))
}
