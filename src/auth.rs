use std;
use std::error::Error;
use std::borrow::Cow;
use std::time::{UNIX_EPOCH, SystemTime};
use url::percent_encoding::{EncodeSet, utf8_percent_encode};
use hyper;
use hyper::client::response::Response as HyperResponse;
use hyper::header::{Authorization, Scheme, ContentType, Basic, Bearer};
use hyper::method::Method;
use mime::Mime;
use rand::{self, Rng};
use crypto::hmac::Hmac;
use crypto::mac::Mac;
use crypto::sha1::Sha1;
use rustc_serialize::base64::{self, ToBase64};
use rustc_serialize::json;
use super::{links, error};
use super::common::*;

//the encode sets in the url crate don't quite match what twitter wants,
//so i'll make up my own
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
///Since different authorization/authentication calls have various parameters
///that go into this header, they're optionally placed at the end of this header.
///On the other hand, `signature` is optional so a structured header can be
///passed to `sign()` for signature.
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

///A key/secret pair representing an OAuth token.
#[derive(Debug, Clone)]
pub struct KeyPair<'a> {
    ///A key used to identify an application or user.
    pub key: Cow<'a, str>,
    ///A private key used to sign messages from an application or user.
    pub secret: Cow<'a, str>,
}

impl<'a> KeyPair<'a> {
    ///Creates a KeyPair with the given key and secret.
    ///
    ///This can be called with either `&str` or `String`. In the former
    ///case the resulting KeyPair will have the same lifetime as the given
    ///reference. If two Strings are given, the KeyPair effectively has
    ///lifetime `'static`.
    pub fn new<K, S>(key: K, secret: S) -> KeyPair<'a>
        where K: Into<Cow<'a, str>>,
              S: Into<Cow<'a, str>>
    {
        KeyPair {
            key: key.into(),
            secret: secret.into(),
        }
    }
}

///A token that can be used to sign requests to Twitter.
#[derive(Debug, Clone)]
pub enum Token<'a> {
    ///An OAuth Access token indicating the request is coming from a specific user.
    Access {
        ///A "consumer" key/secret that represents the application sending the request.
        consumer: KeyPair<'a>,
        ///An "access" key/secret that represents the user's authorization of the application.
        access: KeyPair<'a>,
    },
    ///An OAuth Bearer token indicating the request is coming from the application itself, not a
    ///particular user. See [`bearer_token`] for more information.
    ///
    ///[`bearer_token`]: fn.bearer_token.html
    Bearer(String),
}

///With the given OAuth header and method parameters, create an OAuth
///signature and return the header with the signature in line.
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

    let mut sig = Hmac::new(Sha1::new(), key.as_bytes());
    sig.input(base_str.as_bytes());

    let config = base64::Config {
        char_set: base64::CharacterSet::Standard,
        newline: base64::Newline::LF,
        pad: true,
        line_length: None,
    };

    TwitterOAuth {
        signature: Some(sig.result().code().to_base64(config)),
        ..header
    }
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

pub fn get(uri: &str,
           token: &Token,
           params: Option<&ParamList>) -> Result<HyperResponse, error::Error> {
    let full_url = if let Some(p) = params {
        let query = p.iter()
                     .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
                     .collect::<Vec<_>>()
                     .join("&");

        format!("{}?{}", uri, query)
    }
    else { uri.to_string() };

    let client = hyper::Client::new();

    let request = match *token {
        Token::Access {
            consumer: ref con_token,
            access: ref access_token,
        } => {
            let header = get_header(Method::Get, uri, con_token, Some(access_token),
                                    None, None, params);
            client.get(&full_url).header(Authorization(header))
        },
        Token::Bearer(ref token) => {
            client.get(&full_url).header(Authorization(Bearer { token: token.clone() }))
        },
    };

    Ok(try!(request.send()))
}

pub fn post(uri: &str,
            token: &Token,
            params: Option<&ParamList>) -> Result<HyperResponse, error::Error> {
    let content: Mime = "application/x-www-form-urlencoded".parse().unwrap();
    let body = if let Some(p) = params {
        p.iter()
         .map(|(k, v)| format!("{}={}", k, percent_encode(v)))
         .collect::<Vec<_>>()
         .join("&")
    }
    else { "".to_string() };

    let client = hyper::Client::new();
    let request = client.post(uri).body(body.as_bytes())
                                  .header(ContentType(content));

    let request = match *token {
        Token::Access {
            consumer: ref con_token,
            access: ref access_token,
        } => {
            let header = get_header(Method::Post, uri, con_token, Some(access_token),
                                    None, None, params);

            request.header(Authorization(header))
        },
        Token::Bearer(ref token) => {
            request.header(Authorization(Bearer { token: token.clone() }))
        },
    };


    Ok(try!(request.send()))
}

///With the given consumer KeyPair, ask Twitter for a request KeyPair that can be
///used to request access to the user's account.
///
///This can be considered Step 1 in obtaining access to a user's account. With
///this KeyPair, a web-based application can use `authenticate_url`, and a
///desktop-based application can use `authorize_url` to perform the authorization
///request.
///
///The parameter `callback` is used to provide an OAuth Callback URL for a web-
///or mobile-based application to receive the results of the authorization request.
///To use the PIN-Based Auth request, this must be set to `"oob"`. The resulting
///KeyPair can be passed to `authorize_url` to give the user a means to accept the
///request.
pub fn request_token<S: Into<String>>(con_token: &KeyPair, callback: S) -> Result<KeyPair<'static>, error::Error> {
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
            None => return Err(error::Error::InvalidResponse("unexpected end of request_token response", None)),
        }
    }

    Ok(KeyPair::new(try!(key.ok_or(error::Error::MissingValue("oauth_token"))),
                    try!(secret.ok_or(error::Error::MissingValue("oauth_token_secret")))))
}

///With the given request KeyPair, return a URL that a user can access to
///accept or reject an authorization request.
///
///This can be considered Step 2 in obtaining access to a user's account.
///Using [PIN-Based Auth][] for desktop applications, give the URL that this
///function returns to the user so they can process the authorization
///request. They will receive a PIN in return, that can be given as the
///Verifier to `access_token`.
///
///[Pin-Based Auth]: https://dev.twitter.com/oauth/pin-based
pub fn authorize_url(request_token: &KeyPair) -> String {
    format!("{}?oauth_token={}", links::auth::AUTHORIZE, request_token.key)
}

///With the given request KeyPair, return a URL to redirect a user to so they
///can accept or reject an authorization request.
///
///This can be considered Step 2 in obtaining access to a user's account.
///Using the "[Sign in with Twitter][]" authenication flow for websites,
///your application can redirect the user to the URL returned by this
///function. Upon accepting the request, the user is redirected to the
///callback URL given to `access_token`, with an `oauth_token` and
///`oauth_verifier` appended as a query string. That Verifier can then be
///given to `access_token` to complete authorization.
///
///[Sign in with Twitter]: https://dev.twitter.com/web/sign-in
pub fn authenticate_url(request_token: &KeyPair) -> String {
    format!("{}?oauth_token={}", links::auth::AUTHENTICATE, request_token.key)
}

///With the given OAuth tokens and verifier, ask Twitter for an access
///KeyPair that can be used to sign further requests to the Twitter API.
///
///This can be considered Step 3 in obtaining access to a user's account.
///The KeyPair this function returns represents the user's authorization
///that your app can use their account, and needs to be given to all other
///functions in the Twitter API.
///
///The OAuth Verifier this function takes is either given as a result of
///the OAuth Callback given to `request_token`, or the PIN given to the
///user as a result of their access of the `authorize_url`.
///
///This function also returns the User ID and Username of the authenticated
///user.
pub fn access_token<'a, S: Into<String>>(con_token: KeyPair<'a>,
                                     request_token: &KeyPair,
                                     verifier: S) -> Result<(Token<'a>, u64, String), error::Error> {
    let header = get_header(Method::Post, links::auth::ACCESS_TOKEN,
                            &con_token, Some(request_token), None, Some(verifier.into()), None);

    let client = hyper::Client::new();
    let mut resp = try!(client.post(links::auth::ACCESS_TOKEN)
                          .header(Authorization(header))
                          .send());

    let full_resp = try!(response_raw(&mut resp));

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
            None => return Err(error::Error::InvalidResponse("unexpected end of response in access_token", None)),
        }
    }

    let access_key = try!(key.ok_or(error::Error::MissingValue("oauth_token")));
    let access_secret = try!(secret.ok_or(error::Error::MissingValue("oauth_token_secret")));

    Ok((Token::Access {
            consumer: con_token,
            access: KeyPair::new(access_key, access_secret),
        },
        try!(id.ok_or(error::Error::MissingValue("user_id"))),
        try!(username.ok_or(error::Error::MissingValue("screen_name")))))
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
pub fn bearer_token(con_token: &KeyPair) -> Result<Token<'static>, error::Error> {
    let auth_header = bearer_request(con_token);

    let content: Mime = "application/x-www-form-urlencoded;charset=UTF-8".parse().unwrap();
    let client = hyper::Client::new();
    let mut resp = try!(client.post(links::auth::BEARER_TOKEN)
                              .header(Authorization(auth_header))
                              .header(ContentType(content))
                              .body("grant_type=client_credentials".as_bytes())
                              .send());
    let full_resp = try!(response_raw(&mut resp));

    let decoded = try!(json::Json::from_str(&full_resp));
    let result = try!(decoded.find("access_token")
                             .and_then(|s| s.as_string())
                             .ok_or(error::Error::MissingValue("access_token")));

    Ok(Token::Bearer(result.to_owned()))
}

///Invalidate the given Bearer token using the given consumer KeyPair. Upon success, returns the
///Token that was just invalidated.
///
///# Errors
///
///If the Token passed in is not a Bearer token, this function will return a `MissingValue` error
///referencing the "token", without calling Twitter.
pub fn invalidate_bearer(con_token: &KeyPair, token: &Token) -> Result<Token<'static>, error::Error> {
    let token = if let Token::Bearer(ref token) = *token {
        token
    }
    else {
        return Err(error::Error::MissingValue("token"));
    };

    let auth_header = bearer_request(con_token);

    let content: Mime = "application/x-www-form-urlencoded;charset=UTF-8".parse().unwrap();
    let client = hyper::Client::new();
    let mut resp = try!(client.post(links::auth::INVALIDATE_BEARER)
                              .header(Authorization(auth_header))
                              .header(ContentType(content))
                              .body(format!("access_token={}", token).as_bytes())
                              .send());
    let full_resp = try!(response_raw(&mut resp));

    let decoded = try!(json::Json::from_str(&full_resp));
    let result = try!(decoded.find("access_token")
                             .and_then(|s| s.as_string())
                             .ok_or(error::Error::MissingValue("access_token")));

    Ok(Token::Bearer(result.to_owned()))
}

///If the given tokens are valid, return the user information for the authenticated user.
pub fn verify_tokens(token: &Token)
    -> WebResponse<::user::TwitterUser>
{
    let mut resp = try!(get(links::auth::VERIFY_CREDENTIALS, token, None));

    parse_response(&mut resp)
}

#[cfg(test)]
mod tests {
    use super::bearer_request;
    use hyper::header::{Authorization, HeaderFormat};

    #[test]
    fn bearer_header() {
        let con_key = "xvz1evFS4wEEPTGEFPHBog";
        let con_secret = "L8qq9PZyRg6ieKGEKhZolGC0vJWLw8iEJ88DRdyOg";
        let con_token = super::KeyPair::new(con_key, con_secret);

        let header = Authorization(bearer_request(&con_token));
        let test = &header as &(HeaderFormat + Send + Sync);

        let output = test.to_string();

        assert_eq!(output, "Basic eHZ6MWV2RlM0d0VFUFRHRUZQSEJvZzpMOHFxOVBaeVJnNmllS0dFS2hab2xHQzB2SldMdzhpRUo4OERSZHlPZw==");
    }
}
