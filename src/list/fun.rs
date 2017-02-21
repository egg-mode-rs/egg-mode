use common::*;
use super::*;

use std::collections::HashMap;

use auth;
use cursor::{CursorIter, UserCursor, ListCursor};
use links;
use user;

///Look up the lists the given user has been added to.
pub fn memberships<'a>(user: &'a user::UserID, token: &'a auth::Token) -> CursorIter<'a, ListCursor> {
    let mut params = HashMap::new();
    add_name_param(&mut params, user);
    CursorIter::new(links::lists::LISTS_MEMBERSHIPS, token, Some(params), Some(20))
}

///Return up to 100 lists the given user is subscribed to, including those the user made
///themselves.
///
///TODO: this is not strictly `subscriptions` and `ownerships` blended
pub fn list<'a>(user: &'a user::UserID, owned_first: bool, token: &'a auth::Token)
    -> WebResponse<Vec<ListInfo>>
{
    let mut params = HashMap::new();
    add_name_param(&mut params, user);
    add_param(&mut params, "reverse", owned_first.to_string());

    let mut resp = try!(auth::get(links::lists::LISTS_LIST, token, Some(&params)));
    parse_response(&mut resp)
}

///Look up the lists the given user is subscribed to, but not ones the user made themselves.
pub fn subscriptions<'a>(user: &'a user::UserID, token: &'a auth::Token) -> CursorIter<'a, ListCursor> {
    let mut params = HashMap::new();
    add_name_param(&mut params, user);
    CursorIter::new(links::lists::LISTS_SUBSCRIPTIONS, token, Some(params), Some(20))
}

///Look up the lists created by the given user.
pub fn ownerships<'a>(user: &'a user::UserID, token: &'a auth::Token) -> CursorIter<'a, ListCursor> {
    let mut params = HashMap::new();
    add_name_param(&mut params, user);
    CursorIter::new(links::lists::LISTS_OWNERSHIPS, token, Some(params), Some(20))
}

///Look up information for a single list.
pub fn show<'a>(list: ListID<'a>, token: &'a auth::Token) -> WebResponse<ListInfo> {
    let mut params = HashMap::new();

    add_list_param(&mut params, &list);

    let mut resp = try!(auth::get(links::lists::LISTS_SHOW, token, Some(&params)));

    parse_response(&mut resp)
}

///Look up the users that have been added to the given list.
pub fn members<'a>(list: &'a ListID<'a>, token: &'a auth::Token) -> CursorIter<'a, UserCursor> {
    let mut params = HashMap::new();

    add_list_param(&mut params, list);

    CursorIter::new(links::lists::LISTS_MEMBERS, token, Some(params), Some(20))
}

///Check whether the given user has been added to the given list.
pub fn is_member<'a>(user: &'a user::UserID, list: &'a ListID<'a>, token: &auth::Token) -> bool {
    let mut params = HashMap::new();

    add_list_param(&mut params, list);
    add_name_param(&mut params, user);

    //TODO: this needs to properly expose errors/rate limit info/the user data twitter returns
    let mut resp = auth::get(links::lists::LISTS_MEMBERS_SHOW, token, Some(&params)).unwrap();

    let json_resp_result: WebResponse<json::Json> = parse_response(&mut resp);

    if let Ok(j) = json_resp_result {
        if user::TwitterUser::from_json(&j).is_ok() {
            true
        } else {
            false
        }
    } else {
        false
    }
}

///Begin navigating the collection of tweets made by the users added to the given list.
pub fn statuses<'a>(list: &'a ListID<'a>, with_rts: bool, token: &'a auth::Token)
    -> tweet::Timeline<'a>
{
    let mut params = HashMap::new();
    add_list_param(&mut params, list);
    add_param(&mut params, "include_rts", with_rts.to_string());

    tweet::Timeline::new(links::lists::LISTS_STATUSES, Some(params), token)
}
