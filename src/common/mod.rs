// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Set of structs and methods that act as a sort of internal prelude.
//!
//! The elements available in this module and its children are fairly basic building blocks that
//! the other modules all glob-import to make available as a common language. A lot of
//! infrastructure code goes in here.
//!
//! # Module contents
//!
//! Since i split this into multiple files that are then "flattened" into the final module, it's
//! worth giving an inventory of what's in here, since every file has a `use common::*;` in it.
//!
//! ## Type Aliases
//!
//! These types are used commonly enough in the library that they're re-exported here for easy use.
//!
//! * `hyper::headers::HeaderMap<hyper::headers::HeaderValue>` (re-exported as the alias `Headers`)
//!
//! ## `ParamList`
//!
//! `ParamList` is a type alias for use as a collection of parameters to a given web call. It's
//! consumed in the auth module, and provides some easy wrappers to consistently handle some types.
//!
//! `add_param` is a basic function that turns its arguments into `Cow<'static, str>`, then inserts them
//! as a parameter into the given `ParamList`.
//!
//! `add_user_param` provides some special handling for the `UserID` enum, since Twitter always
//! handles user parameters the same way: either as a `"user_id"` parameter with the ID, or as a
//! `"screen_name"` parameter with the screen name. Since that's also how the `UserID` enum is laid
//! out, this just puts the right parameter into the given `ParamList`.
//!
//! `add_list_param` does the same thing, but for `ListID`. Lists get a little more complicated
//! than users, though, since there are technically *three* ways to reference a list: by its ID, by
//! the owner's ID and the list slug, or by the owner's screen name and the list slug. Again, since
//! Twitter always uses the same set of parameters when referencing a list, this deals with all of
//! that work in one place, and i can just take a `ListID` from the user and shove it directly into
//! a `ParamList`.
//!
//! `multiple_names_param` is for when a function takes an `IntoIterator<Item=UserID>` It's
//! possible to mix and match the use of the `"user_id"` and `"screen_name"` parameters on these
//! lookup functions, so this saves up all that handling and splits the iterator into two strings:
//! one for the user IDs, one for the screen names.
//!
//! ## Miscellaneous functions
//!
//! `codepoints_to_bytes` is a convenience function that i use when Twitter returns text ranges in
//! terms of codepoint offsets rather than byte offsets. It takes the pair of numbers from twitter
//! and the string it refers to, and returns a pair that can be used directly to slice the given
//! string. It's also an example of how function parameters are themselves patterns, because i
//! destructure the pair right in the signature. `>_>`
//!
//! `serde_datetime` and `serde_via_string` are helper modules to use with derived
//! `Serialize`/`Deserialize` implementations. `serde_datetime` loads and saves `DateTime`s with
//! the format Twitter uses for timestamps, and `serde_via_string` uses `Display` and `FromStr` to
//! save a string representation of the original type.
//!
//! `merge_by` and its companion type `MergeBy` is a copy of the iterator adapter of the same name
//! from itertools, because i didn't want to add another dependency onto the great towering pile
//! that is my dep tree. `>_>`
//!
//! `max_opt` and `min_opt` are helper functions because i didn't realize that `Option` derived
//! `PartialOrd` and `Ord` at the time. Strictly speaking they're subtly different because
//! `std::cmp::{min,max}` require `Ord` and `min_opt` won't reach for the None if it's there,
//! unlike the derived `PartialOrd` which considers None to be less than Some.
//!
//! ## Authentication functions
//!
//! The functions `get`, `post`, and `post_json` are re-exported here to keep people from having to
//! qualify them from `auth::raw`.
//!
//! ## `Response`
//!
//! Also in its own module, `Response` is a public structure that contains rate-limit information
//! from Twitter, alongside some other desired output. This type is used all over the place in
//! egg-mode, because i wanted to make sure people always had rate-limit information on hand. The
//! module also contains the types and functions that all web calls go through: the ones that load
//! a web call, parse out the rate-limit headers, and call some handler to perform final processing
//! on the result.
//!
//! `request_with_json_response` is the most common future constructor, which just defers to
//! `raw_request` (which just calls `serde_json` and loads up the rate-limit headers)
//! then deserializes the json response to given type.
//!
//! `rate_headers` is an infra function that takes the `Headers` and returns an empty `Response`
//! with the rate-limit info parsed out. It's only exported for a couple functions in `list` which
//! need to get that info even on an error.

use std::borrow::Cow;
use std::collections::HashMap;
use std::future::Future;
use std::iter::Peekable;
use std::pin::Pin;

use hyper::header::{HeaderMap, HeaderValue};
use percent_encoding::{utf8_percent_encode, AsciiSet, PercentEncode};

mod response;

pub use crate::auth::raw::{get, post, post_json};

pub use crate::common::response::*;
use crate::{error, list, user};

/// Macro to create a `Serialize`/`Deserialize` implementation allowing for deserialization via the
/// given "raw" struct or via a "round-trip" using the type's own serialization.
///
/// This macro takes two arguments: the name of a "raw" type, and a public struct definition. The
/// given struct must implement `From` or `TryFrom` for the given raw type. In return, it derives
/// `Serialize` and `Deserialize` for the struct, and creates a handful of helper types to modify
/// the `Deserialize` implementation.
///
/// ## Warning
///
/// If you're adding this to something that should have custom (de-)serialization logic on some
/// fields (e.g.  `DateTime`), make sure to add both the `serialize_with` and `deserialize_with`
/// attributes to the struct definition. All the attributes are copied in to the `SerCopy` struct,
/// so it inherits the deserialization logic that otherwise goes unused. If you don't do this, then
/// the type will fail to "round-trip" properly and may create an error when you try to deserialize
/// from the saved data.
///
/// ## Example
///
/// ```rust,ignore (internal-items)
/// use crate::common::*;
///
/// round_trip! { raw::RawDummyStruct,
///     /// A dummy struct to demonstrate `round_trip!`.
///     pub struct DummyStruct {
///         // ...
///     }
/// }
///
/// impl From<raw::RawDummyStruct> for DummyStruct {
///     fn from(src: RawDummyStruct) -> DummyStruct {
///         // ...
///     }
/// }
/// ```
///
/// ## Implementation
///
/// This macro abuses the `#[serde(untagged)]` enum representation to allow it to deserialize via
/// the existing "raw" type, or the generated "SerCopy" struct which is a field-for-field copy of
/// the original struct. This way, either representation can be used to load the struct without the
/// overhead of loading it all into a `serde_json::Value` first to manually decode into either
/// type.
macro_rules! round_trip {
    ( $raw_name:path,
      $(#[$outer_attr:meta])*
      pub struct $struct_name:ident { $(
          $(#[$attr:meta])*
          $v:vis $f:ident : $t:ty
      ),+ $(,)? } ) => {
        $(#[$outer_attr])*
        #[derive(serde::Serialize)]
        #[derive(serde::Deserialize)]
        #[serde(try_from = "SerEnum")]
        pub struct $struct_name { $(
            $(#[$attr])*
            $v $f: $t
        ),+ }

        impl $struct_name {
            /// Returns the string representation of an error from loading JSON from Twitter, if
            /// applicable.
            ///
            /// Use this function if trying to load something from the API gave you a
            /// deserialization error.
            pub fn twitter_deser_error(input: serde_json::Value) -> Option<String> {
                use crate::common::MapString;

                serde_json::from_value::<$raw_name>(input).err().map_string()
            }

            /// Returns the string representation of an error from loading JSON given by
            /// serializing this type.
            ///
            /// Use this function if trying to load saved JSON from saving previously-loaded data
            /// gave you a deserialization error.
            pub fn roundtrip_deser_error(input: serde_json::Value) -> Option<String> {
                use crate::common::MapString;

                serde_json::from_value::<SerCopy>(input).err().map_string()
            }
        }

        #[derive(serde::Deserialize)]
        struct SerCopy { $(
            $(#[$attr])*
            $v $f: $t
        ),+ }

        impl From<SerCopy> for $struct_name {
            fn from(src: SerCopy) -> $struct_name {
                $struct_name { $(
                    $f: src.$f
                ),+ }
            }
        }

        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum SerEnum {
            Raw($raw_name),
            Ser(SerCopy),
        }

        #[allow(unused_qualifications)]
        impl std::convert::TryFrom<SerEnum> for $struct_name
        where
            $struct_name: std::convert::TryFrom<$raw_name>,
        {
            type Error = <$struct_name as std::convert::TryFrom<$raw_name>>::Error;

            fn try_from(src: SerEnum) -> std::result::Result<$struct_name, Self::Error> {
                use std::convert::TryInto;

                match src {
                    SerEnum::Raw(raw) => raw.try_into(),
                    SerEnum::Ser(ser) => Ok(ser.into()),
                }
            }
        }
    };
}

// n.b. this type alias is re-exported in the `raw` module - these docs are public!
/// A set of headers returned with a response.
pub type Headers = HeaderMap<HeaderValue>;
pub type CowStr = Cow<'static, str>;

// n.b. this type is re-exported in the `raw` module - these docs are public!
/// Represents a list of parameters to a Twitter API call.
///
/// This type is a wrapper around a `HashMap<Cow<'static, str>, Cow<'static, str>>` to collect a
/// set of parameter key/value pairs. These are then used to assemble and sign a Twitter API
/// request. The `Cow` type is used to avoid having to allocate a `String` if a string literal is
/// used for a parameter. All the functions that add parameters to this `ParamList` accept `impl
/// Into<Cow<'static, str>>`, meaning that either a string literal or an owned `String` may be
/// used.
///
/// Most of the functions to add parameters follow a builder pattern, so that you can assemble a
/// `ParamList` in a single statement:
///
/// ```
/// use egg_mode::raw::ParamList;
///
/// // If you were looking up the user `@rustlang` with `GET users/show`, you might assemble a
/// // ParamList like this...
/// let params = ParamList::new()
///     .extended_tweets()
///     .add_user_param("rustlang".into());
/// ```
#[derive(Debug, Clone, Default, derive_more::Deref, derive_more::DerefMut, derive_more::From)]
pub struct ParamList(HashMap<Cow<'static, str>, Cow<'static, str>>);

impl ParamList {
    /// Creates a new, empty `ParamList`.
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    /// Adds the `tweet_mode=extended` parameter to this `ParamList`. Not including this parameter
    /// will cause tweets to be loaded with legacy parameters, and a potentially-truncated `text`
    /// if the tweet is longer than 140 characters. The `Deserialize` impl for `Tweet`s (or
    /// anything that directly or indirectly includes a `Tweet`) expects the extended tweet format
    /// enabled by this function.
    pub fn extended_tweets(self) -> Self {
        self.add_param("tweet_mode", "extended")
    }

    /// Adds the given key/value parameter to this `ParamList`.
    pub fn add_param(
        mut self,
        key: impl Into<Cow<'static, str>>,
        value: impl Into<Cow<'static, str>>,
    ) -> Self {
        self.insert(key.into(), value.into());
        self
    }

    /// Adds the given key/value parameter to this `ParamList` only if the given value is `Some`.
    ///
    /// This can be a convenient wrapper to use in case you may or may not want to include
    /// something based on some condition. If the given value is `None`, then the `ParamList` is
    /// returned unmodified.
    pub fn add_opt_param(
        self,
        key: impl Into<Cow<'static, str>>,
        value: Option<impl Into<Cow<'static, str>>>,
    ) -> Self {
        match value {
            Some(val) => self.add_param(key.into(), val.into()),
            None => self,
        }
    }

    /// Adds the given key/value to this `ParamList` by mutating it in place, rather than consuming
    /// it as in `add_param`.
    pub fn add_param_ref(
        &mut self,
        key: impl Into<Cow<'static, str>>,
        value: impl Into<Cow<'static, str>>,
    ) {
        self.0.insert(key.into(), value.into());
    }

    /// Adds the given `UserID` as a parameter to this `ParamList` by adding either a `user_id` or
    /// `screen_name` parameter as appropriate.
    pub fn add_user_param(self, id: user::UserID) -> Self {
        match id {
            user::UserID::ID(id) => self.add_param("user_id", id.to_string()),
            user::UserID::ScreenName(name) => self.add_param("screen_name", name),
        }
    }

    /// Adds the given `ListID` as a parameter to this `ParamList` by adding either an
    /// `owner_id`/`owner_screen_name` and `slug` pair, or a `list_id`, as appropriate.
    pub fn add_list_param(mut self, list: list::ListID) -> Self {
        match list {
            list::ListID::Slug(owner, name) => {
                match owner {
                    user::UserID::ID(id) => {
                        self.add_param_ref("owner_id", id.to_string());
                    }
                    user::UserID::ScreenName(name) => {
                        self.add_param_ref("owner_screen_name", name);
                    }
                }
                self.add_param("slug", name.clone())
            }
            list::ListID::ID(id) => self.add_param("list_id", id.to_string()),
        }
    }

    /// Merge the parameters from the given `ParamList` into this one.
    pub(crate) fn combine(&mut self, other: ParamList) {
        self.0.extend(other.0);
    }

    /// Renders this `ParamList` as an `application/x-www-form-urlencoded` string.
    ///
    /// The key/value pairs are printed as `key1=value1&key2=value2`, with all keys and values
    /// being percent-encoded according to Twitter's requirements.
    pub fn to_urlencoded(&self) -> String {
        self.0.iter()
            .map(|(k, v)| format!("{}={}", percent_encode(k), percent_encode(v)))
            .collect::<Vec<_>>()
            .join("&")
    }
}

// Helper trait to stringify the contents of an Option
pub(crate) trait MapString {
    fn map_string(&self) -> Option<String>;
}

impl<T: std::fmt::Display> MapString for Option<T> {
    fn map_string(&self) -> Option<String> {
        self.as_ref().map(|v| v.to_string())
    }
}

pub fn multiple_names_param<T, I>(accts: I) -> (String, String)
where
    T: Into<user::UserID>,
    I: IntoIterator<Item = T>,
{
    let mut ids = Vec::new();
    let mut names = Vec::new();

    for x in accts {
        match x.into() {
            user::UserID::ID(id) => ids.push(id.to_string()),
            user::UserID::ScreenName(name) => names.push(name),
        }
    }

    (ids.join(","), names.join(","))
}

///Convenient type alias for futures that resolve to responses from Twitter.
pub(crate) type FutureResponse<T> =
    Pin<Box<dyn Future<Output = error::Result<Response<T>>> + Send>>;

pub fn codepoints_to_bytes(&mut (ref mut start, ref mut end): &mut (usize, usize), text: &str) {
    let mut byte_start = *start;
    let mut byte_end = *end;
    for (ch_offset, (by_offset, _)) in text.char_indices().enumerate() {
        if ch_offset == *start {
            byte_start = by_offset;
        } else if ch_offset == *end {
            byte_end = by_offset;
        }
    }
    *start = byte_start;
    if text.chars().count() == *end {
        *end = text.len()
    } else {
        *end = byte_end
    }
}

///A clone of MergeBy from Itertools.
pub struct MergeBy<Iter, Fun>
where
    Iter: Iterator,
{
    left: Peekable<Iter>,
    right: Peekable<Iter>,
    comp: Fun,
    fused: Option<bool>,
}

impl<Iter, Fun> Iterator for MergeBy<Iter, Fun>
where
    Iter: Iterator,
    Fun: FnMut(&Iter::Item, &Iter::Item) -> bool,
{
    type Item = Iter::Item;

    fn next(&mut self) -> Option<Self::Item> {
        let is_left = match self.fused {
            Some(lt) => lt,
            None => match (self.left.peek(), self.right.peek()) {
                (Some(a), Some(b)) => (self.comp)(a, b),
                (Some(_), None) => {
                    self.fused = Some(true);
                    true
                }
                (None, Some(_)) => {
                    self.fused = Some(false);
                    false
                }
                (None, None) => return None,
            },
        };

        if is_left {
            self.left.next()
        } else {
            self.right.next()
        }
    }
}

pub mod serde_datetime {
    use serde::{Serializer, Deserialize, Deserializer};
    use serde::de::Error;
    use chrono::TimeZone;

    const DATE_FORMAT: &'static str = "%a %b %d %T %z %Y";

    pub fn deserialize<'de, D>(ser: D) -> Result<chrono::DateTime<chrono::Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(ser)?;
        let date = (chrono::Utc)
            .datetime_from_str(&s, DATE_FORMAT)
            .map_err(|e| D::Error::custom(e))?;
        Ok(date)
    }

    pub fn serialize<S>(src: &chrono::DateTime<chrono::Utc>, ser: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        ser.collect_str(&src.format(DATE_FORMAT))
    }
}

pub mod serde_via_string {
    use serde::{Serializer, Deserialize, Deserializer};
    use serde::de::Error;

    use std::fmt;

    pub fn deserialize<'de, D, T>(ser: D) -> Result<T, D::Error>
    where
        D: Deserializer<'de>,
        T: std::str::FromStr,
        <T as std::str::FromStr>::Err: std::fmt::Display,
    {
        let str = String::deserialize(ser)?;
        str.parse().map_err(|e| D::Error::custom(e))
    }

    pub fn serialize<T, S>(src: &T, ser: S) -> Result<S::Ok, S::Error>
    where
        T: fmt::Display,
        S: Serializer,
    {
        ser.collect_str(src)
    }
}

/// Percent-encodes the given string based on the Twitter API specification.
///
/// Twitter bases its encoding scheme on RFC 3986, Section 2.1. They describe the process in full
/// [in their documentation][twitter-percent], but the process can be summarized by saying that
/// every *byte* that is not an ASCII number or letter, or the ASCII characters `-`, `.`, `_`, or
/// `~` must be replaced with a percent sign (`%`) and the byte value in hexadecimal.
///
/// [twitter-percent]: https://developer.twitter.com/en/docs/basics/authentication/oauth-1-0a/percent-encoding-parameters
///
/// When this function was originally implemented, the `percent_encoding` crate did not have an
/// encoding set that matched this, so it was recreated here.
pub fn percent_encode(src: &str) -> PercentEncode {
    lazy_static::lazy_static! {
        static ref ENCODER: AsciiSet = percent_encoding::NON_ALPHANUMERIC.remove(b'-').remove(b'.').remove(b'_').remove(b'~');
    }
    utf8_percent_encode(src, &*ENCODER)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Read;

    pub(crate) fn load_file(path: &str) -> String {
        let mut file = File::open(path).unwrap();
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();
        content
    }

    #[test]
    fn test_codepoints_to_bytes() {
        let unicode = "frônt Iñtërnâtiônàližætiøn ënd";
        // suppose we want to slice out the middle word.
        // 30 codepoints of which we want the middle 20;
        let mut range = (6, 26);
        codepoints_to_bytes(&mut range, unicode);
        assert_eq!(&unicode[range.0..range.1], "Iñtërnâtiônàližætiøn");

        let mut range = (6, 30);
        codepoints_to_bytes(&mut range, unicode);
        assert_eq!(&unicode[range.0..range.1], "Iñtërnâtiônàližætiøn ënd");
    }
}
