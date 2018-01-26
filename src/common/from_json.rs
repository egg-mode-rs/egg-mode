// // This Source Code Form is subject to the terms of the Mozilla Public
// // License, v. 2.0. If a copy of the MPL was not distributed with this
// // file, You can obtain one at http://mozilla.org/MPL/2.0/.

// //! Infrastructure trait and related functions for deserializing data from Twitter.

// use rustc_serialize::json;
// use chrono::{self, TimeZone};
// use error;
// use error::Error::InvalidResponse;
// use mime;
// use serde::{Deserialize, Deserializer};
// use serde::de::Error;

// ///Helper macro to return MissingValue for null/absent fields that aren't optional.
// macro_rules! field_present {
//     ($input:ident, $field:ident) => {
//         {
//             if $input.find(stringify!($field)).is_none() {
//                 return Err(::error::Error::MissingValue(stringify!($field)));
//             } else if let Some(val) = $input.find(stringify!($field)) {
//                 if val.is_null() {
//                     return Err(::error::Error::MissingValue(stringify!($field)));
//                 }
//             }
//         }
//     };
// }

// ///Helper trait to provide a general interface for deserializing Twitter API data structures.
// ///
// ///Note that this is only here so i can customize the deserialization behavior of things from
// ///Twitter. (And that i didn't think to use the actual `From` trait when i was setting this up, and
// ///now it's one of the biggest backbone pieces of this library's infrastructure. `>_>`) If/when i
// ///replace `rustc-serialize` with `serde`, this entire trait and associated infrastructure may be
// ///discarded if i can replace it with attributes on the Deserialize derive.
// ///
// ///This is the gateway between "receiving a response from Twitter" and "giving a completed
// ///structure to the user". For the most part, this is a fairly rote operation: use the provided
// ///implementations in this module for standard-library types to assemble your final structure, or
// ///defer to an implementation in some contained structure (this is what the `field` function is
// ///for). However, if there's some conversion step i want to perform on top of the data - for
// ///example, convert from codepoint indices to byte indices on a text range - that will also be
// ///performed within the `FromJson` impl.
// pub trait FromJson : Sized {
//     ///Parse the given Json object into a data structure.
//     fn from_json(&json::Json) -> Result<Self, error::Error>;

//     ///Parse the given string into a Json object, then into a data structure.
//     fn from_str(input: &str) -> Result<Self, error::Error> {
//         let json = try!(json::Json::from_str(input));

//         Self::from_json(&json)
//     }
// }

// ///Turn JSON arrays into Vecs.
// impl<T> FromJson for Vec<T> where T: FromJson {
//     fn from_json(input: &json::Json) -> Result<Self, error::Error> {
//         let arr = try!(input.as_array().ok_or_else(|| InvalidResponse("expected an array", Some(input.to_string()))));

//         arr.iter().map(|x| T::from_json(x)).collect()
//     }
// }

// ///Turn a value that can be null into an optional value. Also turns empty arrays into None.
// impl<T> FromJson for Option<T> where T: FromJson {
//     fn from_json(input: &json::Json) -> Result<Self, error::Error> {
//         if input.is_null() {
//             return Ok(None);
//         }

//         if let Some(arr) = input.as_array() {
//             if arr.is_empty() {
//                 return Ok(None);
//             }
//         }

//         match T::from_json(input) {
//             Ok(val) => Ok(Some(val)),
//             Err(err) => Err(err),
//         }
//     }
// }

// ///Box transparently defers to the inner type's impl.
// impl<T> FromJson for Box<T> where T: FromJson {
//     fn from_json(input: &json::Json) -> Result<Self, error::Error> {
//         Ok(Box::new(try!(T::from_json(input))))
//     }
// }

// impl FromJson for usize {
//     fn from_json(input: &json::Json) -> Result<Self, error::Error> {
//         input.as_u64().map(|x| x as usize).ok_or_else(|| InvalidResponse("expected an usize", Some(input.to_string())))
//     }
// }

// impl FromJson for u64 {
//     fn from_json(input: &json::Json) -> Result<Self, error::Error> {
//         input.as_u64().ok_or_else(|| InvalidResponse("expected a u64", Some(input.to_string())))
//     }
// }

// impl FromJson for i64 {
//     fn from_json(input: &json::Json) -> Result<Self, error::Error> {
//         input.as_i64().ok_or_else(|| InvalidResponse("expected an i64", Some(input.to_string())))
//     }
// }

// impl FromJson for i32 {
//     fn from_json(input: &json::Json) -> Result<Self, error::Error> {
//         input.as_i64().map(|x| x as i32).ok_or_else(|| InvalidResponse("expected an i32", Some(input.to_string())))
//     }
// }

// impl FromJson for f64 {
//     fn from_json(input: &json::Json) -> Result<Self, error::Error> {
//         input.as_f64().ok_or_else(|| InvalidResponse("expected an f64", Some(input.to_string())))
//     }
// }

// impl FromJson for String {
//     fn from_json(input: &json::Json) -> Result<Self, error::Error> {
//         input.as_string().map(|s| s.to_string()).ok_or_else(|| InvalidResponse("expected a string", Some(input.to_string())))
//     }
// }

// impl FromJson for bool {
//     fn from_json(input: &json::Json) -> Result<Self, error::Error> {
//         input.as_boolean().ok_or_else(|| InvalidResponse("expected a boolean", Some(input.to_string())))
//     }
// }

// ///Turn arrays of exactly two integers into a pair of integers.
// impl FromJson for (usize, usize) {
//     fn from_json(input: &json::Json) -> Result<Self, error::Error> {
//         //assumptions: input is
//         // - an array
//         // - of integers
//         // - with exactly two entries
//         //any deviation from these assumptions will return an error.
//         let int_vec = try!(Vec::<usize>::from_json(input));

//         if int_vec.len() != 2 {
//             return Err(InvalidResponse("array for pair didn't have two entries", Some(input.to_string())));
//         }

//         Ok((int_vec[0], int_vec[1]))
//     }
// }

// ///Turn arrays of exactly two integers into a pair of integers.
// impl FromJson for (i32, i32) {
//     fn from_json(input: &json::Json) -> Result<Self, error::Error> {
//         //assumptions: input is
//         // - an array
//         // - of integers
//         // - with exactly two entries
//         //any deviation from these assumptions will return an error.
//         let int_vec = try!(Vec::<i32>::from_json(input));

//         if int_vec.len() != 2 {
//             return Err(InvalidResponse("array for pair didn't have two entries", Some(input.to_string())));
//         }

//         Ok((int_vec[0], int_vec[1]))
//     }
// }

// ///Turn arrays of exactly two floats into a pair of floats.
// impl FromJson for (f64, f64) {
//     fn from_json(input: &json::Json) -> Result<Self, error::Error> {
//         //assumptions: input is
//         // - an array
//         // - of floats
//         // - with exactly two entries
//         //any deviation from these assumptions will return an error.
//         let float_vec = try!(Vec::<f64>::from_json(input));

//         if float_vec.len() != 2 {
//             return Err(InvalidResponse("array for pair didn't have two entries", Some(input.to_string())));
//         }

//         Ok((float_vec[0], float_vec[1]))
//     }
// }

// ///For instances where i want to load the raw JSON, here's a pass-through impl. Also overrides
// ///`from_str` to just parse it directly rather than deferring to the `from_json` function, which
// ///would wind up cloning the `Json`.
// impl FromJson for json::Json {
//     fn from_json(input: &json::Json) -> Result<Self, error::Error> {
//         Ok(input.clone())
//     }

//     fn from_str(input: &str) -> Result<Self, error::Error> {
//         Ok(try!(json::Json::from_str(input)))
//     }
// }

// impl FromJson for mime::Mime {
//     fn from_json(input: &json::Json) -> Result<Self, error::Error> {
//         let str = try!(input.as_string().ok_or_else(|| InvalidResponse("expected string for Mime", Some(input.to_string()))));
//         let mime = try!(str.parse().or_else(|_| Err(InvalidResponse("could not parse string as Mime", Some(input.to_string())))));

//         Ok(mime)
//     }
// }

// impl FromJson for chrono::DateTime<chrono::Utc> {
//     fn from_json(input: &json::Json) -> Result<Self, error::Error> {
//         let str = try!(input.as_string().ok_or_else(|| InvalidResponse("expected string for DateTime", Some(input.to_string()))));
//         let date = try!((chrono::Utc).datetime_from_str(str, "%a %b %d %T %z %Y"));

//         Ok(date)
//     }
// }

// ///Load the given field from the given JSON structure as the desired type.
// pub fn field<T: FromJson>(input: &json::Json, field: &'static str) -> Result<T, error::Error> {
//     T::from_json(input.find(field).unwrap_or(&json::Json::Null))
// }
