use std::collections::HashMap;

use auth;
use links;
use common::*;
use user::UserID;

use super::*;

///Lookup a single DM by its numeric ID.
pub fn show(id: i64, con_token: &auth::Token, access_token: &auth::Token)
    -> WebResponse<DirectMessage>
{
    let mut params = HashMap::new();
    add_param(&mut params, "id", id.to_string());

    let mut resp = try!(auth::get(links::direct::SHOW, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}

///Create a `Timeline` struct to navigate the direct messages received by the authenticated user.
pub fn received<'a>(con_token: &'a auth::Token, access_token: &'a auth::Token) -> Timeline<'a> {
    Timeline::new(links::direct::RECEIVED, None, con_token, access_token)
}

///Create a `Timeline` struct to navigate the direct messages sent by the authenticated user.
pub fn sent<'a>(con_token: &'a auth::Token, access_token: &'a auth::Token) -> Timeline<'a> {
    Timeline::new(links::direct::SENT, None, con_token, access_token)
}

///Send a new direct message to the given user.
pub fn send<'a, T: Into<UserID<'a>>>(to: T, text: &str, con_token: &auth::Token, access_token: &auth::Token)
    -> WebResponse<DirectMessage>
{
    let mut params = HashMap::new();
    add_name_param(&mut params, &to.into());

    add_param(&mut params, "text", text);

    let mut resp = try!(auth::post(links::direct::SEND, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}
