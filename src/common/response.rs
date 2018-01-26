// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Infrastructure types related to packaging rate-limit information alongside responses from
//! Twitter.

use std::{slice, vec, io, mem};
use std::iter::FromIterator;
use std::ops::{Deref, DerefMut};
use hyper::client::FutureResponse;
use hyper::{self, Body, StatusCode, Request};
use hyper::header::{Headers, ContentLength};
use hyper_tls::HttpsConnector;
use tokio_core::reactor::Handle;
use futures::{Async, Future, Poll, Stream};
use rustc_serialize::json;
use error::{self, TwitterErrors};
use error::Error::*;
use serde;
use serde_json;

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
#[derive(Debug, Deserialize)]
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
        } else {
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
        } else {
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
        } else {
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
        } else {
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
        } else {
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
        } else {
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
            } else if (item.rate_limit_reset == resp.rate_limit_reset) &&
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

pub fn get_response(handle: &Handle, request: Request) -> Result<FutureResponse, error::Error> {
    // TODO: num-cpus?
    let connector = try!(HttpsConnector::new(1, handle));
    let client = hyper::Client::configure().connector(connector).build(handle);
    Ok(client.request(request))
}

/// A `Future` that resolves a web request and loads the complete response into a String.
///
/// This also does some header inspection, and attempts to parse the response as a `TwitterErrors`
/// before returning the String.
#[must_use = "futures do nothing unless polled"]
pub struct RawFuture {
    handle: Handle,
    request: Option<Request>,
    response: Option<FutureResponse>,
    resp_headers: Option<Headers>,
    resp_status: Option<StatusCode>,
    body_stream: Option<Body>,
    body: Vec<u8>,
}

impl RawFuture {
    fn headers(&self) -> &Headers {
        self.resp_headers.as_ref().unwrap()
    }
}

impl Future for RawFuture {
    type Item = String;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if let Some(req) = self.request.take() {
            // needed to pull this section into the future so i could try!() on the connector
            self.response = Some(try!(get_response(&self.handle, req)));
        }

        if let Some(mut resp) = self.response.take() {
            match resp.poll() {
                Err(e) => return Err(e.into()),
                Ok(Async::NotReady) => {
                    self.response = Some(resp);
                    return Ok(Async::NotReady);
                }
                Ok(Async::Ready(resp)) => {
                    self.resp_headers = Some(resp.headers().clone());
                    self.resp_status = Some(resp.status());
                    if let Some(len) = resp.headers().get::<ContentLength>() {
                        self.body.reserve(len.0 as usize);
                    }
                    self.body_stream = Some(resp.body());
                }
            }
        }

        if let Some(mut resp) = self.body_stream.take() {
            loop {
                match resp.poll() {
                    Err(e) => return Err(e.into()),
                    Ok(Async::NotReady) => {
                        self.body_stream = Some(resp);
                        return Ok(Async::NotReady);
                    }
                    Ok(Async::Ready(None)) => break,
                    Ok(Async::Ready(Some(chunk))) => {
                        self.body.extend(&*chunk);
                    }
                }
            }
        } else {
            return Err(FutureAlreadyCompleted);
        };

        match String::from_utf8(mem::replace(&mut self.body, Vec::new())) {
            Err(_) => Err(io::Error::new(io::ErrorKind::InvalidData,
                                         "stream did not contain valid UTF-8").into()),
            Ok(resp) => {
                if let Ok(err) = json::decode::<TwitterErrors>(&resp) {
                    if err.errors.iter().any(|e| e.code == 88) &&
                        self.headers().has::<XRateLimitReset>()
                    {
                        return Err(
                            RateLimit(
                                self.headers().get::<XRateLimitReset>().map(|h| h.0).unwrap()
                            )
                        );
                    } else {
                        return Err(TwitterError(err));
                    }
                }

                match self.resp_status.unwrap() {
                    st if st.is_success() => Ok(Async::Ready(resp)),
                    st => Err(BadStatus(st)),
                }
            }
        }
    }
}

/// Creates a new `RawFuture` starting with the given `Request`, to be run on the Core represented
/// by the given `Handle`.
pub fn make_raw_future(handle: &Handle, request: Request) -> RawFuture {
    RawFuture {
        handle: handle.clone(),
        request: Some(request),
        response: None,
        resp_headers: None,
        resp_status: None,
        body_stream: None,
        body: Vec::new(),
    }
}


/// A `Future` that will resolve to a complete Twitter response.
///
/// When this `Future` is fully complete, the pending web request will have successfully completed,
/// loaded, and parsed into the desired response. Any errors encountered along the way will be
/// reflected in the return type of `poll`.
///
/// For more information on how to use `Future`s, see the guides at [hyper.rs] and [tokio.rs].
///
/// [hyper.rs]: https://hyper.rs/guides/
/// [tokio.rs]: https://tokio.rs/docs/getting-started/tokio/
///
/// Most functions in this library use the type alias [`FutureResponse`][], which is a
/// `TwitterFuture` that has a [`Response`][] around its item.
///
/// [`FutureResponse`]: type.FutureResponse.html
/// [`Response`]: struct.Response.html
#[must_use = "futures do nothing unless polled"]
pub struct TwitterFuture<T> {
    request: RawFuture,
    make_resp: fn(String, &Headers) -> Result<T, error::Error>,
}

impl<T> Future for TwitterFuture<T> {
    type Item = T;
    type Error = error::Error;

     fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
         let full_resp = match self.request.poll() {
             Err(e) => return Err(e),
             Ok(Async::NotReady) => return Ok(Async::NotReady),
             Ok(Async::Ready(r)) => r,
         };

         Ok(Async::Ready(try!((self.make_resp)(full_resp, self.request.headers()))))
     }
}

/// Shortcut `MakeResponse` method that attempts to parse the given type from the response and
/// loads rate-limit information from the response headers.
pub fn make_response_serde<T: for <'a> serde::Deserialize<'a>>(full_resp: String, headers: &Headers)
    -> Result<Response<T>, error::Error>
{
    let out = serde_json::from_str(&full_resp)?; // TODO OK to use ?
    Ok(Response::map(rate_headers(headers), |_| out))
}

pub fn make_future<T>(handle: &Handle,
                      request: Request,
                      make_resp: fn(String, &Headers) -> Result<T, error::Error>)
    -> TwitterFuture<T>
{
    TwitterFuture {
        request: make_raw_future(handle, request),
        make_resp: make_resp,
    }
}

/// Shortcut function to create a `TwitterFuture` that parses out the given type from its response.
pub fn make_parsed_future_serde<T: for <'de> serde::Deserialize<'de>>(handle: &Handle, request: Request)
    -> TwitterFuture<Response<T>>
{
    make_future(handle, request, make_response_serde)
}

pub fn rate_headers(resp: &Headers) -> Response<()> {
    Response {
        rate_limit: resp.get::<XRateLimitLimit>().map_or(-1, |h| h.0),
        rate_limit_remaining: resp.get::<XRateLimitRemaining>().map_or(-1, |h| h.0),
        rate_limit_reset: resp.get::<XRateLimitReset>().map_or(-1, |h| h.0),
        response: (),
    }
}
