//! Structs and functions for working with statuses and timelines.
use std::collections::HashMap;
use auth;
use links;
use error;
use common::*;

mod structs;

pub use self::structs::*;

///Lookup a single tweet by numeric ID.
pub fn show(id: i64, con_token: &auth::Token, access_token: &auth::Token)
    -> Result<Response<Tweet>, error::Error>
{
    let mut params = HashMap::new();
    add_param(&mut params, "id", id.to_string());

    let mut resp = try!(auth::get(links::statuses::SHOW, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}
