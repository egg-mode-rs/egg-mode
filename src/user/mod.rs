//! Structs and methods for pulling user information from Twitter.
//!
//! Everything in here acts on users in some way, whether looking up user information, finding the
//! relations between two users, or actions like following or blocking a user.
//!
//! ## Types
//!
//! - `Relationship`/`RelationSource`/`RelationTarget`: returned by `relation`, these types
//!   (`Relationship` contains the other two) show the ways two accounts relate to each other.
//! - `RelationLookup`/`Connection`: returned as part of a collection by `relation_lookup`, these
//!   types (`RelationLookup` contains a `Vec<Connection>`) shows the ways the authenticated user
//!   relates to a specific account.
//! - `TwitterUser`/`UserEntities`/`UserEntityDetail`: returned by many functions in this module,
//!   these types (`TwitterUser` contains the other two) describe the content of a user's profile,
//!   and a handful of settings relating to how their profile is displayed.
//! - `UserSearch`: returned by `search`, this is an iterator over search results.
//!
//! ## Functions
//!
//! ### User actions
//!
//! These functions perform actions to the user's account. Their use requires that your application
//! request write access to authenticated accounts.
//!
//! - `block`/`report_spam`/`unblock`
//! - `follow`/`unfollow`/`update_follow`
//! - `mute`/`unmute`
//!
//! ### Direct lookup
//!
//! These functions return single users, or groups of users without having to iterate over the
//! results.
//!
//! - `show`
//! - `lookup`/`lookup_ids`/`lookup_names`
//! - `friends_no_retweets`
//! - `relation`/`relation_lookup`
//!
//! ### Cursored lookup
//!
//! These functions imply that they can return more entries than Twitter is willing to return at
//! once, so they're delivered in pages. This library takes those paginated results and wraps an
//! iterator around them that loads the pages as-needed.
//!
//! - `search`
//! - `friends_of`/`friends_ids`
//! - `followers_of`/`followers_ids`
//! - `blocks`/`blocks_ids`
//! - `mutes`/`mutes_ids`
//! - `incoming_requests`/`outgoing_requests`

use std::borrow::Borrow;
use std::collections::HashMap;
use common::*;
use error;
use auth;
use links;
use cursor;

mod structs;

pub use user::structs::*;

//---Groups of users---

///Lookup a set of Twitter users by their numerical ID.
#[deprecated(note = "you can call lookup with &[i64] now")]
pub fn lookup_ids(ids: &[i64], con_token: &auth::Token, access_token: &auth::Token)
    -> Result<Response<Vec<TwitterUser>>, error::Error>
{
    let mut params = HashMap::new();
    let id_param = ids.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(",");
    add_param(&mut params, "user_id", id_param);

    let mut resp = try!(auth::post(links::users::LOOKUP, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}

///Lookup a set of Twitter users by their screen name.
#[deprecated(note = "you can call lookup with &[&str] and &[String] now")]
pub fn lookup_names<S: Borrow<str>>(names: &[S], con_token: &auth::Token, access_token: &auth::Token)
    -> Result<Response<Vec<TwitterUser>>, error::Error>
{
    let mut params = HashMap::new();
    let id_param = names.join(",");
    add_param(&mut params, "screen_name", id_param);

    let mut resp = try!(auth::post(links::users::LOOKUP, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}

///Lookup a set of Twitter users by either ID and screen name, as applicable.
///
///This function is set up so it can be called with any number of slice types; whether just IDs,
///just screen names, or even a mix of both (by using `&[UserID]` directly).
///
///## Examples
///
///```rust,no_run
///# let con_token = egg_mode::Token::new("", "");
///# let access_token = egg_mode::Token::new("", "");
///let mut list: Vec<i64> = Vec::new();
///
///list.push(1234);
///list.push(2345);
///
///let users = egg_mode::user::lookup(&list, &con_token, &access_token).unwrap();
///```
///
///```rust,no_run
///# let con_token = egg_mode::Token::new("", "");
///# let access_token = egg_mode::Token::new("", "");
///let mut list: Vec<String> = Vec::new();
///
///list.push("rustlang".into());
///list.push("ThisWeekInRust".into());
///
///let users = egg_mode::user::lookup(&list, &con_token, &access_token).unwrap();
///```
///
///```rust,no_run
///# let con_token = egg_mode::Token::new("", "");
///# let access_token = egg_mode::Token::new("", "");
///let mut list: Vec<egg_mode::user::UserID> = Vec::new();
///
///list.push(1234.into());
///list.push("rustlang".into());
///
///let users = egg_mode::user::lookup(&list, &con_token, &access_token).unwrap();
///```
pub fn lookup<'a, T: 'a>(accts: &'a [T], con_token: &auth::Token, access_token: &auth::Token)
    -> Result<Response<Vec<TwitterUser>>, error::Error>
    where &'a T: Into<UserID<'a>>
{
    let mut params = HashMap::new();
    let id_param = accts.iter()
                        .filter_map(|x| match x.into() {
                            UserID::ID(id) => Some(id.to_string()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join(",");
    let name_param = accts.iter()
                          .filter_map(|x| match x.into() {
                              UserID::ScreenName(name) => Some(name),
                              _ => None,
                          })
                          .collect::<Vec<_>>()
                          .join(",");

    add_param(&mut params, "user_id", id_param);
    add_param(&mut params, "screen_name", name_param);

    let mut resp = try!(auth::post(links::users::LOOKUP, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}

///Lookup user information for a single user.
pub fn show<'a, T: Into<UserID<'a>>>(acct: T, con_token: &auth::Token, access_token: &auth::Token)
    -> Result<Response<TwitterUser>, error::Error>
{
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());

    let mut resp = try!(auth::get(links::users::SHOW, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}

///Lookup the user IDs that the authenticating user has disabled retweets from.
///
///Use `update_follow` to enable/disable viewing retweets from a specific user.
pub fn friends_no_retweets<'a>(con_token: &'a auth::Token, access_token: &'a auth::Token)
    -> Result<Response<Vec<i64>>, error::Error>
{
    let mut resp = try!(auth::get(links::users::FRIENDS_NO_RETWEETS, con_token, access_token, None));

    parse_response(&mut resp)
}

///Lookup relationship settings between two arbitrary users.
pub fn relation<'a, F, T>(from: F, to: T, con_token: &auth::Token, access_token: &auth::Token)
    -> Result<Response<Relationship>, error::Error>
    where F: Into<UserID<'a>>,
          T: Into<UserID<'a>>
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

    let mut resp = try!(auth::get(links::users::FRIENDSHIP_SHOW, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}

///Lookup the relations between the authenticated user and the given accounts.
pub fn relation_lookup(accts: &[UserID], con_token: &auth::Token, access_token: &auth::Token)
    -> Result<Response<Vec<RelationLookup>>, error::Error>
{
    let mut params = HashMap::new();
    let id_param = accts.iter()
                        .filter_map(|x| match *x {
                            UserID::ID(id) => Some(id.to_string()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join(",");
    let name_param = accts.iter()
                          .filter_map(|x| match *x {
                              UserID::ScreenName(name) => Some(name),
                              _ => None,
                          })
                          .collect::<Vec<_>>()
                          .join(",");

    add_param(&mut params, "user_id", id_param);
    add_param(&mut params, "screen_name", name_param);

    let mut resp = try!(auth::get(links::users::FRIENDSHIP_LOOKUP, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}

//---Cursored collections---

///Lookup users based on the given search term.
///
///This function returns an iterator over the `TwitterUser` objects returned by Twitter. Due to a
///limitation in the API, you can only obtain the first 1000 search results. This method defaults
///to returning 10 users in a single network call; the maximum is 20. See the [`UserSearch`][] page
///for details.
///
///[`UserSearch`]: struct.UserSearch.html
pub fn search<'a>(query: &'a str, con_token: &'a auth::Token, access_token: &'a auth::Token)
    -> UserSearch<'a>
{
    UserSearch::new(query, con_token, access_token)
}

///Lookup the users a given account follows, also called their "friends" within the API.
///
///This function returns an iterator over the `TwitterUser` objects returned by Twitter. This
///method defaults to returning 20 users in a single network call; the maximum is 200.
pub fn friends_of<'a, T: Into<UserID<'a>>>(acct: T, con_token: &'a auth::Token, access_token: &'a auth::Token)
    -> cursor::CursorIter<'a, cursor::UserCursor>
{
    cursor::CursorIter::new(links::users::FRIENDS_LIST, con_token, access_token, Some(acct.into()), Some(20))
}

///Lookup the users a given account follows, also called their "friends" within the API, but only
///return their user IDs.
///
///This function returns an iterator over the User IDs returned by Twitter. This method defaults to
///returning 500 IDs in a single network call; the maximum is 5000.
///
///Choosing only to load the user IDs instead of the full user information results in a call that
///can return more accounts per-page, which can be useful if you anticipate having to page through
///several results and don't need all the user information.
pub fn friends_ids<'a, T: Into<UserID<'a>>>(acct: T, con_token: &'a auth::Token, access_token: &'a auth::Token)
    -> cursor::CursorIter<'a, cursor::IDCursor>
{
    cursor::CursorIter::new(links::users::FRIENDS_IDS, con_token, access_token, Some(acct.into()), Some(500))
}

///Lookup the users that follow a given account.
///
///This function returns an iterator over the `TwitterUser` objects returned by Twitter. This
///method defaults to returning 20 users in a single network call; the maximum is 200.
pub fn followers_of<'a, T: Into<UserID<'a>>>(acct: T, con_token: &'a auth::Token, access_token: &'a auth::Token)
    -> cursor::CursorIter<'a, cursor::UserCursor>
{
    cursor::CursorIter::new(links::users::FOLLOWERS_LIST, con_token, access_token, Some(acct.into()), Some(20))
}

///Lookup the users that follow a given account, but only return their user IDs.
///
///This function returns an iterator over the User IDs returned by Twitter. This method defaults to
///returning 500 IDs in a single network call; the maximum is 5000.
///
///Choosing only to load the user IDs instead of the full user information results in a call that
///can return more accounts per-page, which can be useful if you anticipate having to page through
///several results and don't need all the user information.
pub fn followers_ids<'a, T: Into<UserID<'a>>>(acct: T, con_token: &'a auth::Token, access_token: &'a auth::Token)
    -> cursor::CursorIter<'a, cursor::IDCursor>
{
    cursor::CursorIter::new(links::users::FOLLOWERS_IDS, con_token, access_token, Some(acct.into()), Some(500))
}

///Lookup the users that have been blocked by the authenticated user.
///
///Note that while loading a user's blocks list is a cursored search, it does not allow you to set
///the page size. Calling `with_page_size` on the iterator returned by this function will not
///change the page size used by the network call. Setting `page_size` manually may result in an
///error from Twitter.
pub fn blocks<'a>(con_token: &'a auth::Token, access_token: &'a auth::Token) -> cursor::CursorIter<'a, cursor::UserCursor> {
    cursor::CursorIter::new(links::users::BLOCKS_LIST, con_token, access_token, None, None)
}

///Lookup the users that have been blocked by the authenticated user, but only return their user
///IDs.
///
///Choosing only to load the user IDs instead of the full user information results in a call that
///can return more accounts per-page, which can be useful if you anticipate having to page through
///several results and don't need all the user information.
///
///Note that while loading a user's blocks list is a cursored search, it does not allow you to set
///the page size. Calling `with_page_size` on the iterator returned by this function will not
///change the page size used by the network call. Setting `page_size` manually may result in an
///error from Twitter.
pub fn blocks_ids<'a>(con_token: &'a auth::Token, access_token: &'a auth::Token) -> cursor::CursorIter<'a, cursor::IDCursor> {
    cursor::CursorIter::new(links::users::BLOCKS_IDS, con_token, access_token, None, None)
}

///Lookup the users that have been muted by the authenticated user.
///
///Note that while loading a user's mutes list is a cursored search, it does not allow you to set
///the page size. Calling `with_page_size` on the iterator returned by this function will not
///change the page size used by the network call. Setting `page_size` manually may result in an
///error from Twitter.
pub fn mutes<'a>(con_token: &'a auth::Token, access_token: &'a auth::Token) -> cursor::CursorIter<'a, cursor::UserCursor> {
    cursor::CursorIter::new(links::users::MUTES_LIST, con_token, access_token, None, None)
}

///Lookup the users that have been muted by the authenticated user, but only return their user IDs.
///
///Choosing only to load the user IDs instead of the full user information results in a call that
///can return more accounts per-page, which can be useful if you anticipate having to page through
///several results and don't need all the user information.
///
///Note that while loading a user's mutes list is a cursored search, it does not allow you to set
///the page size. Calling `with_page_size` on the iterator returned by this function will not
///change the page size used by the network call. Setting `page_size` manually may result in an
///error from Twitter.
pub fn mutes_ids<'a>(con_token: &'a auth::Token, access_token: &'a auth::Token) -> cursor::CursorIter<'a, cursor::IDCursor> {
    cursor::CursorIter::new(links::users::MUTES_IDS, con_token, access_token, None, None)
}

///Lookup the user IDs who have pending requests to follow the authenticated protected user.
///
///If the authenticated user is not a protected account, this will return an empty collection.
pub fn incoming_requests<'a>(con_token: &'a auth::Token, access_token: &'a auth::Token)
    -> cursor::CursorIter<'a, cursor::IDCursor>
{
    cursor::CursorIter::new(links::users::FRIENDSHIPS_INCOMING, con_token, access_token, None, None)
}

///Lookup the user IDs with which the authenticating user has a pending follow request.
pub fn outgoing_requests<'a>(con_token: &'a auth::Token, access_token: &'a auth::Token)
    -> cursor::CursorIter<'a, cursor::IDCursor>
{
    cursor::CursorIter::new(links::users::FRIENDSHIPS_OUTGOING, con_token, access_token, None, None)
}

//---User actions---

///Follow the given account with the authenticated user, and set whether device notifications
///should be enabled.
///
///Upon success, this function returns `Ok` with the user that was just followed, even when
///following a protected account. In the latter case, this indicates that the follow request was
///successfully sent.
///
///Calling this with an account the user already follows may return an error, or ("for performance
///reasons") may return success without changing any account settings.
pub fn follow<'a, T: Into<UserID<'a>>>(acct: T, notifications: bool, con_token: &auth::Token, access_token: &auth::Token)
    -> Result<Response<TwitterUser>, error::Error>
{
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());
    add_param(&mut params, "follow", notifications.to_string());

    let mut resp = try!(auth::post(links::users::FOLLOW, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}

///Unfollow the given account with the authenticated user.
///
///Upon success, this function returns `Ok` with the user that was just unfollowed.
///
///Calling this with an account the user doesn't follow will return success, even though it doesn't
///change any settings.
pub fn unfollow<'a, T: Into<UserID<'a>>>(acct: T, con_token: &auth::Token, access_token: &auth::Token)
    -> Result<Response<TwitterUser>, error::Error>
{
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());

    let mut resp = try!(auth::post(links::users::UNFOLLOW, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}

///Update notification settings and reweet visibility for the given user.
///
///Calling this for an account the authenticated user does not already follow will not cause them
///to follow that user. It will return an error if you pass `Some(true)` for `notifications` or
///`Some(false)` for `retweets`. Any other combination of arguments will return a `Relationship` as
///if you had called `relation` between the authenticated user and the given user.
pub fn update_follow<'a, T>(acct: T, notifications: Option<bool>, retweets: Option<bool>,
                            con_token: &auth::Token, access_token: &auth::Token)
    -> Result<Response<Relationship>, error::Error>
    where T: Into<UserID<'a>>
{
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());
    if let Some(notifications) = notifications {
        add_param(&mut params, "device", notifications.to_string());
    }
    if let Some(retweets) = retweets {
        add_param(&mut params, "retweets", retweets.to_string());
    }

    let mut resp = try!(auth::post(links::users::FRIENDSHIP_UPDATE, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}

///Block the given account with the authenticated user.
///
///Upon success, this function returns `Ok` with the given user.
pub fn block<'a, T: Into<UserID<'a>>>(acct: T, con_token: &auth::Token, access_token: &auth::Token)
    -> Result<Response<TwitterUser>, error::Error>
{
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());

    let mut resp = try!(auth::post(links::users::BLOCK, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}

///Block the given account and report it for spam, with the authenticated user.
///
///Upon success, this function returns `Ok` with the given user.
pub fn report_spam<'a, T: Into<UserID<'a>>>(acct: T, con_token: &auth::Token, access_token: &auth::Token)
    -> Result<Response<TwitterUser>, error::Error>
{
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());

    let mut resp = try!(auth::post(links::users::REPORT_SPAM, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}

///Unblock the given user with the authenticated user.
///
///Upon success, this function returns `Ok` with the given user.
pub fn unblock<'a, T: Into<UserID<'a>>>(acct: T, con_token: &auth::Token, access_token: &auth::Token)
    -> Result<Response<TwitterUser>, error::Error>
{
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());

    let mut resp = try!(auth::post(links::users::UNBLOCK, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}

///Mute the given user with the authenticated user.
///
///Upon success, this function returns `Ok` with the given user.
pub fn mute<'a, T: Into<UserID<'a>>>(acct: T, con_token: &auth::Token, access_token: &auth::Token)
    -> Result<Response<TwitterUser>, error::Error>
{
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());

    let mut resp = try!(auth::post(links::users::MUTE, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}

///Unmute the given user with the authenticated user.
///
///Upon success, this function returns `Ok` with the given user.
pub fn unmute<'a, T: Into<UserID<'a>>>(acct: T, con_token: &auth::Token, access_token: &auth::Token)
    -> Result<Response<TwitterUser>, error::Error>
{
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());

    let mut resp = try!(auth::post(links::users::UNMUTE, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}
