use std;
use hyper;

#[derive(Debug)]
pub enum Error {
    InvalidResponse,
    MissingValue(&'static str),
    NetError(hyper::error::Error),
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
