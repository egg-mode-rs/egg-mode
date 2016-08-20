//! A composite error type for errors that can occur while interacting with Twitter.

use std::{self, fmt};
use hyper;
use rustc_serialize;

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

///A set of errors that can occur when interacting with Twitter.
#[derive(Debug)]
pub enum Error {
    ///The response from Twitter was formatted incorrectly or in
    ///an unexpected manner.
    InvalidResponse,
    ///The response from Twitter was missing an expected value.
    ///The enclosed value was the expected parameter.
    MissingValue(&'static str),
    ///The response from Twitter returned an error structure
    ///instead of the expected response. The enclosed value was
    ///the response from Twitter.
    TwitterError(TwitterErrors),
    ///The response from Twitter gave a response code that
    ///indicated an error. The enclosed value was the response
    ///code.
    BadStatus(hyper::status::StatusCode),
    ///The web request experienced an error. The enclosed value
    ///was returned from hyper.
    NetError(hyper::error::Error),
    ///An error was experienced while processing the response
    ///stream. The enclosed value was returned from libstd.
    IOError(std::io::Error),
    ///An error occurred while parsing the JSON resposne. The
    ///enclosed value was returned from rustc_serialize.
    JSONError(rustc_serialize::json::ParserError),
    ///An error occurred while loading the JSON response. The
    ///enclosed value was returned from rustc_serialize.
    DecodeError(rustc_serialize::json::DecoderError),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Error::InvalidResponse => write!(f, "Invalid response received"),
            Error::MissingValue(val) => write!(f, "Value missing from response: {}", val),
            Error::TwitterError(ref err) => write!(f, "Error(s) returned from Twitter: {}", err),
            Error::BadStatus(ref val) => write!(f, "Error status received: {}", val),
            Error::NetError(ref err) => write!(f, "Network error: {}", err),
            Error::IOError(ref err) => write!(f, "IO error: {}", err),
            Error::JSONError(ref err) => write!(f, "JSON parse Error: {}", err),
            Error::DecodeError(ref err) => write!(f, "JSON decode error: {}", err),
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::InvalidResponse => "Invalid response received",
            Error::MissingValue(_) => "Value missing from response",
            Error::TwitterError(_) => "Error returned from Twitter",
            Error::BadStatus(_) => "Response included error code",
            Error::NetError(ref err) => err.description(),
            Error::IOError(ref err) => err.description(),
            Error::JSONError(ref err) => err.description(),
            Error::DecodeError(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&std::error::Error> {
        match *self {
            Error::NetError(ref err) => Some(err),
            Error::IOError(ref err) => Some(err),
            Error::JSONError(ref err) => Some(err),
            Error::DecodeError(ref err) => Some(err),
            _ => None,
        }
    }
}

impl From<hyper::error::Error> for Error {
    fn from(err: hyper::error::Error) -> Error {
        Error::NetError(err)
    }
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Error {
        Error::IOError(err)
    }
}

impl From<rustc_serialize::json::ParserError> for Error {
    fn from(err: rustc_serialize::json::ParserError) -> Error {
        Error::JSONError(err)
    }
}

impl From<rustc_serialize::json::DecoderError> for Error {
    fn from(err: rustc_serialize::json::DecoderError) -> Error {
        Error::DecodeError(err)
    }
}
