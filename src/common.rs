use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;
use std::io::Read;
use url::percent_encoding::{EncodeSet, utf8_percent_encode};
use hyper::client::response::Response as HyperResponse;
use hyper::status::StatusCode;
use rustc_serialize::Decodable;
use rustc_serialize::json;
use super::error;

//the encode sets in the url crate don't quite match what twitter wants,
//so i'll make up my own
#[derive(Copy, Clone)]
struct TwitterEncodeSet;

impl EncodeSet for TwitterEncodeSet {
    fn contains(&self, byte: u8) -> bool {
        match byte {
            b'a' ... b'z' => false,
            b'A' ... b'Z' => false,
            b'0' ... b'9' => false,
            b'-' | b'.' | b'_' | b'~' => false,
            _ => true
        }
    }
}

///Encodes the given string slice for transmission to Twitter.
pub fn percent_encode(src: &str) -> String {
    utf8_percent_encode(src, TwitterEncodeSet).collect::<String>()
}

///Convenience type used to hold parameters to an API call.
pub type ParamList<'a> = HashMap<Cow<'a, str>, Cow<'a, str>>;

///Convenience function to add a key/value parameter to a ParamList.
pub fn add_param<'a, K, V>(list: &mut ParamList<'a>, key: K, value: V) -> Option<Cow<'a, str>>
    where K: Into<Cow<'a, str>>,
          V: Into<Cow<'a, str>>
{
    list.insert(key.into(), value.into())
}

header! { (XRateLimitLimit, "X-Rate-Limit-Limit") => [i32] }
header! { (XRateLimitRemaining, "X-Rate-Limit-Remaining") => [i32] }
header! { (XRateLimitReset, "X-Rate-Limit-Reset") => [i32] }

///A helper struct to wrap response data with accompanying rate limit information.
#[derive(Debug)]
pub struct Response<T> {
    ///The rate limit ceiling for the given request.
    pub rate_limit: i32,
    ///The number of requests left for the 15-minute window.
    pub rate_limit_remaining: i32,
    ///The remaining window before the rate limit resets, in seconds.
    ///
    ///(The Twiter API docs say "in UTC epoch seconds", so ???)
    pub rate_limit_reset: i32,
    ///The decoded response from the request.
    pub response: T,
}

///Represents a collection of errors returned from a Twitter API call.
#[derive(Debug, RustcDecodable, RustcEncodable)]
pub struct TwitterError {
    pub errors: Vec<TwitterErrorCode>,
}

impl fmt::Display for TwitterError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut first = true;
        for e in &self.errors {
            if first { first = false; }
            else { try!(writeln!(f, ",")); }

            try!(write!(f, "{}", e));
        }

        Ok(())
    }
}

///Represents a specific error returned from a Twitter API call.
#[derive(Debug, RustcDecodable, RustcEncodable)]
pub struct TwitterErrorCode {
    pub message: String,
    pub code: i32,
}

impl fmt::Display for TwitterErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{}: {}", self.code, self.message)
    }
}

///With the given response struct, parse it into a String.
pub fn response_raw(resp: &mut HyperResponse) -> Result<String, error::Error> {
    let mut full_resp = String::new();
    try!(resp.read_to_string(&mut full_resp));

    if let Ok(err) = json::decode::<TwitterError>(&full_resp) {
        return Err(error::Error::TwitterError(err));
    }

    match resp.status {
        StatusCode::Ok | StatusCode::NotModified => (),
        _ => return Err(error::Error::BadStatus(resp.status)),
    }

    Ok(full_resp)
}

///With the given response struct, parse it into the desired format and
///return it along with rate limit information.
pub fn parse_response<T>(resp: &mut HyperResponse) -> Result<Response<T>, error::Error>
    where T: Decodable
{
    let resp_str = try!(response_raw(resp));

    Ok(Response {
        rate_limit: resp.headers.get::<XRateLimitLimit>().map(|h| h.0).unwrap_or(-1),
        rate_limit_remaining: resp.headers.get::<XRateLimitRemaining>().map(|h| h.0).unwrap_or(-1),
        rate_limit_reset: resp.headers.get::<XRateLimitReset>().map(|h| h.0).unwrap_or(-1),
        response: try!(json::decode::<T>(&resp_str)),
    })
}
