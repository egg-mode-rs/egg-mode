use std::collections::HashMap;

use auth;
use links;
use common::*;

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

///Create a `Timeline` struct to traverse the direct messages received by the authenticated user.
pub fn received<'a>(con_token: &'a auth::Token, access_token: &'a auth::Token) -> Timeline<'a> {
    Timeline::new(links::direct::RECEIVED, None, con_token, access_token)
}
