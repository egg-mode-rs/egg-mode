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

// n.b. this type is re-exported at the crate root - these docs are public!
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
    /// The latest rate-limit information returned with the request.
    #[serde(flatten)]
    pub rate_limit_status: RateLimit,
    /// The decoded response from the request.
    #[deref]
    #[deref_mut]
    #[serde(default)]
    pub response: T,
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

// n.b. this function is re-exported in the `raw` module - these docs are public!
/// Converts the given request into a raw `ResponseFuture` from hyper.
pub fn get_response(request: Request<Body>) -> ResponseFuture {
    let connector = HttpsConnector::new();
    let client = hyper::Client::builder().build(connector);
    client.request(request)
}

// n.b. this function is re-exported in the `raw` module - these docs are public!
/// Loads the given request, parses the headers and response for potential errors given by Twitter,
/// and returns the headers and raw bytes returned from the response.
pub async fn raw_request(request: Request<Body>) -> Result<(Headers, Vec<u8>)> {
    let connector = HttpsConnector::new();
    let client = hyper::Client::builder().build(connector);
    let resp = client.request(request).await?;
    let (parts, body) = resp.into_parts();
    let body: Vec<_> = hyper::body::to_bytes(body).await?.to_vec();
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

// n.b. this function is re-exported in the `raw` module - these docs are public!
/// Loads the given request and parses the response as JSON into the given type, including
/// rate-limit headers.
pub async fn request_with_json_response<T: DeserializeOwned>(
    request: Request<Body>,
) -> Result<Response<T>> {
    let (headers, body) = raw_request(request).await?;
    let response = serde_json::from_slice(&body)?;
    let rate_limit_status = RateLimit::try_from(&headers)?;
    Ok(Response {
        rate_limit_status,
        response,
    })
}

// n.b. this type is exported at the crate root - these docs are public!
/// Rate limit information returned with a `Response`.
///
/// With every API call, Twitter returns information about how many times you're allowed to call
/// that endpoint, and at what point your limit refreshes and allows you to call it more. These are
/// normally passed through the response headers, and egg-mode reads for these headers when a
/// function returns a `Response<T>`. If the headers are absent for a given request, the field will
/// be `-1`.
///
/// Rate limits are tracked separately based on the kind of `Token` you're using. For Bearer tokens
/// using Application-only authentication, the rate limit is based on your application as a whole,
/// regardless of how many instances are using that token. For Access tokens, the rate limit is
/// broken down by-user, so more-active users will not use up the rate limit for less-active ones.
///
/// For more information about rate-limiting, see [Twitter's documentation about rate
/// limits][rate-limit].
///
/// [rate-limit]: https://developer.twitter.com/en/docs/basics/rate-limiting
#[derive(Copy, Clone, Debug, Deserialize)]
pub struct RateLimit {
    /// The rate limit ceiling for the given request.
    pub limit: i32,
    /// The number of requests left for the 15-minute window.
    pub remaining: i32,
    /// The UTC Unix timestamp at which the rate window resets.
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
