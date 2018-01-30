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
//! ## Reexports
//!
//! These types are shared by some very common infrastructure, and i caught myself loading them
//! enough times that i just put them in here instead.
//!
//! * `tokio_core::reactor::Handle`
//! * `hyper::Headers`
//!
//! ## `ParamList`
//!
//! `ParamList` is a type alias for use as a collection of parameters to a given web call. It's
//! consumed in the auth module, and provides some easy wrappers to consistently handle some types.
//!
//! `add_param` is a basic function that turns its arguments into `Cow<'a, str>`, then inserts them
//! as a parameter into the given `ParamList`.
//!
//! `add_name_param` provides some special handling for the `UserID` enum, since Twitter always
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
//! ## `WebResponse` and `FutureResponse`
//!
//! These are just convenience type aliases for when i need to return rate-limit information with a
//! call. Most of the methods in this library do that, so the alias is there to make that easier.
//!
//! ## Miscellaneous functions
//!
//! `codepoints_to_bytes` is a convenience function that i use when Twitter returns text ranges in
//! terms of codepoint offsets rather than byte offsets. It takes the pair of numbers from twitter
//! and the string it refers to, and returns a pair that can be used directly to slice the given
//! string. It's also an example of how function parameters are themselves patterns, because i
//! destructure the pair right in the signature. `>_>`
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
//! ## `FromJson`
//!
//! `FromJson` is factored into its own file, but its contents are spilled into this one and
//! re-exported, so it's worth mentioning them here. `FromJson` itself is a lynchpin infrastructure
//! trait that i lean on very heavily to convert the raw JSON responses from Twitter into the final
//! structure that i hand to users. It has a bunch of standard implementations that are documented
//! in that module.
//!
//! `field` is a function that loads up the given field from the given JSON, running it through a
//! desired `FromJson` implementation.
//!
//! `field_present!()` is a macro that i use in `FromJson` implementations when i assume a value
//! needs to be present at all times. It checks whether the given field is either absent or null,
//! and returns `Error::MissingValue` if so. This could *probably* be extended to act like
//! `try!()`, i.e. evaluate the whole macro to `field(input, field)` if it's actually there. I
//! haven't done that yet.
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
//! `ResponseIterRef`, `ResponseIterMut`, and `ResponseIter` are iterator adaptors on
//! `Response<Vec<T>>` that copy out the rate-limit information to all the elements of the
//! contained Vec, individually. There's also a `FromIterator` implementation for
//! `Response<Vec<T>>`, which takes an iterator of `Response<T>` and loads up the last set of
//! rate-limit information for the collection as a whole.
//!
//! `RawFuture` and `TwitterFuture` are the central `Future` types in egg-mode. `RawFuture` is the
//! base-line Future that handles all the steps of a web call, loading up the response into a
//! String to be handled later. `TwitterFuture` wraps `RawFuture` and allows arbitrary handling of
//! the response when it completes.
//!
//! *Most* of the futures in this library can use `TwitterFuture`, but several cannot, because it
//! uses a bare function pointer at its core. As a core design point i didn't want to use `impl
//! Trait`, nor did i want to box any function pointers or trait objects, so i instead chose to
//! create several special-purpose Futures whenever something needed to carry around extra state.
//! Those are contained within the modules that need them - `common::response` only contains
//! `RawFuture` and `TwitterFuture`.
//!
//! `make_raw_future` is only exported for `AuthFuture`, otherwise it's called by the other Future
//! constructors to get the basic load-to-String action.
//!
//! `make_future` is the general form of the `TwitterFuture` constructor, where the processing
//! function pointer is handed in directly.
//!
//! `make_parsed_future` is the most common `TwitterFuture` constructor, which just uses
//! `make_response` (which just calls `FromJson` and loads up the rate-limit headers - it's also
//! exported) as the processor.
//!
//! `rate_headers` is an infra function that takes the `Headers` and returns an empty `Response`
//! with the rate-limit info parsed out. It's only exported for a couple functions in `list` which
//! need to get that info even on an error.

use std::borrow::Cow;
use std::collections::HashMap;
use std::iter::Peekable;
use user;
use list;

pub use tokio_core::reactor::Handle;
pub use hyper::Headers;
use chrono::{self, TimeZone};
use mime;
use serde::{Deserialize, Deserializer};
use serde::de::Error;

// TODO fix up the docs
mod response;

pub use common::response::*;

///Convenience type used to hold parameters to an API call.
pub type ParamList<'a> = HashMap<Cow<'a, str>, Cow<'a, str>>;

///Convenience function to add a key/value parameter to a `ParamList`.
pub fn add_param<'a, K, V>(list: &mut ParamList<'a>, key: K, value: V) -> Option<Cow<'a, str>>
    where K: Into<Cow<'a, str>>,
          V: Into<Cow<'a, str>>
{
    list.insert(key.into(), value.into())
}

pub fn add_name_param<'a>(list: &mut ParamList<'a>, id: &user::UserID<'a>) -> Option<Cow<'a, str>> {
    match *id {
        user::UserID::ID(id) => add_param(list, "user_id", id.to_string()),
        user::UserID::ScreenName(name) => add_param(list, "screen_name", name),
    }
}

pub fn add_list_param<'a>(params: &mut ParamList<'a>, list: &list::ListID<'a>) {
    match *list {
        list::ListID::Slug(ref owner, name) => {
            match *owner {
                user::UserID::ID(id) => {
                    add_param(params, "owner_id", id.to_string());
                },
                user::UserID::ScreenName(name) => {
                    add_param(params, "owner_screen_name", name);
                },
            }
            add_param(params, "slug", name);
        },
        list::ListID::ID(id) => {
            add_param(params, "list_id", id.to_string());
        }
    }
}

pub fn multiple_names_param<'a, T, I>(accts: I) -> (String, String)
    where T: Into<user::UserID<'a>>, I: IntoIterator<Item=T>
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

///Type alias for responses from Twitter.
pub type WebResponse<T> = Result<Response<T>, ::error::Error>;

///Type alias for futures that resolve to responses from Twitter.
///
///See the page for [`TwitterFuture`][] for details on how to use this type. `FutureResponse` is a
///convenience alias that is only there so i don't have to write `Response<T>` all the time.
///
///[`TwitterFuture`]: struct.TwitterFuture.html
pub type FutureResponse<T> = TwitterFuture<Response<T>>;

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
    where Iter: Iterator
{
    left: Peekable<Iter>,
    right: Peekable<Iter>,
    comp: Fun,
    fused: Option<bool>,
}

impl<Iter, Fun> Iterator for MergeBy<Iter, Fun>
    where Iter: Iterator,
          Fun: FnMut(&Iter::Item, &Iter::Item) -> bool
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
                },
                (None, Some(_)) => {
                    self.fused = Some(false);
                    false
                },
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

pub fn merge_by<Iter, Fun>(left: Iter, right: Iter, comp: Fun) -> MergeBy<Iter::IntoIter, Fun>
    where Iter: IntoIterator,
          Fun: FnMut(&Iter::Item, &Iter::Item) -> bool
{
    MergeBy {
        left: left.into_iter().peekable(),
        right: right.into_iter().peekable(),
        comp: comp,
        fused: None,
    }
}

pub fn max_opt<T: PartialOrd>(left: Option<T>, right: Option<T>) -> Option<T> {
    match (left, right) {
        (Some(left), Some(right)) => {
            if left >= right {
                Some(left)
            } else {
                Some(right)
            }
        },
        (left, None) => left,
        (None, right) => right,
    }
}

pub fn min_opt<T: PartialOrd>(left: Option<T>, right: Option<T>) -> Option<T> {
    match (left, right) {
        (Some(left), Some(right)) => {
            if left <= right {
                Some(left)
            } else {
                Some(right)
            }
        },
        (left, None) => left,
        (None, right) => right,
    }
}

pub fn deserialize_datetime<'de, D>(ser: D) -> Result<chrono::DateTime<chrono::Utc>, D::Error> where D: Deserializer<'de> {
    let s = String::deserialize(ser)?;
    let date = (chrono::Utc).datetime_from_str(&s, "%a %b %d %T %z %Y").map_err(|e| D::Error::custom(e))?;
    Ok(date)
}

pub fn deserialize_mime<'de, D>(ser: D) -> Result<mime::Mime, D::Error> where D: Deserializer<'de> {
    let str = String::deserialize(ser)?;
    str.parse().map_err(|e| D::Error::custom(e))
}

#[cfg(test)]
pub (crate) mod tests {
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
