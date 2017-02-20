use common::*;
use super::*;

use auth;
use user;

pub fn memberships<'a>(user: &'a user::UserID, token: &'a auth::Token) -> ListIter<'a> {
    ListIter::new(user, ListIterType::Memberships, token)
}

pub fn list<'a>(user: &'a user::UserID, token: &'a auth::Token) -> ListIter<'a> {
    ListIter::new(user, ListIterType::Lists, token)
}

pub fn subscriptions<'a>(user: &'a user::UserID, token: &'a auth::Token) -> ListIter<'a> {
    ListIter::new(user, ListIterType::Subscriptions, token)
}

pub fn ownerships<'a>(user: &'a user::UserID, token: &'a auth::Token) -> ListIter<'a> {
    ListIter::new(user, ListIterType::Ownerships, token)
}

pub fn show<'a>(list: ListID<'a>, token: &'a auth::Token) -> WebResponse<ListInfo> {
    let mut params = HashMap::new();

    add_list_param(&mut params, &list);

    let mut resp = try!(auth::get(links::lists::LISTS_SHOW, token, Some(&params)));

    parse_response(&mut resp)
}

pub fn members<'a>(list: &'a List<'a>) -> CursorIter<'a, cursor::UserCursor> {
    list.members()
}

pub fn is_member<'a>(user: &'a user::UserID, list: &'a List<'a>) -> bool {
    list.is_member(user)
}

pub fn statuses<'a>(list: &'a List<'a>, since_id: Option<u64>, max_id: Option<u64>) -> WebResponse<Vec<tweet::Tweet>> {
    list.statuses(since_id, max_id)
}
