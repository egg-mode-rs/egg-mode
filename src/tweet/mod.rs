//! Structs and functions for working with statuses and timelines.
use std::collections::HashMap;
use rustc_serialize::json;
use auth;
use cursor;
use user::UserID;
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

///Lookup the user IDs that have retweeted the given tweet.
///
///Note that while loading a user's mutes list is a cursored search, it does not allow you to set
///the page size. Calling `with_page_size` on the iterator returned by this function will not
///change the page size used by the network call. Setting `page_size` manually may result in an
///error from Twitter.
pub fn retweeters_of<'a>(id: i64, con_token: &'a auth::Token, access_token: &'a auth::Token)
    -> cursor::CursorIter<'a, cursor::IDCursor>
{
    let mut params = HashMap::new();
    add_param(&mut params, "id", id.to_string());
    cursor::CursorIter::new(links::statuses::RETWEETERS_OF, con_token, access_token, Some(params), None)
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
    Timeline::new(links::statuses::HOME_TIMELINE, None, con_token, access_token)
}

///Make a `Timeline` struct for navigating the collection of tweets that mention the authenticated
///user's screen name.
///
///This method has a default page size of 20 tweets, with a maximum of 200.
///
///Twitter will only return the most recent 800 tweets by navigating this method.
pub fn mentions_timeline<'a>(con_token: &'a auth::Token, access_token: &'a auth::Token) -> Timeline<'a> {
    Timeline::new(links::statuses::MENTIONS_TIMELINE, None, con_token, access_token)
}

///Make a `Timeline` struct for navigating the collection of tweets posted by the given user,
///optionally including or excluding replies or retweets.
///
///Attempting to load the timeline of a protected account will only work if the account is the
///authenticated user's, or if the authenticated user is an approved follower of the account.
///
///This method has a default page size of 20 tweets, with a maximum of 200. Note that asking to
///leave out replies or retweets will generate pages that may have fewer tweets than your requested
///page size; Twitter will load the requested number of tweets before removing replies and/or
///retweets.
///
///Twitter will only load the most recent 3,200 tweets with this method.
pub fn user_timeline<'a, T: Into<UserID<'a>>>(acct: T, with_replies: bool, with_rts: bool,
                                              con_token: &'a auth::Token, access_token: &'a auth::Token)
    -> Timeline<'a>
{
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());
    add_param(&mut params, "exclude_replies", (!with_replies).to_string());
    add_param(&mut params, "include_rts", with_rts.to_string());

    Timeline::new(links::statuses::USER_TIMELINE, Some(params), con_token, access_token)
}

///Make a `Timeline` struct for navigating the collection of tweets posted by the authenticated
///user that have been retweeted by others.
///
///This method has a default page size of 20 tweets, with a maximum of 100.
pub fn retweets_of_me<'a>(con_token: &'a auth::Token, access_token: &'a auth::Token) -> Timeline<'a> {
    Timeline::new(links::statuses::RETWEETS_OF_ME, None, con_token, access_token)
}

///Make a `Timeline` struct for navigating the collection of tweets liked by the given user.
///
///This method has a default page size of 20 tweets, with a maximum of 200.
pub fn liked_by<'a, T: Into<UserID<'a>>>(acct: T, con_token: &'a auth::Token, access_token: &'a auth::Token)
    -> Timeline<'a>
{
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());
    Timeline::new(links::statuses::LIKES_OF, Some(params), con_token, access_token)
}

///Retweet the given status as the authenticated user.
///
///On success, returns the retweet, with the original status contained in `retweeted_status`.
pub fn retweet(id: i64, con_token: &auth::Token, access_token: &auth::Token) -> WebResponse<Tweet> {
    let url = format!("{}/{}.json", links::statuses::RETWEET_STEM, id);

    let mut resp = try!(auth::post(&url, con_token, access_token, None));

    parse_response(&mut resp)
}

///Unretweet the given status as the authenticated user.
///
///The given ID may either be the original status, or the ID of the authenticated user's retweet of
///it.
///
///On success, returns the original tweet.
pub fn unretweet(id: i64, con_token: &auth::Token, access_token: &auth::Token) -> WebResponse<Tweet> {
    let url = format!("{}/{}.json", links::statuses::UNRETWEET_STEM, id);

    let mut resp = try!(auth::post(&url, con_token, access_token, None));

    parse_response(&mut resp)
}

///Like the given status as the authenticated user.
///
///On success, returns the liked tweet.
pub fn like(id: i64, con_token: &auth::Token, access_token: &auth::Token) -> WebResponse<Tweet> {
    let mut params = HashMap::new();
    add_param(&mut params, "id", id.to_string());

    let mut resp = try!(auth::post(links::statuses::LIKE, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}

///Clears a like of the given status as the authenticated user.
///
///On success, returns the given tweet.
pub fn unlike(id: i64, con_token: &auth::Token, access_token: &auth::Token) -> WebResponse<Tweet> {
    let mut params = HashMap::new();
    add_param(&mut params, "id", id.to_string());

    let mut resp = try!(auth::post(links::statuses::UNLIKE, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}
