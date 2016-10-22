//! Infrastructure trait and related functions for deserializing data from Twitter.

use rustc_serialize::json;
use chrono::{self, TimeZone};
use error;
use error::Error::InvalidResponse;
use mime;

///Helper trait to provide a general interface for deserializing Twitter API data structures.
pub trait FromJson : Sized {
    ///Parse the given Json object into a data structure.
    fn from_json(&json::Json) -> Result<Self, error::Error>;

    ///Parse the given string into a Json object, then into a data structure.
    fn from_str(input: &str) -> Result<Self, error::Error> {
        let json = try!(json::Json::from_str(input));

        Self::from_json(&json)
    }
}

impl<T> FromJson for Vec<T> where T: FromJson {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        let arr = try!(input.as_array().ok_or_else(|| InvalidResponse("expected an array", Some(input.to_string()))));

        arr.iter().map(|x| T::from_json(x)).collect()
    }
}

impl<T> FromJson for Option<T> where T: FromJson {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if input.is_null() {
            return Ok(None);
        }

        if let Some(arr) = input.as_array() {
            if arr.is_empty() {
                return Ok(None);
            }
        }

        match T::from_json(input) {
            Ok(val) => Ok(Some(val)),
            Err(err) => Err(err),
        }
    }
}

impl<T> FromJson for Box<T> where T: FromJson {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        Ok(Box::new(try!(T::from_json(input))))
    }
}

impl FromJson for i64 {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        input.as_i64().ok_or_else(|| InvalidResponse("expected an i64", Some(input.to_string())))
    }
}

impl FromJson for i32 {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        input.as_i64().map(|x| x as i32).ok_or_else(|| InvalidResponse("expected an i32", Some(input.to_string())))
    }
}

impl FromJson for f64 {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        input.as_f64().ok_or_else(|| InvalidResponse("expected an f64", Some(input.to_string())))
    }
}

impl FromJson for String {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        input.as_string().map(|s| s.to_string()).ok_or_else(|| InvalidResponse("expected a string", Some(input.to_string())))
    }
}

impl FromJson for bool {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        input.as_boolean().ok_or_else(|| InvalidResponse("expected a boolean", Some(input.to_string())))
    }
}

impl FromJson for (i32, i32) {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        //assumptions: input is
        // - an array
        // - of integers
        // - with exactly two entries
        //any deviation from these assumptions will return an error.
        let int_vec = try!(input.as_array()
                                .ok_or_else(|| InvalidResponse("expected an array for a pair", Some(input.to_string())))
                                .and_then(|v| v.iter()
                                               .map(|i| i.as_i64())
                                               .collect::<Option<Vec<_>>>()
                                               .ok_or_else(|| InvalidResponse("array for pair was not fully integers",
                                                                              Some(input.to_string())))));

        if int_vec.len() != 2 {
            return Err(InvalidResponse("array for pair didn't have two entries", Some(input.to_string())));
        }

        Ok((int_vec[0] as i32, int_vec[1] as i32))
    }
}

impl FromJson for (f64, f64) {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        //assumptions: input is
        // - an array
        // - of floats
        // - with exactly two entries
        //any deviation from these assumptions will return an error.
        let float_vec = try!(input.as_array()
                                  .ok_or_else(|| InvalidResponse("expected an array for a pair", Some(input.to_string())))
                                  .and_then(|v| v.iter()
                                                 .map(|i| i.as_f64())
                                                 .collect::<Option<Vec<_>>>()
                                                 .ok_or_else(|| InvalidResponse("array for pair was not fully floats",
                                                                                Some(input.to_string())))));

        if float_vec.len() != 2 {
            return Err(InvalidResponse("array for pair didn't have two entries", Some(input.to_string())));
        }

        Ok((float_vec[0], float_vec[1]))
    }
}

impl FromJson for json::Json {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        Ok(input.clone())
    }

    fn from_str(input: &str) -> Result<Self, error::Error> {
        Ok(try!(json::Json::from_str(input)))
    }
}

impl FromJson for mime::Mime {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        let str = try!(input.as_string().ok_or_else(|| InvalidResponse("expected string for Mime", Some(input.to_string()))));
        let mime = try!(str.parse().or_else(|_| Err(InvalidResponse("could not parse string as Mime", Some(input.to_string())))));

        Ok(mime)
    }
}

impl FromJson for chrono::DateTime<chrono::UTC> {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        let str = try!(input.as_string().ok_or_else(|| InvalidResponse("expected string for DateTime", Some(input.to_string()))));
        let date = try!((chrono::UTC).datetime_from_str(str, "%a %b %d %T %z %Y"));

        Ok(date)
    }
}

pub fn field<T: FromJson>(input: &json::Json, field: &'static str) -> Result<T, error::Error> {
    T::from_json(input.find(field).unwrap_or(&json::Json::Null))
}
