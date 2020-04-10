// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Infrastructure types related to packaging rate-limit information alongside responses from
//! Twitter.

use crate::error::Error::{self, *};
use crate::error::{Result, TwitterErrors};

use hyper::client::ResponseFuture;
use hyper::{self, Body, Request};
#[cfg(feature = "hyper-rustls")]
use hyper_rustls::HttpsConnector;
#[cfg(feature = "native_tls")]
use hyper_tls::HttpsConnector;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json;

use std::convert::TryFrom;

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
#[derive(
    Debug, Deserialize, derive_more::Constructor, derive_more::Deref, derive_more::DerefMut,
)]
pub struct Response<T> {
    /// Latest rate lime status
    #[serde(flatten)]
    pub rate_limit_status: RateLimit,
    ///The decoded response from the request.
    #[deref]
    #[deref_mut]
    #[serde(default)]
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

pub fn get_response(request: Request<Body>) -> ResponseFuture {
    let connector = HttpsConnector::new();
    let client = hyper::Client::builder().build(connector);
    client.request(request)
}

pub(crate) async fn twitter_raw_request(request: Request<Body>) -> Result<(Headers, Vec<u8>)> {
    let connector = HttpsConnector::new();
    let client = hyper::Client::builder().build(connector);
    let resp = client.request(request).await?;
    let (parts, body) = resp.into_parts();
    let body: Vec<_> = hyper::body::to_bytes(body).await?.to_vec();
    println!("{:?}", String::from_utf8_lossy(&body));
    if let Ok(errors) = serde_json::from_slice::<TwitterErrors>(&body) {
        if errors.errors.iter().any(|e| e.code == 88)
            && parts.headers.contains_key(X_RATE_LIMIT_RESET)
        {
            return Err(RateLimit(rate_limit_reset(&parts.headers)?.unwrap()));
        } else {
            return Err(TwitterError(parts.headers, errors));
        }
    }
    if !parts.status.is_success() {
        return Err(BadStatus(parts.status));
    }
    Ok((parts.headers, body))
}

/// Shortcut `MakeResponse` method that attempts to parse the given type from the response and
/// loads rate-limit information from the response headers.
pub(crate) async fn twitter_json_request<T: DeserializeOwned>(
    request: Request<Body>,
) -> Result<Response<T>> {
    let (headers, body) = twitter_raw_request(request).await?;
    let response = serde_json::from_slice(&body)?;
    let rate_limit_status = RateLimit::try_from(&headers)?;
    Ok(Response {
        rate_limit_status,
        response,
    })
}

/// Shortcut function to create a `TwitterFuture` that parses out the given type from its response.
pub async fn make_parsed_future<T: for<'de> Deserialize<'de>>(
    request: Request<Body>,
) -> Result<Response<T>> {
    twitter_json_request(request).await
}

#[derive(Copy, Clone, Debug, Deserialize)]
pub struct RateLimit {
    ///The rate limit ceiling for the given request.
    pub limit: i32,
    ///The number of requests left for the 15-minute window.
    pub remaining: i32,
    ///The UTC Unix timestamp at which the rate window resets.
    pub reset: i32,
}

impl TryFrom<&Headers> for RateLimit {
    type Error = Error;
    fn try_from(headers: &Headers) -> Result<Self> {
        Ok(Self {
            limit: rate_limit_limit(headers)?.unwrap_or(-1),
            remaining: rate_limit_remaining(headers)?.unwrap_or(-1),
            reset: rate_limit_reset(headers)?.unwrap_or(-1),
        })
    }
}
