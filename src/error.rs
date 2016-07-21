use std;
use hyper;

///A set of errors that can occur when interacting with Twitter.
#[derive(Debug)]
pub enum Error {
    ///The response from Twitter was formatted incorrectly or in
    ///an unexpected manner.
    InvalidResponse,
    ///The response from Twitter was missing an expected value.
    ///The enclosed value was the expected parameter.
    MissingValue(&'static str),
    ///The web request experienced an error. The enclosed value
    ///was returned from hyper.
    NetError(hyper::error::Error),
    ///An error was experienced while processing the response
    ///stream. The enclosed value was returned from libstd.
    IOError(std::io::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Error::InvalidResponse => write!(f, "Invalid response received"),
            Error::MissingValue(ref val) => write!(f, "Value missing from response: {}", val),
            Error::NetError(ref err) => write!(f, "Network error: {}", err),
            Error::IOError(ref err) => write!(f, "IO Error: {}", err),
        }
    }
}

impl std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::InvalidResponse => "Invalid response received",
            Error::MissingValue(_) => "Value missing from response",
            Error::NetError(ref err) => err.description(),
            Error::IOError(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&std::error::Error> {
        match *self {
            Error::NetError(ref err) => Some(err),
            Error::IOError(ref err) => Some(err),
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
