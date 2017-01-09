//! Set of structs and methods that act as a sort of internal prelude.
//!
//! The elements available in this module and its children are fairly basic building blocks that
//! the other modules all glob-import to make available as a common language. A lot of
//! infrastructure code goes in here.

use std::borrow::Cow;
use std::collections::HashMap;
use std::iter::Peekable;
use user;

#[macro_use] mod from_json;
mod response;

pub use common::response::*;
pub use common::from_json::*;

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

///Type alias for responses from Twitter.
pub type WebResponse<T> = Result<Response<T>, ::error::Error>;

pub fn codepoints_to_bytes(&mut (ref mut start, ref mut end): &mut (usize, usize), text: &str) {
    for (ch_offset, (by_offset, _)) in text.char_indices().enumerate() {
        if ch_offset == *start {
            *start = by_offset;
        }
        else if ch_offset == *end {
            *end = by_offset;
        }
    }

    if text.chars().count() == *end {
        *end = text.len();
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
        }
        else {
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
            }
            else {
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
            }
            else {
                Some(right)
            }
        },
        (left, None) => left,
        (None, right) => right,
    }
}
