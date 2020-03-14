// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Infrastructure types related to packaging rate-limit information alongside responses from
//! Twitter.

use crate::error::Error::{self, *};
use crate::error::{Result, TwitterErrors};

use futures::Stream;
use hyper::client::ResponseFuture;
use hyper::header::CONTENT_LENGTH;
use hyper::{self, Body, Request, StatusCode};
#[cfg(feature = "hyper-rustls")]
use hyper_rustls::HttpsConnector;
#[cfg(feature = "native_tls")]
use hyper_tls::HttpsConnector;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json;

use std::convert::TryFrom;
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::{io, mem};

use super::Headers;

const X_RATE_LIMIT_LIMIT: &'static str = "X-Rate-Limit-Limit";
const X_RATE_LIMIT_REMAINING: &'static str = "X-Rate-Limit-Remaining";
const X_RATE_LIMIT_RESET: &'static str = "X-Rate-Limit-Reset";

fn rate_limit(headers: &Headers, header: &'static str) -> Result<Option<i32>> {
    let val = headers.get(header);

    if let Some(val) = val {
        let val = val.to_str()?.parse::<i32>()?;
        Ok(Some(val))
    } else {
        Ok(None)
    }
}

fn rate_limit_limit(headers: &Headers) -> Result<Option<i32>> {
    rate_limit(headers, X_RATE_LIMIT_LIMIT)
}

fn rate_limit_remaining(headers: &Headers) -> Result<Option<i32>> {
    rate_limit(headers, X_RATE_LIMIT_REMAINING)
}

fn rate_limit_reset(headers: &Headers) -> Result<Option<i32>> {
    rate_limit(headers, X_RATE_LIMIT_RESET)
}

///A helper struct to wrap response data with accompanying rate limit information.
///
///This is returned by any function that calls a rate-limited method on Twitter, to allow for
///inline checking of the rate-limit information without an extra call to
///`service::rate_limit_info`.
///
///As this implements `Deref` and `DerefMut`, you can transparently use the contained `response`'s
///methods as if they were methods on this struct.
#[derive(Debug, Deserialize, derive_more::Constructor)]
pub struct Response<T> {
    /// Latest rate lime status
    pub rate_limit_status: RateLimit,
    ///The decoded response from the request.
    pub response: T,
}

impl Response<()> {
    pub(crate) fn unit(headers: &Headers) -> Result<Self> {
        Ok(Self {
            rate_limit_status: RateLimit::try_from(headers)?,
            response: (),
        })
    }
}

impl<T> Response<T> {
    ///Convert a `Response<T>` to a `Response<U>` by running its contained response through the
    ///given function. This preserves its rate-limit information.
    ///
    ///Note that this is not a member function, so as to not conflict with potential methods on the
    ///contained `T`.
    pub fn map<F, U>(src: Response<T>, fun: F) -> Response<U>
    where
        F: FnOnce(T) -> U,
    {
        Response {
            rate_limit_status: src.rate_limit_status,
            response: fun(src.response),
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

pub fn get_response(request: Request<Body>) -> ResponseFuture {
    // TODO: num-cpus?
    let connector = HttpsConnector::new();
    let client = hyper::Client::builder().build(connector);
    client.request(request)
}

/// A `Future` that resolves a web request and loads the complete response into a String.
///
/// This also does some header inspection, and attempts to parse the response as a `TwitterErrors`
/// before returning the String.
#[must_use = "futures do nothing unless polled"]
pub struct RawFuture {
    request: Option<Request<Body>>,
    response: Option<ResponseFuture>,
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
    type Output = Result<String>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        if let Some(req) = self.request.take() {
            // needed to pull this section into the future so i could try!() on the connector
            self.response = Some(get_response(req));
        }

        if let Some(mut resp) = self.response.take() {
            match Pin::new(&mut resp).poll(cx) {
                Poll::Pending => {
                    self.response = Some(resp);
                    return Poll::Pending;
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e.into())),
                Poll::Ready(Ok(resp)) => {
                    self.resp_headers = Some(resp.headers().clone());
                    self.resp_status = Some(resp.status());
                    if let Some(len) = resp.headers().get(CONTENT_LENGTH) {
                        if let Ok(len) = len.to_str() {
                            if let Ok(len) = len.parse::<usize>() {
                                self.body.reserve(len);
                            }
                        }
                    }
                    self.body_stream = Some(resp.into_body());
                }
            }
        }

        if let Some(mut resp) = self.body_stream.take() {
            loop {
                match Pin::new(&mut resp).poll_next(cx) {
                    Poll::Pending => {
                        self.body_stream = Some(resp);
                        return Poll::Pending;
                    }
                    Poll::Ready(None) => break,
                    Poll::Ready(Some(Err(e))) => return Poll::Ready(Err(e.into())),
                    Poll::Ready(Some(Ok(chunk))) => {
                        self.body.extend(&*chunk);
                    }
                }
            }
        } else {
            return Poll::Ready(Err(FutureAlreadyCompleted));
        };

        match String::from_utf8(mem::replace(&mut self.body, Vec::new())) {
            Err(_) => Poll::Ready(Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "stream did not contain valid UTF-8",
            )
            .into())),
            Ok(resp) => {
                if let Ok(err) = serde_json::from_str::<TwitterErrors>(&resp) {
                    if err.errors.iter().any(|e| e.code == 88)
                        && self.headers().contains_key(X_RATE_LIMIT_RESET)
                    {
                        return Poll::Ready(Err(RateLimit(
                            rate_limit_reset(self.headers())?.unwrap(),
                        )));
                    } else {
                        return Poll::Ready(Err(TwitterError(err)));
                    }
                }

                let st = self.resp_status.unwrap();
                if st.is_success() {
                    Poll::Ready(Ok(resp))
                } else {
                    Poll::Ready(Err(BadStatus(st)))
                }
            }
        }
    }
}

/// Creates a new `RawFuture` starting with the given `Request`.
pub fn make_raw_future(request: Request<Body>) -> RawFuture {
    RawFuture {
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
    make_resp: fn(String, &Headers) -> Result<T>,
}

impl<T> Future for TwitterFuture<T> {
    type Output = Result<T>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let full_resp = match Pin::new(&mut self.request).poll(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
            Poll::Ready(Ok(r)) => r,
        };

        Poll::Ready(Ok((self.make_resp)(full_resp, self.request.headers())?))
    }
}

/// Shortcut `MakeResponse` method that attempts to parse the given type from the response and
/// loads rate-limit information from the response headers.
pub fn make_response<T: DeserializeOwned>(body: String, headers: &Headers) -> Result<Response<T>> {
    let response = serde_json::from_str(&body)?;
    let rate_limit_status = RateLimit::try_from(headers)?;
    Ok(Response {
        rate_limit_status,
        response,
    })
}

pub async fn make_future<T>(
    request: Request<Body>,
    make_resp: fn(String, &Headers) -> Result<T>,
) -> Result<T> {
    TwitterFuture {
        request: make_raw_future(request),
        make_resp: make_resp,
    }
    .await
}

/// Shortcut function to create a `TwitterFuture` that parses out the given type from its response.
pub async fn make_parsed_future<T: for<'de> Deserialize<'de>>(
    request: Request<Body>,
) -> Result<Response<T>> {
    make_future(request, make_response).await
}

#[derive(Clone, Debug, Deserialize)]
pub struct RateLimit {
    ///The rate limit ceiling for the given request.
    pub rate_limit: i32,
    ///The number of requests left for the 15-minute window.
    pub rate_limit_remaining: i32,
    ///The UTC Unix timestamp at which the rate window resets.
    pub rate_limit_reset: i32,
}

impl TryFrom<&Headers> for RateLimit {
    type Error = Error;
    fn try_from(headers: &Headers) -> Result<Self> {
        Ok(Self {
            rate_limit: rate_limit_limit(headers)?.unwrap_or(-1),
            rate_limit_remaining: rate_limit_remaining(headers)?.unwrap_or(-1),
            rate_limit_reset: rate_limit_reset(headers)?.unwrap_or(-1),
        })
    }
}
