use std::borrow::Cow;
use std::collections::HashMap;

mod response;
mod from_json;

pub use common::response::*;
pub use common::from_json::*;

///Convenience type used to hold parameters to an API call.
pub type ParamList<'a> = HashMap<Cow<'a, str>, Cow<'a, str>>;

///Convenience function to add a key/value parameter to a ParamList.
pub fn add_param<'a, K, V>(list: &mut ParamList<'a>, key: K, value: V) -> Option<Cow<'a, str>>
    where K: Into<Cow<'a, str>>,
          V: Into<Cow<'a, str>>
{
    list.insert(key.into(), value.into())
}

pub fn add_name_param<'a>(list: &mut ParamList<'a>, id: &UserID<'a>) -> Option<Cow<'a, str>> {
    match id {
        &UserID::ID(id) => add_param(list, "user_id", id.to_string()),
        &UserID::ScreenName(name) => add_param(list, "screen_name", name),
    }
}

///Convenience enum to generalize between referring to an account by numeric ID or by screen name.
pub enum UserID<'a> {
    ///Referring via the account's numeric ID.
    ID(i64),
    ///Referring via the account's screen name.
    ScreenName(&'a str),
}

impl<'a> From<i64> for UserID<'a> {
    fn from(id: i64) -> UserID<'a> {
        UserID::ID(id)
    }
}

impl<'a> From<&'a str> for UserID<'a> {
    fn from(name: &'a str) -> UserID<'a> {
        UserID::ScreenName(name)
    }
}

impl<'a> From<&'a String> for UserID<'a> {
    fn from(name: &'a String) -> UserID<'a> {
        UserID::ScreenName(name.as_str())
    }
}
