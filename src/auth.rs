use std;
use std::error::Error;
use hyper::header::Scheme;
use super::percent_encode;

//TODO: should this be just a HashMap<String, String> instead?
///Basic component of the OAuth authorization header. Intended to be combined
///with hyper's `Authorization` struct to create a full request header.
#[derive(Clone, Debug)]
pub struct TwitterOAuth {
    pub consumer_key: String,
    pub nonce: String,
    pub signature: String,
    pub timestamp: u64,
    pub token: Option<String>,
    pub callback: Option<String>,
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

        for substr in s.split(',') {
            let mut parts = substr.trim().split('=');
            match parts.next() {
                Some("oauth_consumer_key") => consumer_key = parts.next().map(str::to_string),
                Some("oauth_nonce") => nonce = parts.next().map(str::to_string),
                Some("oauth_signature") => signature = parts.next().map(str::to_string),
                Some("oauth_timestamp") => match parts.next().map(u64::from_str) {
                    Some(Ok(n)) => timestamp = Some(n),
                    Some(Err(e)) => return Err(e.description().to_string()),
                    None => timestamp = None,
                },
                Some("oauth_token") => token = parts.next().map(str::to_string),
                Some("oauth_callback") => callback = parts.next().map(str::to_string),
                Some(_) => return Err("unexpected OAuth Authorization header field".to_string()),
                None => return Err("unexpected header format".to_string()),
            }
        }

        Ok(TwitterOAuth {
            consumer_key: try!(consumer_key.ok_or("no oauth_consumer_key")),
            nonce: try!(nonce.ok_or("no oauth_nonce")),
            signature: try!(signature.ok_or("no oauth_signature")),
            timestamp: try!(timestamp.ok_or("no oauth_timestamp")),
            token: token,
            callback: callback,
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

        try!(write!(f, ", oauth_signature=\"{}\"", percent_encode(&self.signature)));

        try!(write!(f, ", oauth_signature_method=\"{}\"", percent_encode("HMAC-SHA1")));

        try!(write!(f, ", oauth_timestamp=\"{}\"", self.timestamp));

        if let Some(ref token) = self.token {
            try!(write!(f, ", oauth_token=\"{}\"", percent_encode(token)));
        }

        try!(write!(f, ", oauth_version=\"{}\"", "1.0"));

        if let Some(ref callback) = self.callback {
            try!(write!(f, ", oauth_callback=\"{}\"", percent_encode(callback)));
        }

        Ok(())
    }
}
