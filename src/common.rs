use std::borrow::Cow;
use std::collections::HashMap;
use std::{fmt, vec};
use std::io::Read;
use std::iter::FromIterator;
use std::marker::PhantomData;
use url::percent_encoding::{EncodeSet, utf8_percent_encode};
use hyper::client::response::Response as HyperResponse;
use hyper::status::StatusCode;
use rustc_serialize::json;
use error;
use error::Error::*;
use auth;

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

pub fn add_name_param<'a>(list: &mut ParamList<'a>, id: &UserID<'a>) -> Option<Cow<'a, str>> {
    match id {
        &UserID::ID(id) => add_param(list, "user_id", id.to_string()),
        &UserID::ScreenName(name) => add_param(list, "screen_name", name),
    }
}

///Convenience enum to generalize between referring to an account by numeric ID or by screen name.
pub enum UserID<'a> {
    ///Referring via the account's numeric ID.
    ID(i64),
    ///Referring via the account's screen name.
    ScreenName(&'a str),
}

impl<'a> From<i64> for UserID<'a> {
    fn from(id: i64) -> UserID<'a> {
        UserID::ID(id)
    }
}

impl<'a> From<&'a str> for UserID<'a> {
    fn from(name: &'a str) -> UserID<'a> {
        UserID::ScreenName(name)
    }
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
    ///The UTC Unix timestamp at which the rate window resets.
    pub rate_limit_reset: i32,
    ///The decoded response from the request.
    pub response: T,
}

///Iterator returned by calling `.into_iter()` on a `Response<Vec<T>>`.
pub struct ResponseIter<T> {
    rate_limit: i32,
    rate_limit_remaining: i32,
    rate_limit_reset: i32,
    resp_iter: vec::IntoIter<T>,
}

impl<T> ResponseIter<T> {
    pub fn len(&self) -> usize {
        self.resp_iter.len()
    }
}

impl<T> Iterator for ResponseIter<T> {
    type Item = Response<T>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(resp) = self.resp_iter.next() {
            Some(Response {
                rate_limit: self.rate_limit,
                rate_limit_remaining: self.rate_limit_remaining,
                rate_limit_reset: self.rate_limit_reset,
                response: resp,
            })
        }
        else {
            None
        }
    }
}

impl<T> IntoIterator for Response<Vec<T>> {
    type Item = Response<T>;
    type IntoIter = ResponseIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        ResponseIter {
            rate_limit: self.rate_limit,
            rate_limit_remaining: self.rate_limit_remaining,
            rate_limit_reset: self.rate_limit_reset,
            resp_iter: self.response.into_iter(),
        }
    }
}

impl<T> FromIterator<Response<T>> for Response<Vec<T>> {
    fn from_iter<I>(iter: I) -> Self
        where I: IntoIterator<Item=Response<T>>
    {
        let mut resp = Response {
            rate_limit: -1,
            rate_limit_remaining: -1,
            rate_limit_reset: -1,
            response: Vec::new(),
        };

        for item in iter {
            resp.rate_limit = item.rate_limit;
            resp.rate_limit_remaining = item.rate_limit_remaining;
            resp.rate_limit_reset = item.rate_limit_reset;
            resp.response.push(item.response);
        }

        resp
    }
}

///Represents a collection of errors returned from a Twitter API call.
#[derive(Debug, RustcDecodable, RustcEncodable)]
pub struct TwitterErrors {
    ///A collection of errors returned by Twitter.
    pub errors: Vec<TwitterErrorCode>,
}

impl fmt::Display for TwitterErrors {
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
    ///The error message returned by Twitter.
    pub message: String,
    ///The numeric error code returned by Twitter. A list of possible error codes can be found in
    ///the [API documentation][error-codes].
    ///
    ///[error-codes]: https://dev.twitter.com/overview/api/response-codes
    pub code: i32,
}

impl fmt::Display for TwitterErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{}: {}", self.code, self.message)
    }
}

///Helper trait to provide a general interface for deserializing Twitter API data structures.
pub trait FromJson : Sized {
    ///Parse the given Json object into a data structure.
    fn from_json(&json::Json) -> Result<Self, error::Error>;

    ///Parse the given string into a Json object, then into a data structure.
    fn from_str(input: &str) -> Result<Self, error::Error> {
        let json = try!(json::Json::from_str(input));

        Self::from_json(&json)
    }
}

impl<T> FromJson for Vec<T> where T: FromJson {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        let arr = try!(input.as_array().ok_or(InvalidResponse));

        arr.iter().map(|x| T::from_json(x)).collect()
    }
}

impl FromJson for i64 {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        input.as_i64().ok_or(InvalidResponse)
    }
}

impl FromJson for i32 {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        input.as_i64().map(|x| x as i32).ok_or(InvalidResponse)
    }
}

impl FromJson for String {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        input.as_string().map(|s| s.to_string()).ok_or(InvalidResponse)
    }
}

impl FromJson for bool {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        input.as_boolean().ok_or(InvalidResponse)
    }
}

impl FromJson for (i32, i32) {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        //assumptions: input is
        // - an array
        // - of integers
        // - with exactly two entries
        //any deviation from these assumptions will return an error.
        let int_vec = try!(input.as_array()
                                .ok_or(InvalidResponse)
                                .and_then(|v| v.iter()
                                               .map(|i| i.as_i64())
                                               .collect::<Option<Vec<_>>>()
                                               .ok_or(InvalidResponse)));

        if int_vec.len() != 2 {
            return Err(InvalidResponse);
        }

        Ok((int_vec[0] as i32, int_vec[1] as i32))
    }
}

///Trait to generalize over paginated views of API results.
pub trait Cursor {
    ///What type is being returned by the API call?
    type Item;

    ///Returns a numeric reference to the previous page of results.
    fn previous_cursor_id(&self) -> i64;
    ///Returns a numeric reference to the next page of results.
    fn next_cursor_id(&self) -> i64;
    ///Consumes the cursor and returns the collection of results from inside.
    fn into_inner(self) -> Vec<Self::Item>;
}

///Represents a paginated list of results, such as the users who follow a specific user or the
///lists owned by that user.
///
///Implemented as an iterator that lazily loads a page of results at a time, but returns a single
///item per-iteration. See examples in [the user module-level documentation][user-mod].
///
///[user-mod]: user/index.html
pub struct CursorIter<'a, T>
    where T: Cursor + FromJson
{
    link: &'static str,
    con_token: &'a auth::Token<'a>,
    access_token: &'a auth::Token<'a>,
    user_id: Option<UserID<'a>>,
    ///The number of results returned in one network call.
    ///
    ///Certain calls set their own minimums and maximums for what this value can be. Furthermore,
    ///some calls don't allow you to set the size of the pages at all. Refer to the individual
    ///methods' documentation for specifics.
    pub page_size: Option<i32>,
    ///Numeric reference to the previous page of results.
    ///
    ///This value is intended to be automatically set and used as part of this struct's Iterator
    ///implementation. It is made available for those who wish to manually manage network calls and
    ///pagination.
    pub previous_cursor: i64,
    ///Numeric reference to the next page of results. A value of zero indicates that the current
    ///page of results is the last page of the cursor.
    ///
    ///This value is intended to be automatically set and used as part of this struct's Iterator
    ///implementation. It is made available for those who wish to manually manage network calls and
    ///pagination.
    pub next_cursor: i64,
    iter: Option<ResponseIter<T::Item>>,
    _marker: PhantomData<T>,
}

impl<'a, T> CursorIter<'a, T>
    where T: Cursor + FromJson
{
    ///Sets the number of results returned in a single network call.
    ///
    ///Certain calls set their own minimums and maximums for what this value can be. Furthermore,
    ///some calls don't allow you to set the size of the pages at all. Refer to the individual
    ///methods' documentation for specifics. If this method is called for a response that does not
    ///accept changing the page size, no change to the underlying struct will occur.
    ///
    ///Calling this function will invalidate any current results, if any were previously loaded.
    pub fn with_page_size(self, page_size: i32) -> CursorIter<'a, T> {
        if self.page_size.is_some() {
            CursorIter {
                link: self.link,
                con_token: self.con_token,
                access_token: self.access_token,
                user_id: self.user_id,
                page_size: Some(page_size),
                previous_cursor: -1,
                next_cursor: -1,
                iter: None,
                _marker: self._marker,
            }
        }
        else {
            self
        }
    }

    ///Loads the next page of results.
    ///
    ///This is intended to be used as part of this struct's Iterator implementation. It is provided
    ///as a convenience for those who wish to manage network calls and pagination manually.
    pub fn call(&self) -> Result<Response<T>, error::Error> {
        let mut params = HashMap::new();
        if let Some(ref id) = self.user_id {
            add_name_param(&mut params, id);
        }
        add_param(&mut params, "cursor", self.next_cursor.to_string());
        if let Some(count) = self.page_size {
            add_param(&mut params, "count", count.to_string());
        }

        let mut resp = try!(auth::get(self.link, self.con_token, self.access_token, Some(&params)));

        parse_response(&mut resp)
    }

    ///Creates a new instance of CursorIter, with the given parameters and empty initial results.
    pub fn new(link: &'static str, con_token: &'a auth::Token, access_token: &'a auth::Token,
               user_id: Option<UserID<'a>>, page_size: Option<i32>) -> CursorIter<'a, T> {
        CursorIter {
            link: link,
            con_token: con_token,
            access_token: access_token,
            user_id: user_id,
            page_size: page_size,
            previous_cursor: -1,
            next_cursor: -1,
            iter: None,
            _marker: PhantomData,
        }
    }
}

impl<'a, T> Iterator for CursorIter<'a, T>
    where T: Cursor + FromJson
{
    type Item = Result<Response<T::Item>, error::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ref mut results) = self.iter {
            if let Some(item) = results.next() {
                return Some(Ok(item));
            }
            else if self.next_cursor == 0 {
                return None;
            }
        }

        match self.call() {
            Ok(resp) => {
                self.previous_cursor = resp.response.previous_cursor_id();
                self.next_cursor = resp.response.next_cursor_id();

                let resp = Response {
                    rate_limit: resp.rate_limit,
                    rate_limit_remaining: resp.rate_limit_remaining,
                    rate_limit_reset: resp.rate_limit_reset,
                    response: resp.response.into_inner(),
                };

                let mut iter = resp.into_iter();
                let first = iter.next();
                self.iter = Some(iter);

                match first {
                    Some(item) => Some(Ok(item)),
                    None => None,
                }
            },
            Err(err) => Some(Err(err)),
        }
    }
}

///With the given response struct, parse it into a String.
pub fn response_raw(resp: &mut HyperResponse) -> Result<String, error::Error> {
    let mut full_resp = String::new();
    try!(resp.read_to_string(&mut full_resp));

    if let Ok(err) = json::decode::<TwitterErrors>(&full_resp) {
        return Err(TwitterError(err));
    }

    match resp.status {
        StatusCode::Ok | StatusCode::NotModified => (),
        _ => return Err(BadStatus(resp.status)),
    }

    Ok(full_resp)
}

///With the given response struct, parse it into the desired format and
///return it along with rate limit information.
pub fn parse_response<T>(resp: &mut HyperResponse) -> Result<Response<T>, error::Error>
    where T: FromJson
{
    let resp_str = try!(response_raw(resp));

    Ok(Response {
        rate_limit: resp.headers.get::<XRateLimitLimit>().map(|h| h.0).unwrap_or(-1),
        rate_limit_remaining: resp.headers.get::<XRateLimitRemaining>().map(|h| h.0).unwrap_or(-1),
        rate_limit_reset: resp.headers.get::<XRateLimitReset>().map(|h| h.0).unwrap_or(-1),
        response: try!(T::from_str(&resp_str)),
    })
}

pub fn field<T: FromJson>(input: &json::Json, field: &'static str) -> Result<T, error::Error> {
    T::from_json(try!(input.find(field).ok_or(MissingValue(field))))
}
