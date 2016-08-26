//! Infrastructure trait and related functions for deserializing data from Twitter.

use rustc_serialize::json;
use error;
use error::Error::{InvalidResponse, MissingValue};

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
        let arr = try!(input.as_array().ok_or(InvalidResponse));

        arr.iter().map(|x| T::from_json(x)).collect()
    }
}

impl FromJson for i64 {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        input.as_i64().ok_or(InvalidResponse)
    }
}

impl FromJson for i32 {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        input.as_i64().map(|x| x as i32).ok_or(InvalidResponse)
    }
}

impl FromJson for String {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        input.as_string().map(|s| s.to_string()).ok_or(InvalidResponse)
    }
}

impl FromJson for bool {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        input.as_boolean().ok_or(InvalidResponse)
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
                                .ok_or(InvalidResponse)
                                .and_then(|v| v.iter()
                                               .map(|i| i.as_i64())
                                               .collect::<Option<Vec<_>>>()
                                               .ok_or(InvalidResponse)));

        if int_vec.len() != 2 {
            return Err(InvalidResponse);
        }

        Ok((int_vec[0] as i32, int_vec[1] as i32))
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

pub fn field<T: FromJson>(input: &json::Json, field: &'static str) -> Result<T, error::Error> {
    T::from_json(try!(input.find(field).ok_or(MissingValue(field))))
}
