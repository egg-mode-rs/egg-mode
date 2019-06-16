// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::auth;
use crate::common::*;
use crate::cursor;
use crate::links;
use std::borrow::Cow;
use std::collections::HashMap;

use super::*;

//---Groups of users---

/// Look up profile information for several Twitter users.
///
/// This function is set up so it can be called with a few different Item types; whether just IDs
/// with `u64`, just screen names with `&str` or `String`, or even a mix of both (by using `UserID`
/// directly).
///
/// ## Examples
///
/// ```rust,no_run
/// # extern crate egg_mode; extern crate tokio; extern crate futures;
/// # use egg_mode::Token;
/// use tokio::runtime::current_thread::block_on_all;
/// # fn main() {
/// # let token: Token = unimplemented!();
/// let mut list: Vec<u64> = Vec::new();
///
/// list.push(1234);
/// list.push(2345);
///
/// let users = block_on_all(egg_mode::user::lookup(&list, &token)).unwrap();
/// # }
/// ```
///
/// ```rust,no_run
/// # extern crate egg_mode; extern crate tokio; extern crate futures;
/// # use egg_mode::Token;
/// use tokio::runtime::current_thread::block_on_all;
/// # fn main() {
/// # let token: Token = unimplemented!();
/// let mut list: Vec<&str> = Vec::new();
///
/// list.push("rustlang");
/// list.push("ThisWeekInRust");
///
/// let users = block_on_all(egg_mode::user::lookup(&list, &token)).unwrap();
/// # }
/// ```
///
/// ```rust,no_run
/// # extern crate egg_mode; extern crate tokio; extern crate futures;
/// # use egg_mode::Token;
/// use tokio::runtime::current_thread::block_on_all;
/// # fn main() {
/// # let token: Token = unimplemented!();
/// let mut list: Vec<String> = Vec::new();
///
/// list.push("rustlang".to_string());
/// list.push("ThisWeekInRust".to_string());
///
/// let users = block_on_all(egg_mode::user::lookup(&list, &token)).unwrap();
/// # }
/// ```
///
/// ```rust,no_run
/// # extern crate egg_mode; extern crate tokio; extern crate futures;
/// # use egg_mode::Token;
/// use tokio::runtime::current_thread::block_on_all;
/// # fn main() {
/// # let token: Token = unimplemented!();
/// let mut list: Vec<egg_mode::user::UserID> = Vec::new();
///
/// list.push(1234.into());
/// list.push("rustlang".into());
///
/// let users = block_on_all(egg_mode::user::lookup(&list, &token)).unwrap();
/// # }
/// ```
pub fn lookup<'a, T, I>(accts: I, token: &auth::Token) -> FutureResponse<Vec<TwitterUser>>
where
    T: Into<UserID<'a>>,
    I: IntoIterator<Item = T>,
{
    let mut params = HashMap::new();
    let (id_param, name_param) = multiple_names_param(accts);

    add_param(&mut params, "user_id", id_param);
    add_param(&mut params, "screen_name", name_param);

    let req = auth::post(links::users::LOOKUP, token, Some(&params));

    make_parsed_future(req)
}

/// Lookup user information for a single user.
pub fn show<'a, T: Into<UserID<'a>>>(acct: T, token: &auth::Token) -> FutureResponse<TwitterUser> {
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());

    let req = auth::get(links::users::SHOW, token, Some(&params));

    make_parsed_future(req)
}

/// Lookup the user IDs that the authenticating user has disabled retweets from.
///
/// Use `update_follow` to enable/disable viewing retweets from a specific user.
pub fn friends_no_retweets(token: &auth::Token) -> FutureResponse<Vec<u64>> {
    let req = auth::get(links::users::FRIENDS_NO_RETWEETS, token, None);

    make_parsed_future(req)
}

/// Lookup relationship settings between two arbitrary users.
pub fn relation<'a, F, T>(from: F, to: T, token: &auth::Token) -> FutureResponse<Relationship>
where
    F: Into<UserID<'a>>,
    T: Into<UserID<'a>>,
{
    let mut params = HashMap::new();
    match from.into() {
        UserID::ID(id) => add_param(&mut params, "source_id", id.to_string()),
        UserID::ScreenName(name) => add_param(&mut params, "source_screen_name", name),
    };
    match to.into() {
        UserID::ID(id) => add_param(&mut params, "target_id", id.to_string()),
        UserID::ScreenName(name) => add_param(&mut params, "target_screen_name", name),
    };

    let req = auth::get(links::users::FRIENDSHIP_SHOW, token, Some(&params));

    make_parsed_future(req)
}

/// Lookup the relations between the authenticated user and the given accounts.
pub fn relation_lookup<'a, T, I>(
    accts: I,
    token: &auth::Token,
) -> FutureResponse<Vec<RelationLookup>>
where
    T: Into<UserID<'a>>,
    I: IntoIterator<Item = T>,
{
    let mut params = HashMap::new();
    let (id_param, name_param) = multiple_names_param(accts);

    add_param(&mut params, "user_id", id_param);
    add_param(&mut params, "screen_name", name_param);

    let req = auth::get(links::users::FRIENDSHIP_LOOKUP, token, Some(&params));

    make_parsed_future(req)
}

//---Cursored collections---

/// Lookup users based on the given search term.
///
/// This function returns a stream over the `TwitterUser` objects returned by Twitter. Due to a
/// limitation in the API, you can only obtain the first 1000 search results. This method defaults
/// to returning 10 users in a single network call; the maximum is 20. See the [`UserSearch`][]
/// page for details.
///
/// [`UserSearch`]: struct.UserSearch.html
pub fn search<'a, S: Into<Cow<'a, str>>>(query: S, token: &auth::Token) -> UserSearch<'a> {
    UserSearch::new(query, token)
}

/// Lookup the users a given account follows, also called their "friends" within the API.
///
/// This function returns a stream over the `TwitterUser` objects returned by Twitter. This
/// method defaults to returning 20 users in a single network call; the maximum is 200.
pub fn friends_of<'a, T: Into<UserID<'a>>>(
    acct: T,
    token: &auth::Token,
) -> cursor::CursorIter<'a, cursor::UserCursor> {
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());
    cursor::CursorIter::new(links::users::FRIENDS_LIST, token, Some(params), Some(20))
}

/// Lookup the users a given account follows, also called their "friends" within the API, but only
/// return their user IDs.
///
/// This function returns a stream over the User IDs returned by Twitter. This method defaults to
/// returning 500 IDs in a single network call; the maximum is 5000.
///
/// Choosing only to load the user IDs instead of the full user information results in a call that
/// can return more accounts per-page, which can be useful if you anticipate having to page through
/// several results and don't need all the user information.
pub fn friends_ids<'a, T: Into<UserID<'a>>>(
    acct: T,
    token: &auth::Token,
) -> cursor::CursorIter<'a, cursor::IDCursor> {
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());
    cursor::CursorIter::new(links::users::FRIENDS_IDS, token, Some(params), Some(500))
}

/// Lookup the users that follow a given account.
///
/// This function returns a stream over the `TwitterUser` objects returned by Twitter. This
/// method defaults to returning 20 users in a single network call; the maximum is 200.
pub fn followers_of<'a, T: Into<UserID<'a>>>(
    acct: T,
    token: &auth::Token,
) -> cursor::CursorIter<'a, cursor::UserCursor> {
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());
    cursor::CursorIter::new(links::users::FOLLOWERS_LIST, token, Some(params), Some(20))
}

/// Lookup the users that follow a given account, but only return their user IDs.
///
/// This function returns a stream over the User IDs returned by Twitter. This method defaults to
/// returning 500 IDs in a single network call; the maximum is 5000.
///
/// Choosing only to load the user IDs instead of the full user information results in a call that
/// can return more accounts per-page, which can be useful if you anticipate having to page through
/// several results and don't need all the user information.
pub fn followers_ids<'a, T: Into<UserID<'a>>>(
    acct: T,
    token: &auth::Token,
) -> cursor::CursorIter<'a, cursor::IDCursor> {
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());
    cursor::CursorIter::new(links::users::FOLLOWERS_IDS, token, Some(params), Some(500))
}

/// Lookup the users that have been blocked by the authenticated user.
///
/// Note that while loading a user's blocks list is a cursored search, it does not allow you to set
/// the page size. Calling `with_page_size` on a stream returned by this function will not
/// change the page size used by the network call. Setting `page_size` manually may result in an
/// error from Twitter.
pub fn blocks(token: &auth::Token) -> cursor::CursorIter<'static, cursor::UserCursor> {
    cursor::CursorIter::new(links::users::BLOCKS_LIST, token, None, None)
}

/// Lookup the users that have been blocked by the authenticated user, but only return their user
/// IDs.
///
/// Choosing only to load the user IDs instead of the full user information results in a call that
/// can return more accounts per-page, which can be useful if you anticipate having to page through
/// several results and don't need all the user information.
///
/// Note that while loading a user's blocks list is a cursored search, it does not allow you to set
/// the page size. Calling `with_page_size` on a stream returned by this function will not
/// change the page size used by the network call. Setting `page_size` manually may result in an
/// error from Twitter.
pub fn blocks_ids(token: &auth::Token) -> cursor::CursorIter<'static, cursor::IDCursor> {
    cursor::CursorIter::new(links::users::BLOCKS_IDS, token, None, None)
}

/// Lookup the users that have been muted by the authenticated user.
///
/// Note that while loading a user's mutes list is a cursored search, it does not allow you to set
/// the page size. Calling `with_page_size` on a stream returned by this function will not
/// change the page size used by the network call. Setting `page_size` manually may result in an
/// error from Twitter.
pub fn mutes(token: &auth::Token) -> cursor::CursorIter<'static, cursor::UserCursor> {
    cursor::CursorIter::new(links::users::MUTES_LIST, token, None, None)
}

/// Lookup the users that have been muted by the authenticated user, but only return their user IDs.
///
/// Choosing only to load the user IDs instead of the full user information results in a call that
/// can return more accounts per-page, which can be useful if you anticipate having to page through
/// several results and don't need all the user information.
///
/// Note that while loading a user's mutes list is a cursored search, it does not allow you to set
/// the page size. Calling `with_page_size` on a stream returned by this function will not
/// change the page size used by the network call. Setting `page_size` manually may result in an
/// error from Twitter.
pub fn mutes_ids(token: &auth::Token) -> cursor::CursorIter<'static, cursor::IDCursor> {
    cursor::CursorIter::new(links::users::MUTES_IDS, token, None, None)
}

/// Lookup the user IDs who have pending requests to follow the authenticated protected user.
///
/// If the authenticated user is not a protected account, this will return an empty collection.
pub fn incoming_requests(token: &auth::Token) -> cursor::CursorIter<'static, cursor::IDCursor> {
    cursor::CursorIter::new(links::users::FRIENDSHIPS_INCOMING, token, None, None)
}

/// Lookup the user IDs with which the authenticating user has a pending follow request.
pub fn outgoing_requests(token: &auth::Token) -> cursor::CursorIter<'static, cursor::IDCursor> {
    cursor::CursorIter::new(links::users::FRIENDSHIPS_OUTGOING, token, None, None)
}

//---User actions---

/// Follow the given account with the authenticated user, and set whether device notifications
/// should be enabled.
///
/// Upon success, the future returned by this function yields the user that was just followed, even
/// when following a protected account. In the latter case, this indicates that the follow request
/// was successfully sent.
///
/// Calling this with an account the user already follows may return an error, or ("for performance
/// reasons") may return success without changing any account settings.
pub fn follow<'a, T: Into<UserID<'a>>>(
    acct: T,
    notifications: bool,
    token: &auth::Token,
) -> FutureResponse<TwitterUser> {
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());
    add_param(&mut params, "follow", notifications.to_string());

    let req = auth::post(links::users::FOLLOW, token, Some(&params));

    make_parsed_future(req)
}

/// Unfollow the given account with the authenticated user.
///
/// Upon success, the future returned by this function yields the user that was just unfollowed.
///
/// Calling this with an account the user doesn't follow will return success, even though it doesn't
/// change any settings.
pub fn unfollow<'a, T: Into<UserID<'a>>>(
    acct: T,
    token: &auth::Token,
) -> FutureResponse<TwitterUser> {
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());

    let req = auth::post(links::users::UNFOLLOW, token, Some(&params));

    make_parsed_future(req)
}

/// Update notification settings and reweet visibility for the given user.
///
/// Calling this for an account the authenticated user does not already follow will not cause them
/// to follow that user. It will return an error if you pass `Some(true)` for `notifications` or
/// `Some(false)` for `retweets`. Any other combination of arguments will return a `Relationship` as
/// if you had called `relation` between the authenticated user and the given user.
pub fn update_follow<'a, T>(
    acct: T,
    notifications: Option<bool>,
    retweets: Option<bool>,
    token: &auth::Token,
) -> FutureResponse<Relationship>
where
    T: Into<UserID<'a>>,
{
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());
    if let Some(notifications) = notifications {
        add_param(&mut params, "device", notifications.to_string());
    }
    if let Some(retweets) = retweets {
        add_param(&mut params, "retweets", retweets.to_string());
    }

    let req = auth::post(links::users::FRIENDSHIP_UPDATE, token, Some(&params));

    make_parsed_future(req)
}

/// Block the given account with the authenticated user.
///
/// Upon success, the future returned by this function yields the given user.
pub fn block<'a, T: Into<UserID<'a>>>(acct: T, token: &auth::Token) -> FutureResponse<TwitterUser> {
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());

    let req = auth::post(links::users::BLOCK, token, Some(&params));

    make_parsed_future(req)
}

/// Block the given account and report it for spam, with the authenticated user.
///
/// Upon success, the future returned by this function yields the given user.
pub fn report_spam<'a, T: Into<UserID<'a>>>(
    acct: T,
    token: &auth::Token,
) -> FutureResponse<TwitterUser> {
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());

    let req = auth::post(links::users::REPORT_SPAM, token, Some(&params));

    make_parsed_future(req)
}

/// Unblock the given user with the authenticated user.
///
/// Upon success, the future returned by this function yields the given user.
pub fn unblock<'a, T: Into<UserID<'a>>>(
    acct: T,
    token: &auth::Token,
) -> FutureResponse<TwitterUser> {
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());

    let req = auth::post(links::users::UNBLOCK, token, Some(&params));

    make_parsed_future(req)
}

/// Mute the given user with the authenticated user.
///
/// Upon success, the future returned by this function yields the given user.
pub fn mute<'a, T: Into<UserID<'a>>>(acct: T, token: &auth::Token) -> FutureResponse<TwitterUser> {
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());

    let req = auth::post(links::users::MUTE, token, Some(&params));

    make_parsed_future(req)
}

/// Unmute the given user with the authenticated user.
///
/// Upon success, the future returned by this function yields the given user.
pub fn unmute<'a, T: Into<UserID<'a>>>(
    acct: T,
    token: &auth::Token,
) -> FutureResponse<TwitterUser> {
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());

    let req = auth::post(links::users::UNMUTE, token, Some(&params));

    make_parsed_future(req)
}
