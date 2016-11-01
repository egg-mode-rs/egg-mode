//! Set of structs and methods that act as a sort of internal prelude.
//!
//! The elements available in this module and its children are fairly basic building blocks that
//! the other modules all glob-import to make available as a common language. A lot of
//! infrastructure code goes in here.

use std::borrow::Cow;
use std::collections::HashMap;
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
