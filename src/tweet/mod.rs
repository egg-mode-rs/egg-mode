//! Structs and functions for working with statuses and timelines.
use std::collections::HashMap;
use rustc_serialize::json;
use auth;
use error::Error::InvalidResponse;
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
    add_param(&mut params, "include_my_retweet", "true");

    let mut resp = try!(auth::get(links::statuses::SHOW, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}

///Lookup the most recent 100 (or fewer) retweets of the given tweet.
///
///Use the `count` parameter to indicate how many retweets you would like to retrieve. If `count`
///is 0 or greater than 100, it will be defaulted to 100 before making the call.
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

///Lookup tweet information for the given list of tweet IDs.
///
///This function differs from `lookup_map` in how it handles protected or nonexistent tweets.
///`lookup` simply returns a Vec of all the tweets it could find, leaving out any that it couldn't
///find.
pub fn lookup(ids: &[i64], con_token: &auth::Token, access_token: &auth::Token)
    -> WebResponse<Vec<Tweet>>
{
    let mut params = HashMap::new();
    let id_param = ids.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(",");
    add_param(&mut params, "id", id_param);

    let mut resp = try!(auth::post(links::statuses::LOOKUP, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}

///Lookup tweet information for the given list of tweet IDs, and return a map indicating which IDs
///couldn't be found.
///
///This function differs from `lookup` in how it handles protected or nonexistent tweets.
///`lookup_map` returns a map containing every ID in the input slice; tweets that don't exist or
///can't be read by the authenticated user store `None` in the map, whereas tweets that could be
///loaded store `Some` and the requested status.
pub fn lookup_map(ids: &[i64], con_token: &auth::Token, access_token: &auth::Token)
    -> WebResponse<HashMap<i64, Option<Tweet>>>
{
    let mut params = HashMap::new();
    let id_param = ids.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(",");
    add_param(&mut params, "id", id_param);
    add_param(&mut params, "map", "true");

    let mut resp = try!(auth::post(links::statuses::LOOKUP, con_token, access_token, Some(&params)));

    let parsed: Response<json::Json> = try!(parse_response(&mut resp));
    let mut map = HashMap::new();

    for (key, val) in try!(parsed.response
                                 .find("id")
                                 .and_then(|v| v.as_object())
                                 .ok_or(InvalidResponse("unexpected response for lookup_map",
                                                        Some(parsed.response.to_string())))) {
        let id = try!(key.parse::<i64>().or(Err(InvalidResponse("could not parse id as integer",
                                                                Some(key.to_string())))));
        if val.is_null() {
            map.insert(id, None);
        }
        else {
            let tweet = try!(Tweet::from_json(&val));
            map.insert(id, Some(tweet));
        }
    }

    Ok(Response {
        rate_limit: parsed.rate_limit,
        rate_limit_remaining: parsed.rate_limit_remaining,
        rate_limit_reset: parsed.rate_limit_reset,
        response: map,
    })
}

///Make a `Timeline` struct for navigating the collection of tweets posted by the authenticated
///user and the users they follow.
///
///This method has a default page size of 20 tweets, with a maximum of 200.
///
///Twitter will only return the most recent 800 tweets by navigating this method.
pub fn home_timeline<'a>(con_token: &'a auth::Token, access_token: &'a auth::Token) -> Timeline<'a> {
    Timeline::new(links::statuses::HOME_TIMELINE, con_token, access_token)
}

///Make a `Timeline` struct for navigating the collection of tweets that mention the authenticated
///user's screen name.
///
///This method has a default page size of 20 tweets, with a maximum of 200.
///
///Twitter will only return the most recent 800 tweets by navigating this method.
pub fn mentions_timeline<'a>(con_token: &'a auth::Token, access_token: &'a auth::Token) -> Timeline<'a> {
    Timeline::new(links::statuses::MENTIONS_TIMELINE, con_token, access_token)
}
