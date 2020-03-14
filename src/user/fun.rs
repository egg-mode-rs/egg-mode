// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::common::*;
use crate::error::Result;
use crate::{auth, cursor, links};

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
/// # use egg_mode::Token;
/// # #[tokio::main]
/// # async fn main() {
/// # let token: Token = unimplemented!();
/// let mut list: Vec<u64> = Vec::new();
///
/// list.push(1234);
/// list.push(2345);
///
/// let users = egg_mode::user::lookup(list, &token).await.unwrap();
/// # }
/// ```
///
/// ```rust,no_run
/// # use egg_mode::Token;
/// # #[tokio::main]
/// # async fn main() {
/// # let token: Token = unimplemented!();
/// let mut list: Vec<&str> = Vec::new();
///
/// list.push("rustlang");
/// list.push("ThisWeekInRust");
///
/// let users = egg_mode::user::lookup(list, &token).await.unwrap();
/// # }
/// ```
///
/// ```rust,no_run
/// # use egg_mode::Token;
/// # #[tokio::main]
/// # async fn main() {
/// # let token: Token = unimplemented!();
/// let mut list: Vec<String> = Vec::new();
///
/// list.push("rustlang".to_string());
/// list.push("ThisWeekInRust".to_string());
///
/// let users = egg_mode::user::lookup(list, &token).await.unwrap();
/// # }
/// ```
///
/// ```rust,no_run
/// # use egg_mode::Token;
/// # #[tokio::main]
/// # async fn main() {
/// # let token: Token = unimplemented!();
/// let mut list: Vec<egg_mode::user::UserID> = Vec::new();
///
/// list.push(1234.into());
/// list.push("rustlang".into());
///
/// let users = egg_mode::user::lookup(list, &token).await.unwrap();
/// # }
/// ```
pub async fn lookup<T, I>(accts: I, token: &auth::Token) -> Result<Response<Vec<TwitterUser>>>
where
    T: Into<UserID>,
    I: IntoIterator<Item = T>,
{
    let (id_param, name_param) = multiple_names_param(accts);

    let params = ParamList::new()
        .extended_tweets()
        .add_param("user_id", id_param)
        .add_param("screen_name", name_param);

    let req = auth::post(links::users::LOOKUP, token, Some(&params));

    make_parsed_future(req).await
}

/// Lookup user information for a single user.
pub async fn show<T: Into<UserID>>(acct: T, token: &auth::Token) -> Result<Response<TwitterUser>> {
    let params = ParamList::new()
        .extended_tweets()
        .add_name_param(acct.into());

    let req = auth::get(links::users::SHOW, token, Some(&params));

    make_parsed_future(req).await
}

/// Lookup the user IDs that the authenticating user has disabled retweets from.
///
/// Use `update_follow` to enable/disable viewing retweets from a specific user.
pub async fn friends_no_retweets(token: &auth::Token) -> Result<Response<Vec<u64>>> {
    let req = auth::get(links::users::FRIENDS_NO_RETWEETS, token, None);

    make_parsed_future(req).await
}

/// Lookup relationship settings between two arbitrary users.
pub async fn relation<F, T>(from: F, to: T, token: &auth::Token) -> Result<Response<Relationship>>
where
    F: Into<UserID>,
    T: Into<UserID>,
{
    let mut params = match from.into() {
        UserID::ID(id) => ParamList::new().add_param("source_id", id.to_string()),
        UserID::ScreenName(name) => ParamList::new().add_param("source_screen_name", name),
    };
    match to.into() {
        UserID::ID(id) => params.add_param_ref("target_id", id.to_string()),
        UserID::ScreenName(name) => params.add_param_ref("target_screen_name", name),
    };

    let req = auth::get(links::users::FRIENDSHIP_SHOW, token, Some(&params));

    make_parsed_future(req).await
}

/// Lookup the relations between the authenticated user and the given accounts.
pub async fn relation_lookup<T, I>(
    accts: I,
    token: &auth::Token,
) -> Result<Response<Vec<RelationLookup>>>
where
    T: Into<UserID>,
    I: IntoIterator<Item = T>,
{
    let (id_param, name_param) = multiple_names_param(accts);

    let params = ParamList::new()
        .add_param("user_id", id_param)
        .add_param("screen_name", name_param);

    let req = auth::get(links::users::FRIENDSHIP_LOOKUP, token, Some(&params));

    make_parsed_future(req).await
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
pub fn search<S: Into<CowStr>>(query: S, token: &auth::Token) -> UserSearch {
    UserSearch::new(query, token)
}

/// Lookup the users a given account follows, also called their "friends" within the API.
///
/// This function returns a stream over the `TwitterUser` objects returned by Twitter. This
/// method defaults to returning 20 users in a single network call; the maximum is 200.
pub fn friends_of<T: Into<UserID>>(
    acct: T,
    token: &auth::Token,
) -> cursor::CursorIter<cursor::UserCursor> {
    let params = ParamList::new().add_name_param(acct.into());
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
pub fn friends_ids<T: Into<UserID>>(
    acct: T,
    token: &auth::Token,
) -> cursor::CursorIter<cursor::IDCursor> {
    let params = ParamList::new().add_name_param(acct.into());
    cursor::CursorIter::new(links::users::FRIENDS_IDS, token, Some(params), Some(500))
}

/// Lookup the users that follow a given account.
///
/// This function returns a stream over the `TwitterUser` objects returned by Twitter. This
/// method defaults to returning 20 users in a single network call; the maximum is 200.
pub fn followers_of<T: Into<UserID>>(
    acct: T,
    token: &auth::Token,
) -> cursor::CursorIter<cursor::UserCursor> {
    let params = ParamList::new()
        .extended_tweets()
        .add_name_param(acct.into());
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
pub fn followers_ids<T: Into<UserID>>(
    acct: T,
    token: &auth::Token,
) -> cursor::CursorIter<cursor::IDCursor> {
    let params = ParamList::new().add_name_param(acct.into());
    cursor::CursorIter::new(links::users::FOLLOWERS_IDS, token, Some(params), Some(500))
}

/// Lookup the users that have been blocked by the authenticated user.
///
/// Note that while loading a user's blocks list is a cursored search, it does not allow you to set
/// the page size. Calling `with_page_size` on a stream returned by this function will not
/// change the page size used by the network call. Setting `page_size` manually may result in an
/// error from Twitter.
pub fn blocks(token: &auth::Token) -> cursor::CursorIter<cursor::UserCursor> {
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
pub fn blocks_ids(token: &auth::Token) -> cursor::CursorIter<cursor::IDCursor> {
    cursor::CursorIter::new(links::users::BLOCKS_IDS, token, None, None)
}

/// Lookup the users that have been muted by the authenticated user.
///
/// Note that while loading a user's mutes list is a cursored search, it does not allow you to set
/// the page size. Calling `with_page_size` on a stream returned by this function will not
/// change the page size used by the network call. Setting `page_size` manually may result in an
/// error from Twitter.
pub fn mutes(token: &auth::Token) -> cursor::CursorIter<cursor::UserCursor> {
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
pub fn mutes_ids(token: &auth::Token) -> cursor::CursorIter<cursor::IDCursor> {
    cursor::CursorIter::new(links::users::MUTES_IDS, token, None, None)
}

/// Lookup the user IDs who have pending requests to follow the authenticated protected user.
///
/// If the authenticated user is not a protected account, this will return an empty collection.
pub fn incoming_requests(token: &auth::Token) -> cursor::CursorIter<cursor::IDCursor> {
    cursor::CursorIter::new(links::users::FRIENDSHIPS_INCOMING, token, None, None)
}

/// Lookup the user IDs with which the authenticating user has a pending follow request.
pub fn outgoing_requests(token: &auth::Token) -> cursor::CursorIter<cursor::IDCursor> {
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
pub async fn follow<T: Into<UserID>>(
    acct: T,
    notifications: bool,
    token: &auth::Token,
) -> Result<Response<TwitterUser>> {
    let params = ParamList::new()
        .extended_tweets()
        .add_name_param(acct.into())
        .add_param("follow", notifications.to_string());
    let req = auth::post(links::users::FOLLOW, token, Some(&params));
    make_parsed_future(req).await
}

/// Unfollow the given account with the authenticated user.
///
/// Upon success, the future returned by this function yields the user that was just unfollowed.
///
/// Calling this with an account the user doesn't follow will return success, even though it doesn't
/// change any settings.
pub async fn unfollow<T: Into<UserID>>(
    acct: T,
    token: &auth::Token,
) -> Result<Response<TwitterUser>> {
    let params = ParamList::new()
        .extended_tweets()
        .add_name_param(acct.into());
    let req = auth::post(links::users::UNFOLLOW, token, Some(&params));
    make_parsed_future(req).await
}

/// Update notification settings and reweet visibility for the given user.
///
/// Calling this for an account the authenticated user does not already follow will not cause them
/// to follow that user. It will return an error if you pass `Some(true)` for `notifications` or
/// `Some(false)` for `retweets`. Any other combination of arguments will return a `Relationship` as
/// if you had called `relation` between the authenticated user and the given user.
pub async fn update_follow<T>(
    acct: T,
    notifications: Option<bool>,
    retweets: Option<bool>,
    token: &auth::Token,
) -> Result<Response<Relationship>>
where
    T: Into<UserID>,
{
    let params = ParamList::new()
        .add_name_param(acct.into())
        .add_opt_param("device", notifications.map(|v| v.to_string()))
        .add_opt_param("retweets", retweets.map(|v| v.to_string()));
    let req = auth::post(links::users::FRIENDSHIP_UPDATE, token, Some(&params));
    make_parsed_future(req).await
}

/// Block the given account with the authenticated user.
///
/// Upon success, the future returned by this function yields the given user.
pub async fn block<T: Into<UserID>>(acct: T, token: &auth::Token) -> Result<Response<TwitterUser>> {
    let params = ParamList::new()
        .extended_tweets()
        .add_name_param(acct.into());
    let req = auth::post(links::users::BLOCK, token, Some(&params));
    make_parsed_future(req).await
}

/// Block the given account and report it for spam, with the authenticated user.
///
/// Upon success, the future returned by this function yields the given user.
pub async fn report_spam<T: Into<UserID>>(
    acct: T,
    token: &auth::Token,
) -> Result<Response<TwitterUser>> {
    let params = ParamList::new()
        .extended_tweets()
        .add_name_param(acct.into());
    let req = auth::post(links::users::REPORT_SPAM, token, Some(&params));
    make_parsed_future(req).await
}

/// Unblock the given user with the authenticated user.
///
/// Upon success, the future returned by this function yields the given user.
pub async fn unblock<T: Into<UserID>>(
    acct: T,
    token: &auth::Token,
) -> Result<Response<TwitterUser>> {
    let params = ParamList::new()
        .extended_tweets()
        .add_name_param(acct.into());
    let req = auth::post(links::users::UNBLOCK, token, Some(&params));
    make_parsed_future(req).await
}

/// Mute the given user with the authenticated user.
///
/// Upon success, the future returned by this function yields the given user.
pub async fn mute<T: Into<UserID>>(acct: T, token: &auth::Token) -> Result<Response<TwitterUser>> {
    let params = ParamList::new()
        .extended_tweets()
        .add_name_param(acct.into());
    let req = auth::post(links::users::MUTE, token, Some(&params));
    make_parsed_future(req).await
}

/// Unmute the given user with the authenticated user.
///
/// Upon success, the future returned by this function yields the given user.
pub async fn unmute<T: Into<UserID>>(
    acct: T,
    token: &auth::Token,
) -> Result<Response<TwitterUser>> {
    let params = ParamList::new()
        .extended_tweets()
        .add_name_param(acct.into());
    let req = auth::post(links::users::UNMUTE, token, Some(&params));
    make_parsed_future(req).await
}
