use std::collections::HashMap;

use auth;
use links;
use common::*;
use user::UserID;

use super::*;

///Lookup a single DM by its numeric ID.
pub fn show(id: i64, token: &auth::Token)
    -> WebResponse<DirectMessage>
{
    let mut params = HashMap::new();
    add_param(&mut params, "id", id.to_string());

    let mut resp = try!(auth::get(links::direct::SHOW, token, Some(&params)));

    parse_response(&mut resp)
}

///Create a `Timeline` struct to navigate the direct messages received by the authenticated user.
pub fn received<'a>(token: &'a auth::Token) -> Timeline<'a> {
    Timeline::new(links::direct::RECEIVED, None, token)
}

///Create a `Timeline` struct to navigate the direct messages sent by the authenticated user.
pub fn sent<'a>(token: &'a auth::Token) -> Timeline<'a> {
    Timeline::new(links::direct::SENT, None, token)
}

///Send a new direct message to the given user.
///
///The recipient must allow DMs from the authenticated user for this to be successful. In practice,
///this means that the recipient must either follow the authenticated user, or they must have the
///"allow DMs from anyone" setting enabled. As the latter setting has no visibility on the API,
///there may be situations where you can't verify the recipient's ability to receive the requested
///DM beforehand.
///
///Upon successfully sending the DM, the message will be returned.
pub fn send<'a, T: Into<UserID<'a>>>(to: T, text: &str, token: &auth::Token)
    -> WebResponse<DirectMessage>
{
    let mut params = HashMap::new();
    add_name_param(&mut params, &to.into());

    add_param(&mut params, "text", text);

    let mut resp = try!(auth::post(links::direct::SEND, token, Some(&params)));

    parse_response(&mut resp)
}

///Delete the direct message with the given ID.
///
///The authenticated user must be the sender of this DM for this call to be successful.
///
///On a successful deletion, returns the freshly-deleted message.
pub fn delete(id: i64, token: &auth::Token)
    -> WebResponse<DirectMessage>
{
    let mut params = HashMap::new();
    add_param(&mut params, "id", id.to_string());

    let mut resp = try!(auth::post(links::direct::DELETE, token, Some(&params)));

    parse_response(&mut resp)
}
