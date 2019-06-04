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

use std::{self, fmt};
use hyper;
use native_tls;
use chrono;
use serde_json;
use tokio;

///Represents a collection of errors returned from a Twitter API call.
///
///This is returned as part of [`Error::TwitterError`][] whenever Twitter has rejected a call.
///
///[`Error::TwitterError`]: enum.Error.html
#[derive(Debug, Deserialize, Serialize)]
pub struct TwitterErrors {
    ///A collection of errors returned by Twitter.
    pub errors: Vec<TwitterErrorCode>,
}

impl fmt::Display for TwitterErrors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut first = true;
        for e in &self.errors {
            if first { first = false; }
            else { writeln!(f, ",")?; }

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
    ///[error-codes]: https://dev.twitter.com/overview/api/response-codes
    pub code: i32,
}

impl fmt::Display for TwitterErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "#{}: {}", self.code, self.message)
    }
}

/// Represents an error that can occur during media processing.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct MediaError {
    /// A numeric error code assigned to the error.
    pub code: i32,
    /// A short name given to the error.
    pub name: String,
    /// The full text of the error message.
    pub message: String,
}

/// A set of errors that can occur when interacting with Twitter.
#[derive(Debug)]
pub enum Error {
    ///A URL was passed to a shortcut function that didn't match the method being called.
    BadUrl,
    ///The response from Twitter was formatted incorrectly or in an unexpected manner. The enclosed
    ///values are an explanatory string and, if applicable, the input that caused the error.
    ///
    ///This usually reflects a bug in this library, as it means I'm not parsing input right.
    InvalidResponse(&'static str, Option<String>),
    ///The response from Twitter was missing an expected value.  The enclosed value was the
    ///expected parameter.
    ///
    ///This usually reflects a bug in this library, as it means I'm expecting a value that may not
    ///always be there, and need to update my parsing to reflect this.
    MissingValue(&'static str),
    ///The `Future` being polled has already returned a completed value (or another error). In
    ///order to retry the request, create the `Future` again.
    FutureAlreadyCompleted,
    ///The response from Twitter returned an error structure instead of the expected response. The
    ///enclosed value was the response from Twitter.
    TwitterError(TwitterErrors),
    ///The response returned from Twitter contained an error indicating that the rate limit for
    ///that method has been reached. The enclosed value is the Unix timestamp in UTC when the next
    ///rate-limit window will open.
    RateLimit(i32),
    ///An attempt to upload a video or gif successfully uploaded the file, but failed in
    ///post-processing. The enclosed value contains the error message from Twitter.
    MediaError(MediaError),
    ///The response from Twitter gave a response code that indicated an error. The enclosed value
    ///was the response code.
    ///
    ///This is only returned if Twitter did not also return an [error code][TwitterErrors] in the
    ///response body. That check is performed before examining the status code.
    ///
    ///[TwitterErrors]: struct.TwitterErrors.html
    BadStatus(hyper::StatusCode),
    ///The web request experienced an error. The enclosed error was returned from hyper.
    NetError(hyper::error::Error),
    ///The `native_tls` implementation returned an error. The enclosed error was returned from
    ///`native_tls`.
    TlsError(native_tls::Error),
    ///An error was experienced while processing the response stream. The enclosed error was
    ///returned from libstd.
    IOError(std::io::Error),
    ///An error occurred while loading the JSON response. The enclosed error was returned from
    ///`serde_json`.
    DeserializeError(serde_json::Error),
    ///An error occurred when parsing a timestamp from Twitter. The enclosed error was returned
    ///from chrono.
    TimestampParseError(chrono::ParseError),
    ///The tokio `Timer` instance was shut down while waiting on a timer, for example while waiting
    ///for media to be processed by Twitter. The enclosed error was returned from `tokio`.
    TimerShutdownError(tokio::timer::Error),
    ///An error occurred when reading the value from a response header. The enclused error was
    ///returned from hyper.
    ///
    ///This error should be considerably rare, but is included to ensure that egg-mode doesn't
    ///panic if it receives malformed headers or the like.
    HeaderParseError(hyper::header::ToStrError),
    ///An error occurred when converting a rate-limit header to an integer. The enclosed error was
    ///returned from the standard library.
    ///
    ///This error should be considerably rare, but is included to ensure that egg-mode doesn't
    ///panic if it receives malformed headers or the like.
    HeaderConvertError(std::num::ParseIntError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Error::BadUrl => write!(f, "URL given did not match API method"),
            Error::InvalidResponse(err, ref ext) => write!(f, "Invalid response received: {} ({:?})", err, ext),
            Error::MissingValue(val) => write!(f, "Value missing from response: {}", val),
            Error::FutureAlreadyCompleted => write!(f, "Future has already been completed"),
            Error::TwitterError(ref err) => write!(f, "Error(s) returned from Twitter: {}", err),
            Error::RateLimit(ts) => write!(f, "Rate limit reached, hold until {}", ts),
            Error::MediaError(ref err) => write!(f, "Error processing media: {}", err.message),
            Error::BadStatus(ref val) => write!(f, "Error status received: {}", val),
            Error::NetError(ref err) => write!(f, "Network error: {}", err),
            Error::TlsError(ref err) => write!(f, "TLS error: {}", err),
            Error::IOError(ref err) => write!(f, "IO error: {}", err),
            Error::DeserializeError(ref err) => write!(f, "JSON deserialize error: {}", err),
            Error::TimestampParseError(ref err) => write!(f, "Error parsing timestamp: {}", err),
            Error::TimerShutdownError(ref err) => write!(f, "Timer runtime shutdown: {}", err),
            Error::HeaderParseError(ref err) => write!(f, "Error decoding header: {}", err),
            Error::HeaderConvertError(ref err) => write!(f, "Error converting header: {}", err),
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::BadUrl => "URL given did not match API method",
            Error::InvalidResponse(_, _) => "Invalid response received",
            Error::MissingValue(_) => "Value missing from response",
            Error::FutureAlreadyCompleted => "Future has already been completed",
            Error::TwitterError(_) => "Error returned from Twitter",
            Error::RateLimit(_) => "Rate limit for method reached",
            Error::MediaError(_) => "Error processing media",
            Error::BadStatus(_) => "Response included error code",
            Error::NetError(ref err) => err.description(),
            Error::TlsError(ref err) => err.description(),
            Error::IOError(ref err) => err.description(),
            Error::DeserializeError(ref err) => err.description(),
            Error::TimestampParseError(ref err) => err.description(),
            Error::TimerShutdownError(ref err) => err.description(),
            Error::HeaderParseError(ref err) => err.description(),
            Error::HeaderConvertError(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&std::error::Error> {
        match *self {
            Error::NetError(ref err) => Some(err),
            Error::TlsError(ref err) => Some(err),
            Error::IOError(ref err) => Some(err),
            Error::TimestampParseError(ref err) => Some(err),
            Error::DeserializeError(ref err) => Some(err),
            Error::TimerShutdownError(ref err) => Some(err),
            Error::HeaderParseError(ref err) => Some(err),
            Error::HeaderConvertError(ref err) => Some(err),
            _ => None,
        }
    }
}

impl From<hyper::error::Error> for Error {
    fn from(err: hyper::error::Error) -> Error {
        Error::NetError(err)
    }
}

impl From<native_tls::Error> for Error {
    fn from(err: native_tls::Error) -> Error {
        Error::TlsError(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::IOError(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Error {
        Error::DeserializeError(err)
    }
}

impl From<chrono::ParseError> for Error {
    fn from(err: chrono::ParseError) -> Error {
        Error::TimestampParseError(err)
    }
}

impl From<tokio::timer::Error> for Error {
    fn from(err: tokio::timer::Error) -> Error {
        Error::TimerShutdownError(err)
    }
}

impl From<hyper::header::ToStrError> for Error {
    fn from(err: hyper::header::ToStrError) -> Error {
        Error::HeaderParseError(err)
    }
}

impl From<std::num::ParseIntError> for Error {
    fn from(err: std::num::ParseIntError) -> Error {
        Error::HeaderConvertError(err)
    }
}
