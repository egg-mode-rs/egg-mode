// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Infrastructure types related to packaging rate-limit information alongside responses from
//! Twitter.

use std::{slice, vec};
use std::iter::FromIterator;
use std::io::Read;
use std::ops::{Deref, DerefMut};
use hyper::client::response::Response as HyperResponse;
use hyper::status::StatusCode;
use rustc_serialize::json;
use super::{FromJson, field};
use error::{self, TwitterErrors};
use error::Error::*;

header! { (XRateLimitLimit, "X-Rate-Limit-Limit") => [i32] }
header! { (XRateLimitRemaining, "X-Rate-Limit-Remaining") => [i32] }
header! { (XRateLimitReset, "X-Rate-Limit-Reset") => [i32] }

///A helper struct to wrap response data with accompanying rate limit information.
///
///This is returned by any function that calls a rate-limited method on Twitter, to allow for
///inline checking of the rate-limit information without an extra call to
///`service::rate_limit_info`.
///
///As this implements `Deref` and `DerefMut`, you can transparently use the contained `response`'s
///methods as if they were methods on this struct.
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

impl<T> Response<T> {
    ///Convert a `Response<T>` to a `Response<U>` by running its contained response through the
    ///given function. This preserves its rate-limit information.
    ///
    ///Note that this is not a member function, so as to not conflict with potential methods on the
    ///contained `T`.
    pub fn map<F, U>(src: Response<T>, fun: F) -> Response<U>
        where F: FnOnce(T) -> U
    {
        Response {
            rate_limit: src.rate_limit,
            rate_limit_remaining: src.rate_limit_remaining,
            rate_limit_reset: src.rate_limit_reset,
            response: fun(src.response)
        }
    }
}

impl<T> Response<Vec<T>> {
    ///Returns an iterator that yields references into the returned collection, alongside
    ///rate-limit information for the whole method call.
    pub fn iter(&self) -> ResponseIterRef<T> {
        ResponseIterRef {
            rate_limit: self.rate_limit,
            rate_limit_remaining: self.rate_limit_remaining,
            rate_limit_reset: self.rate_limit_reset,
            resp_iter: self.response.iter(),
        }
    }

    ///Returns an iterator that yields mutable references into the returned collection, alongside
    ///rate-limit information for the whole method call.
    pub fn iter_mut(&mut self) -> ResponseIterMut<T> {
        ResponseIterMut {
            rate_limit: self.rate_limit,
            rate_limit_remaining: self.rate_limit_remaining,
            rate_limit_reset: self.rate_limit_reset,
            resp_iter: self.response.iter_mut(),
        }
    }
}

//This impl is used for service::rate_limit_status, to represent the individual method statuses
impl FromJson for Response<()> {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse("Response<()> received json that wasn't an object",
                                       Some(input.to_string())));
        }

        field_present!(input, limit);
        field_present!(input, remaining);
        field_present!(input, reset);

        Ok(Response {
            rate_limit: try!(field(input, "limit")),
            rate_limit_remaining: try!(field(input, "remaining")),
            rate_limit_reset: try!(field(input, "reset")),
            response: (),
        })
    }
}

impl<T> Deref for Response<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.response
    }
}

impl<T> DerefMut for Response<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.response
    }
}

///Iterator returned by calling `.iter()` on a `Response<Vec<T>>`.
///
///This provides a convenient method to iterate over a response that returned a collection, while
///copying rate-limit information across the entire iteration.
pub struct ResponseIterRef<'a, T> where T: 'a {
    rate_limit: i32,
    rate_limit_remaining: i32,
    rate_limit_reset: i32,
    resp_iter: slice::Iter<'a, T>,
}

impl<'a, T> Iterator for ResponseIterRef<'a, T> where T: 'a {
    type Item = Response<&'a T>;

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

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.resp_iter.size_hint()
    }
}

impl<'a, T> DoubleEndedIterator for ResponseIterRef<'a, T> where T: 'a {
    fn next_back(&mut self) -> Option<Self::Item> {
        if let Some(resp) = self.resp_iter.next_back() {
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

impl<'a, T> ExactSizeIterator for ResponseIterRef<'a, T> where T: 'a {
    fn len(&self) -> usize {
        self.resp_iter.len()
    }
}

///Iteration over a response that returned a collection, while leaving the response in place.
impl<'a, T> IntoIterator for &'a Response<Vec<T>> where T: 'a {
    type Item = Response<&'a T>;
    type IntoIter = ResponseIterRef<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

///Iterator returned by calling `.iter_mut()` on a `Response<Vec<T>>`.
///
///This provides a convenient method to iterate over a response that returned a collection, while
///copying rate-limit information across the entire iteration.
pub struct ResponseIterMut<'a, T> where T: 'a {
    rate_limit: i32,
    rate_limit_remaining: i32,
    rate_limit_reset: i32,
    resp_iter: slice::IterMut<'a, T>,
}

impl<'a, T> Iterator for ResponseIterMut<'a, T> where T: 'a {
    type Item = Response<&'a mut T>;

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

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.resp_iter.size_hint()
    }
}

impl<'a, T> DoubleEndedIterator for ResponseIterMut<'a, T> where T: 'a {
    fn next_back(&mut self) -> Option<Self::Item> {
        if let Some(resp) = self.resp_iter.next_back() {
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

impl<'a, T> ExactSizeIterator for ResponseIterMut<'a, T> where T: 'a {
    fn len(&self) -> usize {
        self.resp_iter.len()
    }
}

///Mutable iteration over a response that returned a collection, while leaving the response in
///place.
impl<'a, T> IntoIterator for &'a mut Response<Vec<T>> where T: 'a {
    type Item = Response<&'a mut T>;
    type IntoIter = ResponseIterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

///Iterator returned by calling `.into_iter()` on a `Response<Vec<T>>`.
///
///This provides a convenient method to iterate over a response that returned a collection, while
///copying rate-limit information across the entire iteration. For example, this is used in
///`CursorIter`'s implemention to propagate rate-limit information across a given page of results.
pub struct ResponseIter<T> {
    rate_limit: i32,
    rate_limit_remaining: i32,
    rate_limit_reset: i32,
    resp_iter: vec::IntoIter<T>,
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

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.resp_iter.size_hint()
    }
}

impl<T> DoubleEndedIterator for ResponseIter<T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        if let Some(resp) = self.resp_iter.next_back() {
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

impl<T> ExactSizeIterator for ResponseIter<T> {
    fn len(&self) -> usize {
        self.resp_iter.len()
    }
}

///Iteration over a response that returned a collection, copying the rate limit information across
///all values.
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

///`FromIterator` impl that allows collecting several responses into one, preserving the latest
///rate limit information.
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
            if item.rate_limit_reset > resp.rate_limit_reset {
                resp.rate_limit = item.rate_limit;
                resp.rate_limit_remaining = item.rate_limit_remaining;
                resp.rate_limit_reset = item.rate_limit_reset;
            }
            else if (item.rate_limit_reset == resp.rate_limit_reset) &&
                    (item.rate_limit_remaining < resp.rate_limit_remaining) {
                resp.rate_limit = item.rate_limit;
                resp.rate_limit_remaining = item.rate_limit_remaining;
                resp.rate_limit_reset = item.rate_limit_reset;
            }
            resp.response.push(item.response);
        }

        resp
    }
}

pub fn rate_headers(resp: &HyperResponse) -> Response<()> {
    Response {
        rate_limit: resp.headers.get::<XRateLimitLimit>().map_or(-1, |h| h.0),
        rate_limit_remaining: resp.headers.get::<XRateLimitRemaining>().map_or(-1, |h| h.0),
        rate_limit_reset: resp.headers.get::<XRateLimitReset>().map_or(-1, |h| h.0),
        response: (),
    }
}

///With the given response struct, parse it into a String.
pub fn response_raw(resp: &mut HyperResponse) -> Result<String, error::Error> {
    let mut full_resp = String::new();
    try!(resp.read_to_string(&mut full_resp));

    if let Ok(err) = json::decode::<TwitterErrors>(&full_resp) {
        if err.errors.iter().any(|e| e.code == 88) {
            if resp.headers.has::<XRateLimitReset>() {
                return Err(RateLimit(resp.headers.get::<XRateLimitReset>().map(|h| h.0).unwrap()));
            }
            else {
                return Err(TwitterError(err));
            }
        }
        else {
            return Err(TwitterError(err));
        }
    }

    match resp.status {
        StatusCode::Ok => (),
        _ => return Err(BadStatus(resp.status)),
    }

    Ok(full_resp)
}

///With the given response struct, parse it into the desired format and
///return it along with rate limit information.
pub fn parse_response<T: FromJson>(resp: &mut HyperResponse) -> ::common::WebResponse<T> {
    let resp_str = try!(response_raw(resp));
    let out = try!(T::from_str(&resp_str));

    Ok(Response::map(rate_headers(resp), |_| out))
}
