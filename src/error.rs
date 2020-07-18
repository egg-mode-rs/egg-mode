// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! A composite error type for errors that can occur while interacting with Twitter.
//!
//! Any action that crosses the network to call Twitter has many places where it can go wrong.
//! Whether it's a bad network connection, a revoked authorization token, a deleted tweet, or
//! anything in between, those errors are all represented in the (rather sprawling) [`Error`] enum.
//! Any errors direct from Twitter are represented as a collection of [`TwitterErrorCode`]s,
//! contained in a [`TwitterErrors`] wrapper, and held in the `Error::TwitterError` enum variant.
//! For more information, see the documentation for the [`Error`] enum.
//!
//! [`Error`]: enum.Error.html
//! [`TwitterErrorCode`]: struct.TwitterErrorCode.html
//! [`TwitterErrors`]: struct.TwitterErrors.html

use chrono;
use hyper;
#[cfg(feature = "native_tls")]
use native_tls;
use serde::{Deserialize, Serialize};
use serde_json;
use std::{self, fmt};
use tokio;

use crate::common::Headers;

/// Convenient alias to a Result containing a local Error type
pub type Result<T> = std::result::Result<T, Error>;

///Represents a collection of errors returned from a Twitter API call.
///
///This is returned as part of [`Error::TwitterError`][] whenever Twitter has rejected a call.
///
///[`Error::TwitterError`]: enum.Error.html
#[derive(Debug, Deserialize, Serialize, thiserror::Error)]
pub struct TwitterErrors {
    /// A collection of errors
    pub errors: Vec<TwitterErrorCode>,
}

impl fmt::Display for TwitterErrors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut first = true;
        for e in &self.errors {
            if first {
                first = false;
            } else {
                writeln!(f, ",")?;
            }

            write!(f, "{}", e)?;
        }

        Ok(())
    }
}

///Represents a specific error returned from a Twitter API call.
#[derive(Debug, Deserialize, Serialize)]
pub struct TwitterErrorCode {
    ///The error message returned by Twitter.
    pub message: String,
    ///The numeric error code returned by Twitter. A list of possible error codes can be found in
    ///the [API documentation][error-codes].
    ///
    ///[error-codes]: https://developer.twitter.com/en/docs/basics/response-codes
    pub code: i32,
}

impl fmt::Display for TwitterErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{}: {}", self.code, self.message)
    }
}

/// Represents an error that can occur during media processing.
#[derive(Debug, Clone, PartialEq, Deserialize, thiserror::Error)]
#[error("Media error {code} ({name}) - {message}")]
pub struct MediaError {
    /// A numeric error code assigned to the error.
    pub code: i32,
    /// A short name given to the error.
    pub name: String,
    /// The full text of the error message.
    pub message: String,
}

/// A set of errors that can occur when interacting with Twitter.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    ///A URL was passed to a shortcut function that didn't match the method being called.
    #[error("URL given did not match API method")]
    BadUrl,
    ///The response from Twitter was formatted incorrectly or in an unexpected manner. The enclosed
    ///values are an explanatory string and, if applicable, the input that caused the error.
    ///
    ///This usually reflects a bug in this library, as it means I'm not parsing input right.
    #[error("Invalid response received: {} ({:?})", _0, _1)]
    InvalidResponse(&'static str, Option<String>),
    ///The response from Twitter was missing an expected value.  The enclosed value was the
    ///expected parameter.
    ///
    ///This usually reflects a bug in this library, as it means I'm expecting a value that may not
    ///always be there, and need to update my parsing to reflect this.
    #[error("Value missing from response: {}", _0)]
    MissingValue(&'static str),
    ///The `Future` being polled has already returned a completed value (or another error). In
    ///order to retry the request, create the `Future` again.
    #[error("Future has already completed")]
    FutureAlreadyCompleted,
    ///The response from Twitter returned an error structure instead of the expected response. The
    ///enclosed value was the response from Twitter.
    #[error("Errors returned by Twitter: {_1}")]
    TwitterError(Headers, TwitterErrors),
    ///The response returned from Twitter contained an error indicating that the rate limit for
    ///that method has been reached. The enclosed value is the Unix timestamp in UTC when the next
    ///rate-limit window will open.
    #[error("Rate limit reached, hold until {}", _0)]
    RateLimit(i32),
    ///An attempt to upload a video or gif successfully uploaded the file, but failed in
    ///post-processing. The enclosed value contains the error message from Twitter.
    #[error("Error processing media: {}", _0)]
    MediaError(#[from] MediaError),
    ///The response from Twitter gave a response code that indicated an error. The enclosed value
    ///was the response code.
    ///
    ///This is only returned if Twitter did not also return an [error code][TwitterErrors] in the
    ///response body. That check is performed before examining the status code.
    ///
    ///[TwitterErrors]: struct.TwitterErrors.html
    #[error("Error status received: {}", _0)]
    BadStatus(hyper::StatusCode),
    ///The web request experienced an error. The enclosed error was returned from hyper.
    #[error("Network error: {}", _0)]
    NetError(#[from] hyper::error::Error),
    ///The `native_tls` implementation returned an error. The enclosed error was returned from
    ///`native_tls`.
    #[cfg(feature = "native_tls")]
    #[error("TLS error: {}", _0)]
    TlsError(#[from] native_tls::Error),
    ///An error was experienced while processing the response stream. The enclosed error was
    ///returned from libstd.
    #[error("IO error: {}", _0)]
    IOError(#[from] std::io::Error),
    ///An error occurred while loading the JSON response. The enclosed error was returned from
    ///`serde_json`.
    #[error("JSON deserialize error: {}", _0)]
    DeserializeError(#[from] serde_json::Error),
    ///An error occurred when parsing a timestamp from Twitter. The enclosed error was returned
    ///from chrono.
    #[error("Error parsing timestamp: {}", _0)]
    TimestampParseError(#[from] chrono::ParseError),
    ///The tokio `Timer` instance was shut down while waiting on a timer, for example while waiting
    ///for media to be processed by Twitter. The enclosed error was returned from `tokio`.
    #[error("Timer runtime shutdown: {}", _0)]
    TimerShutdownError(#[from] tokio::time::Error),
    ///An error occurred when reading the value from a response header. The enclused error was
    ///returned from hyper.
    ///
    ///This error should be considerably rare, but is included to ensure that egg-mode doesn't
    ///panic if it receives malformed headers or the like.
    #[error("Error decoding headers: {}", _0)]
    HeaderParseError(#[from] hyper::header::ToStrError),
    ///An error occurred when converting a rate-limit header to an integer. The enclosed error was
    ///returned from the standard library.
    ///
    ///This error should be considerably rare, but is included to ensure that egg-mode doesn't
    ///panic if it receives malformed headers or the like.
    #[error("Error converting headers: {}", _0)]
    HeaderConvertError(#[from] std::num::ParseIntError),
}
