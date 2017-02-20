use common::*;
use super::*;

use std::collections::HashMap;

use auth;
use user;

///Look up the lists the given user has been added to.
pub fn memberships<'a>(user: &'a user::UserID, token: &'a auth::Token) -> ListIter<'a> {
    ListIter::new(user, ListIterType::Memberships, token)
}

///Look up the lists the given user is subscribed to, including those the user made themselves.
pub fn list<'a>(user: &'a user::UserID, token: &'a auth::Token) -> ListIter<'a> {
    ListIter::new(user, ListIterType::Lists, token)
}

///Look up the lists the given user is subscribed to, but not ones the user made themselves.
pub fn subscriptions<'a>(user: &'a user::UserID, token: &'a auth::Token) -> ListIter<'a> {
    ListIter::new(user, ListIterType::Subscriptions, token)
}

///Look up the lists created by the given user.
pub fn ownerships<'a>(user: &'a user::UserID, token: &'a auth::Token) -> ListIter<'a> {
    ListIter::new(user, ListIterType::Ownerships, token)
}

///Look up information for a single list.
pub fn show<'a>(list: ListID<'a>, token: &'a auth::Token) -> WebResponse<ListInfo> {
    let mut params = HashMap::new();

    add_list_param(&mut params, &list);

    let mut resp = try!(auth::get(links::lists::LISTS_SHOW, token, Some(&params)));

    parse_response(&mut resp)
}

///Look up the users that have been added to the given list.
pub fn members<'a>(list: &'a List<'a>) -> CursorIter<'a, cursor::UserCursor> {
    list.members()
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
pub fn statuses<'a>(list: &'a List<'a>, since_id: Option<u64>, max_id: Option<u64>)
    -> WebResponse<Vec<tweet::Tweet>>
{
    list.statuses(since_id, max_id)
}
