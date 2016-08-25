//! Structs and functions for working with statuses and timelines.
use std::collections::HashMap;
use auth;
use links;
use common::*;

mod structs;

pub use self::structs::*;

///Lookup a single tweet by numeric ID.
pub fn show(id: i64, con_token: &auth::Token, access_token: &auth::Token)
    -> WebResponse<Tweet>
{
    let mut params = HashMap::new();
    add_param(&mut params, "id", id.to_string());

    let mut resp = try!(auth::get(links::statuses::SHOW, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}

///Lookup the most recent 100 (or fewer) retweets of the given tweet.
///
///Use the `count` parameter to indicate how many retweets you would like to retrieve. If `count`
///is 0 or greater than 100, 100 will be given to Twitter.
pub fn retweets_of(id: i64, count: u32, con_token: &auth::Token, access_token: &auth::Token)
    -> WebResponse<Vec<Tweet>>
{
    let mut params = HashMap::new();

    if count == 0 || count > 100 {
        add_param(&mut params, "count", 100.to_string());
    }
    else {
        add_param(&mut params, "count", count.to_string());
    }

    let url = format!("{}/{}.json", links::statuses::RETWEETS_OF_STEM, id);

    let mut resp = try!(auth::get(&url, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}
