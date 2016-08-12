//! Structs and methods for pulling user information from Twitter.
//!
//! All the functions in this module eventually return either a [TwitterUser][] struct or the
//! numeric ID of one. The TwitterUser struct itself contains many fields, relating to the user's
//! profile information and a handful of UI settings available to them. See the struct's
//! documention for details.
//!
//! [TwitterUser]: struct.TwitterUser.html
//!
//! ## `UserCursor`/`UserLoader` and `IDCursor`/`IDLoader` (and `UserSearch`)
//!
//! The functions that return the \*Loader structs all return paginated results, implemented over
//! the network as the corresponding \*Cursor structs. The Loader structs both implement
//! `Iterator`, returning an individual user or ID at a time. This allows them to easily be used
//! with regular iterator adaptors and looped over:
//!
//! ```rust,no_run
//! # let consumer_token = egg_mode::Token::new("", "");
//! # let access_token = egg_mode::Token::new("", "");
//! for user in egg_mode::user::friends_of("rustlang", &consumer_token, &access_token)
//!                            .with_page_size(5)
//!                            .map(|resp| resp.unwrap().response)
//!                            .take(5) {
//!     println!("{} (@{})", user.name, user.screen_name);
//! }
//! ```
//!
//! The actual Item returned by the iterator is `Result<Response<TwitterUser>, Error>`; rate-limit
//! information and network errors are passed into the loop as-is.

use std::borrow::Borrow;
use std::collections::HashMap;
use common::*;
use error;
use auth;
use links;

mod structs;

pub use user::structs::*;

///Lookup a set of Twitter users by their numerical ID.
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
pub fn lookup_names<S: Borrow<str>>(names: &[S], con_token: &auth::Token, access_token: &auth::Token)
    -> Result<Response<Vec<TwitterUser>>, error::Error>
{
    let mut params = HashMap::new();
    let id_param = names.join(",");
    add_param(&mut params, "screen_name", id_param);

    let mut resp = try!(auth::post(links::users::LOOKUP, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}

///Lookup a set of Twitter users by both ID and screen name, as applicable.
pub fn lookup(accts: &[UserID], con_token: &auth::Token, access_token: &auth::Token)
    -> Result<Response<Vec<TwitterUser>>, error::Error>
{
    let mut params = HashMap::new();
    let id_param = accts.iter()
                        .filter_map(|x| match x {
                            &UserID::ID(id) => Some(id.to_string()),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join(",");
    let name_param = accts.iter()
                          .filter_map(|x| match x {
                              &UserID::ScreenName(name) => Some(name),
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

///Lookup users based on the given search term.
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
    -> CursorIter<'a, UserCursor>
{
    CursorIter::new(links::users::FRIENDS_LIST, con_token, access_token, Some(acct.into()), Some(20))
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
    -> CursorIter<'a, IDCursor>
{
    CursorIter::new(links::users::FRIENDS_IDS, con_token, access_token, Some(acct.into()), Some(500))
}

///Lookup the users that follow a given account.
///
///This function returns an iterator over the `TwitterUser` objects returned by Twitter. This
///method defaults to returning 20 users in a single network call; the maximum is 200.
pub fn followers_of<'a, T: Into<UserID<'a>>>(acct: T, con_token: &'a auth::Token, access_token: &'a auth::Token)
    -> CursorIter<'a, UserCursor>
{
    CursorIter::new(links::users::FOLLOWERS_LIST, con_token, access_token, Some(acct.into()), Some(20))
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
    -> CursorIter<'a, IDCursor>
{
    CursorIter::new(links::users::FOLLOWERS_IDS, con_token, access_token, Some(acct.into()), Some(500))
}

///Lookup the users that have been blocked by the authenticated user.
///
///Note that while loading a user's blocks list is a cursored search, it does not allow you to set
///the page size. Calling `with_page_size` on the iterator returned by this function will not
///change the page size used by the network call. Setting `page_size` manually may result in an
///error from Twitter.
pub fn blocks<'a>(con_token: &'a auth::Token, access_token: &'a auth::Token) -> CursorIter<'a, UserCursor> {
    CursorIter::new(links::users::BLOCKS_LIST, con_token, access_token, None, None)
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
pub fn blocks_ids<'a>(con_token: &'a auth::Token, access_token: &'a auth::Token) -> CursorIter<'a, IDCursor> {
    CursorIter::new(links::users::BLOCKS_IDS, con_token, access_token, None, None)
}

///Lookup the users that have been muted by the authenticated user.
///
///Note that while loading a user's mutes list is a cursored search, it does not allow you to set
///the page size. Calling `with_page_size` on the iterator returned by this function will not
///change the page size used by the network call. Setting `page_size` manually may result in an
///error from Twitter.
pub fn mutes<'a>(con_token: &'a auth::Token, access_token: &'a auth::Token) -> CursorIter<'a, UserCursor> {
    CursorIter::new(links::users::MUTES_LIST, con_token, access_token, None, None)
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
pub fn mutes_ids<'a>(con_token: &'a auth::Token, access_token: &'a auth::Token) -> CursorIter<'a, IDCursor> {
    CursorIter::new(links::users::MUTES_IDS, con_token, access_token, None, None)
}

///Follow the given user with the authenticated account, and set whether device notifications
///should be enabled.
///
///Calling this with an account the user already follows will return success, even though it
///doesn't change any settings.
pub fn follow<'a, T: Into<UserID<'a>>>(acct: T, notifications: bool, con_token: &auth::Token, access_token: &auth::Token)
    -> Result<Response<TwitterUser>, error::Error>
{
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());
    add_param(&mut params, "follow", notifications.to_string());

    let mut resp = try!(auth::post(links::users::FOLLOW, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}

///Unfollow the given user with the authenticated account.
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
