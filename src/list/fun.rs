use super::*;
use auth;
use user;

pub fn memberships<'a>(user: &'a user::UserID, token: &'a auth::Token) -> list::ListIter<'a> {
    list::ListIter::new(user, list::ListIterType::Memberships, token)
}

pub fn list<'a>(user: &'a user::UserID, token: &'a auth::Token) -> list::ListIter<'a> {
    list::ListIter::new(user, list::ListIterType::Lists, token)
}

pub fn subscriptions<'a>(user: &'a user::UserID, token: &'a auth::Token) -> list::ListIter<'a> {
    list::ListIter::new(user, list::ListIterType::Subscriptions, token)
}

pub fn ownerships<'a>(user: &'a user::UserID, token: &'a auth::Token) -> list::ListIter<'a> {
    list::ListIter::new(user, list::ListIterType::Ownerships, token)
}

pub fn show<'a>(list: &'a list::ListID<'a>, token: &'a auth::Token) -> WebResponse<list::ListInfo> {
    list.show(token)
}

pub fn members<'a>(list: &'a list::List<'a>) -> CursorIter<'a, cursor::UserCursor> {
    list.members()
}

pub fn is_member<'a>(user: &'a user::UserID, list: &'a list::List<'a>) -> bool {
    list.is_member(user)
}

pub fn statuses<'a>(list: &'a list::List<'a>, since_id: Option<u64>, max_id: Option<u64>) -> WebResponse<Vec<tweet::Tweet>> {
    list.statuses(since_id, max_id)
}
