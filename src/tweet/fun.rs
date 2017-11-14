// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::collections::HashMap;
use rustc_serialize::json;
use auth;
use cursor;
use user::UserID;
use error::Error::InvalidResponse;
use links;
use common::*;

use super::*;

///Lookup a single tweet by numeric ID.
pub fn show(id: u64, token: &auth::Token, handle: &Handle)
    -> FutureResponse<Tweet>
{
    let mut params = HashMap::new();
    add_param(&mut params, "id", id.to_string());
    add_param(&mut params, "include_my_retweet", "true");
    add_param(&mut params, "tweet_mode", "extended");

    let req = auth::get(links::statuses::SHOW, token, Some(&params));

    make_parsed_future(handle, req)
}

///Lookup the most recent 100 (or fewer) retweets of the given tweet.
///
///Use the `count` parameter to indicate how many retweets you would like to retrieve. If `count`
///is 0 or greater than 100, it will be defaulted to 100 before making the call.
pub fn retweets_of(id: u64, count: u32, token: &auth::Token, handle: &Handle)
    -> FutureResponse<Vec<Tweet>>
{
    let mut params = HashMap::new();
    add_param(&mut params, "tweet_mode", "extended");

    if count == 0 || count > 100 {
        add_param(&mut params, "count", 100.to_string());
    }
    else {
        add_param(&mut params, "count", count.to_string());
    }

    let url = format!("{}/{}.json", links::statuses::RETWEETS_OF_STEM, id);

    let req = auth::get(&url, token, Some(&params));

    make_parsed_future(handle, req)
}

///Lookup the user IDs that have retweeted the given tweet.
///
///Note that while loading the list of retweeters is a cursored search, it does not allow you to
///set the page size. Calling `with_page_size` on the iterator returned by this function will not
///change the page size used by the network call. Setting `page_size` manually may result in an
///error from Twitter.
pub fn retweeters_of<'a>(id: u64, token: &'a auth::Token, handle: &Handle)
    -> cursor::CursorIter<'a, cursor::IDCursor>
{
    let mut params = HashMap::new();
    add_param(&mut params, "id", id.to_string());
    cursor::CursorIter::new(links::statuses::RETWEETERS_OF, token, handle, Some(params), None)
}

///Lookup tweet information for the given list of tweet IDs.
///
///This function differs from `lookup_map` in how it handles protected or nonexistent tweets.
///`lookup` gives a Vec of just the tweets it could load, leaving out any that it couldn't find.
pub fn lookup<I: IntoIterator<Item=u64>>(ids: I, token: &auth::Token, handle: &Handle)
    -> FutureResponse<Vec<Tweet>>
{
    let mut params = HashMap::new();
    let id_param = ids.into_iter().fold(String::new(), |mut acc, x| {
        if !acc.is_empty() { acc.push(','); }
        acc.push_str(&x.to_string());
        acc
    });
    add_param(&mut params, "id", id_param);
    add_param(&mut params, "tweet_mode", "extended");

    let req = auth::post(links::statuses::LOOKUP, token, Some(&params));

    make_parsed_future(handle, req)
}

///Lookup tweet information for the given list of tweet IDs, and return a map indicating which IDs
///couldn't be found.
///
///This function differs from `lookup` in how it handles protected or nonexistent tweets.
///`lookup_map` gives a map containing every ID in the input slice; tweets that don't exist or
///can't be read by the authenticated user store `None` in the map, whereas tweets that could be
///loaded store `Some` and the requested status.
pub fn lookup_map<I: IntoIterator<Item=u64>>(ids: I, token: &auth::Token, handle: &Handle)
    -> FutureResponse<HashMap<u64, Option<Tweet>>>
{
    let mut params = HashMap::new();
    let id_param = ids.into_iter().fold(String::new(), |mut acc, x| {
        if !acc.is_empty() { acc.push(','); }
        acc.push_str(&x.to_string());
        acc
    });
    add_param(&mut params, "id", id_param);
    add_param(&mut params, "map", "true");
    add_param(&mut params, "tweet_mode", "extended");

    let req = auth::post(links::statuses::LOOKUP, token, Some(&params));

    fn parse_map(full_resp: String, headers: &Headers)
        -> Result<Response<HashMap<u64, Option<Tweet>>>, error::Error>
    {
        let parsed: Response<json::Json> = try!(make_response(full_resp, headers));
        let mut map = HashMap::new();

        for (key, val) in try!(parsed.response
                                     .find("id")
                                     .and_then(|v| v.as_object())
                                     .ok_or_else(|| InvalidResponse("unexpected response for lookup_map",
                                                                    Some(parsed.response.to_string())))) {
            let id = try!(key.parse::<u64>().or(Err(InvalidResponse("could not parse id as integer",
                                                                    Some(key.to_string())))));
            if val.is_null() {
                map.insert(id, None);
            }
            else {
                let tweet = try!(Tweet::from_json(&val));
                map.insert(id, Some(tweet));
            }
        }

        Ok(Response::map(parsed, |_| map))
    }

    make_future(handle, req, parse_map)
}

///Make a `Timeline` struct for navigating the collection of tweets posted by the authenticated
///user and the users they follow.
///
///This method has a default page size of 20 tweets, with a maximum of 200.
///
///Twitter will only return the most recent 800 tweets by navigating this method.
pub fn home_timeline<'a>(token: &'a auth::Token, handle: &Handle) -> Timeline<'a> {
    Timeline::new(links::statuses::HOME_TIMELINE, None, token, handle)
}

///Make a `Timeline` struct for navigating the collection of tweets that mention the authenticated
///user's screen name.
///
///This method has a default page size of 20 tweets, with a maximum of 200.
///
///Twitter will only return the most recent 800 tweets by navigating this method.
pub fn mentions_timeline<'a>(token: &'a auth::Token, handle: &Handle) -> Timeline<'a> {
    Timeline::new(links::statuses::MENTIONS_TIMELINE, None, token, handle)
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
                                              token: &'a auth::Token, handle: &Handle)
    -> Timeline<'a>
{
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());
    add_param(&mut params, "exclude_replies", (!with_replies).to_string());
    add_param(&mut params, "include_rts", with_rts.to_string());

    Timeline::new(links::statuses::USER_TIMELINE, Some(params), token, handle)
}

///Make a `Timeline` struct for navigating the collection of tweets posted by the authenticated
///user that have been retweeted by others.
///
///This method has a default page size of 20 tweets, with a maximum of 100.
pub fn retweets_of_me<'a>(token: &'a auth::Token, handle: &Handle) -> Timeline<'a> {
    Timeline::new(links::statuses::RETWEETS_OF_ME, None, token, handle)
}

///Make a `Timeline` struct for navigating the collection of tweets liked by the given user.
///
///This method has a default page size of 20 tweets, with a maximum of 200.
pub fn liked_by<'a, T: Into<UserID<'a>>>(acct: T, token: &'a auth::Token, handle: &Handle)
    -> Timeline<'a>
{
    let mut params = HashMap::new();
    add_name_param(&mut params, &acct.into());
    Timeline::new(links::statuses::LIKES_OF, Some(params), token, handle)
}

///Retweet the given status as the authenticated user.
///
///On success, the future returned by this function yields the retweet, with the original status
///contained in `retweeted_status`.
pub fn retweet(id: u64, token: &auth::Token, handle: &Handle) -> FutureResponse<Tweet> {
    let mut params = HashMap::new();
    add_param(&mut params, "tweet_mode", "extended");

    let url = format!("{}/{}.json", links::statuses::RETWEET_STEM, id);

    let req = auth::post(&url, token, Some(&params));

    make_parsed_future(handle, req)
}

///Unretweet the given status as the authenticated user.
///
///The given ID may either be the original status, or the ID of the authenticated user's retweet of
///it.
///
///On success, the future returned by this function yields the original tweet.
pub fn unretweet(id: u64, token: &auth::Token, handle: &Handle) -> FutureResponse<Tweet> {
    let mut params = HashMap::new();
    add_param(&mut params, "tweet_mode", "extended");

    let url = format!("{}/{}.json", links::statuses::UNRETWEET_STEM, id);

    let req = auth::post(&url, token, Some(&params));

    make_parsed_future(handle, req)
}

///Like the given status as the authenticated user.
///
///On success, the future returned by this function yields the liked tweet.
pub fn like(id: u64, token: &auth::Token, handle: &Handle) -> FutureResponse<Tweet> {
    let mut params = HashMap::new();
    add_param(&mut params, "id", id.to_string());
    add_param(&mut params, "tweet_mode", "extended");

    let req = auth::post(links::statuses::LIKE, token, Some(&params));

    make_parsed_future(handle, req)
}

///Clears a like of the given status as the authenticated user.
///
///On success, the future returned by this function yields the given tweet.
pub fn unlike(id: u64, token: &auth::Token, handle: &Handle) -> FutureResponse<Tweet> {
    let mut params = HashMap::new();
    add_param(&mut params, "id", id.to_string());
    add_param(&mut params, "tweet_mode", "extended");

    let req = auth::post(links::statuses::UNLIKE, token, Some(&params));

    make_parsed_future(handle, req)
}

///Delete the given tweet. The authenticated user must be the user who posted the given tweet.
///
///On success, the future returned by this function yields the given tweet.
pub fn delete(id: u64, token: &auth::Token, handle: &Handle) -> FutureResponse<Tweet> {
    let mut params = HashMap::new();
    add_param(&mut params, "tweet_mode", "extended");

    let url = format!("{}/{}.json", links::statuses::DELETE_STEM, id);

    let req = auth::post(&url, token, Some(&params));

    make_parsed_future(handle, req)
}
