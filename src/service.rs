//! Methods to inquire about the Twitter service itself.

use std::str::FromStr;
use std::collections::HashMap;

use rustc_serialize::json;

use auth;
use entities;
use error;
use error::Error::{InvalidResponse, MissingValue};
use links;
use common::*;

///Returns the current Twitter Terms of Service as plain text.
pub fn terms(con_token: &auth::Token, access_token: &auth::Token) -> WebResponse<String> {
    let mut resp = try!(auth::get(links::service::TERMS, con_token, access_token, None));

    let ret = try!(parse_response::<json::Json>(&mut resp));

    Ok(Response {
        response: try!(field(&ret.response, "tos")),
        rate_limit: ret.rate_limit,
        rate_limit_remaining: ret.rate_limit_remaining,
        rate_limit_reset: ret.rate_limit_reset,
    })
}

///Returns the current Twitter Privacy Policy as plain text.
pub fn privacy(con_token: &auth::Token, access_token: &auth::Token) -> WebResponse<String> {
    let mut resp = try!(auth::get(links::service::PRIVACY, con_token, access_token, None));

    let ret = try!(parse_response::<json::Json>(&mut resp));

    Ok(Response {
        response: try!(field(&ret.response, "privacy")),
        rate_limit: ret.rate_limit,
        rate_limit_remaining: ret.rate_limit_remaining,
        rate_limit_reset: ret.rate_limit_reset,
    })
}

///Return the current configuration from Twitter, including the maximum length of a t.co URL and
///maximum photo resolutions per size, among others.
///
///From Twitter: "It is recommended applications request this endpoint when they are loaded, but no
///more than once a day."
pub fn config(con_token: &auth::Token, access_token: &auth::Token) -> WebResponse<Configuration> {
    let mut resp = try!(auth::get(links::service::CONFIG, con_token, access_token, None));

    parse_response(&mut resp)
}

///Return the current rate-limit status for all available methods from the authenticated user.
pub fn rate_limit_status(con_token: &auth::Token, access_token: &auth::Token) -> WebResponse<RateLimitStatus> {
    let mut resp = try!(auth::get(links::service::RATE_LIMIT_STATUS, con_token, access_token, None));

    parse_response(&mut resp)
}

///Represents a service configuration from Twitter.
#[derive(Debug)]
pub struct Configuration {
    ///The character limit in direct messages.
    pub dm_text_character_limit: i32,
    ///The maximum photo sizes for received media. If an uploaded photo is above the dimensions for
    ///a given size category, it will be scaled to that size according to the `resize` property on
    ///each entry.
    pub photo_sizes: entities::MediaSizes,
    ///The maximum length for a t.co URL when given a URL with protocol `http`.
    pub short_url_length: i32,
    ///The maximum length for a t.co URL when given a URL with protocol `https`.
    pub short_url_length_https: i32,
    ///A list of URL slugs that are not valid usernames when in a URL like
    ///`https://twitter.com/[slug]`.
    pub non_username_paths: Vec<String>,
}

impl FromJson for Configuration {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse("Configuration received json that wasn't an object",
                                       Some(input.to_string())));
        }

        Ok(Configuration {
            dm_text_character_limit: try!(field(input, "dm_text_character_limit")),
            photo_sizes: try!(field(input, "photo_sizes")),
            short_url_length: try!(field(input, "short_url_length")),
            short_url_length_https: try!(field(input, "short_url_length_https")),
            non_username_paths: try!(field(input, "non_username_paths")),
        })
    }
}

///Represents the current rate-limit status of many Twitter API calls.
#[derive(Debug)]
pub struct RateLimitStatus {
    ///The rate-limit status for methods in the `direct` module.
    pub direct: HashMap<DirectMethod, Response<()>>,
    ///The rate-limit status for methods in the `place` module.
    pub place: HashMap<PlaceMethod, Response<()>>,
    ///The rate-limit status for methods in the `search` module.
    pub search: HashMap<SearchMethod, Response<()>>,
    ///The rate-limit status for methods in the `service` module.
    pub service: HashMap<ServiceMethod, Response<()>>,
    ///The rate-limit status for methods in the `tweet` module.
    pub tweet: HashMap<TweetMethod, Response<()>>,
    ///The rate-limit status for methods in the `user` module.
    pub user: HashMap<UserMethod, Response<()>>,
}

impl FromJson for RateLimitStatus {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse("RateLimitStatus received json that wasn't an object",
                                       Some(input.to_string())));
        }

        let mut direct = HashMap::new();
        let mut place = HashMap::new();
        let mut search = HashMap::new();
        let mut service = HashMap::new();
        let mut tweet = HashMap::new();
        let mut user = HashMap::new();

        let map = try!(input.find("resources").ok_or(MissingValue("resources")));

        if let Some(map) = map.as_object() {
            for (k, v) in map.values().filter_map(|v| v.as_object()).flat_map(|v| v.iter()) {
                if let Ok(method) = k.parse::<Method>() {
                    match method {
                        Method::Direct(m) => direct.insert(m, try!(FromJson::from_json(v))),
                        Method::Place(p) => place.insert(p, try!(FromJson::from_json(v))),
                        Method::Search(s) => search.insert(s, try!(FromJson::from_json(v))),
                        Method::Service(s) => service.insert(s, try!(FromJson::from_json(v))),
                        Method::Tweet(t) => tweet.insert(t, try!(FromJson::from_json(v))),
                        Method::User(u) => user.insert(u, try!(FromJson::from_json(v))),
                    };
                }
            }
        }
        else {
            return Err(InvalidResponse("RateLimitStatus field 'resources' wasn't an object",
                                       Some(input.to_string())));
        }

        Ok(RateLimitStatus {
            direct: direct,
            place: place,
            search: search,
            service: service,
            tweet: tweet,
            user: user,
        })
    }
}

///Method identifiers, used by `rate_limit_status` to return rate-limit information.
enum Method {
    ///A method from the `direct` module.
    Direct(DirectMethod),
    ///A method from the `place` module.
    Place(PlaceMethod),
    ///A method from the `search` module.
    Search(SearchMethod),
    ///A method from the `service` module.
    Service(ServiceMethod),
    ///A method from the `tweet` module.
    Tweet(TweetMethod),
    ///A method from the `user` module.
    User(UserMethod),
}

impl FromStr for Method {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "/direct_messages" => Ok(Method::Direct(DirectMethod::Received)),
            "/direct_messages/sent" => Ok(Method::Direct(DirectMethod::Sent)),
            "/direct_messages/show" => Ok(Method::Direct(DirectMethod::Show)),

            "/geo/search" => Ok(Method::Place(PlaceMethod::Search)),
            "/geo/reverse_geocode" => Ok(Method::Place(PlaceMethod::ReverseGeocode)),
            "/geo/id/:place_id" => Ok(Method::Place(PlaceMethod::Show)),

            "/search/tweets" => Ok(Method::Search(SearchMethod::Search)),

            "/help/configuration" => Ok(Method::Service(ServiceMethod::Config)),
            "/help/privacy" => Ok(Method::Service(ServiceMethod::Privacy)),
            "/help/tos" => Ok(Method::Service(ServiceMethod::Terms)),
            "/account/verify_credentials" => Ok(Method::Service(ServiceMethod::VerifyTokens)),
            "/application/rate_limit_status" => Ok(Method::Service(ServiceMethod::RateLimitStatus)),

            "/statuses/mentions_timeline" => Ok(Method::Tweet(TweetMethod::MentionsTimeline)),
            "/statuses/user_timeline" => Ok(Method::Tweet(TweetMethod::UserTimeline)),
            "/statuses/home_timeline" => Ok(Method::Tweet(TweetMethod::HomeTimeline)),
            "/statuses/retweets_of_me" => Ok(Method::Tweet(TweetMethod::RetweetsOfMe)),
            "/statuses/retweets/:id" => Ok(Method::Tweet(TweetMethod::RetweetsOf)),
            "/statuses/show/:id" => Ok(Method::Tweet(TweetMethod::Show)),
            "/statuses/retweeters/ids" => Ok(Method::Tweet(TweetMethod::RetweetersOf)),
            "/statuses/lookup" => Ok(Method::Tweet(TweetMethod::Lookup)),
            "/favorites/list" => Ok(Method::Tweet(TweetMethod::LikedBy)),

            "/users/show/:id" => Ok(Method::User(UserMethod::Show)),
            "/users/lookup" => Ok(Method::User(UserMethod::Lookup)),
            "/users/search" => Ok(Method::User(UserMethod::Search)),
            "/friends/list" => Ok(Method::User(UserMethod::FriendsOf)),
            "/friends/ids" => Ok(Method::User(UserMethod::FriendsIds)),
            "/friendships/incoming" => Ok(Method::User(UserMethod::IncomingRequests)),
            "/friendships/outgoing" => Ok(Method::User(UserMethod::OutgoingRequests)),
            "/friendships/no_retweets/ids" => Ok(Method::User(UserMethod::FriendsNoRetweets)),
            "/followers/list" => Ok(Method::User(UserMethod::FollowersOf)),
            "/followers/ids" => Ok(Method::User(UserMethod::FollowersIds)),
            "/blocks/list" => Ok(Method::User(UserMethod::Blocks)),
            "/blocks/ids" => Ok(Method::User(UserMethod::BlocksIds)),
            "/users/report_spam" => Ok(Method::User(UserMethod::ReportSpam)),
            "/mutes/users/list" => Ok(Method::User(UserMethod::Mutes)),
            "/mutes/users/ids" => Ok(Method::User(UserMethod::MutesIds)),
            "/friendships/show" => Ok(Method::User(UserMethod::Relation)),
            "/friendships/lookup" => Ok(Method::User(UserMethod::RelationLookup)),

            _ => Err(()),
        }
    }
}

///Method identifiers from the `direct` module, for use by `rate_limit_status`.
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum DirectMethod {
    ///`direct::show`
    Show,
    ///`direct::sent`
    Sent,
    ///`direct::received`
    Received,
}

///Method identifiers from the `place` module, for use by `rate_limit_status`.
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum PlaceMethod {
    ///`place::show`
    Show,
    ///`place::search_point`, `place::search_query`, `place::search_ip` and `place::search_url`
    Search,
    ///`place::reverse_geocode` and `place::reverse_geocode_url`
    ReverseGeocode,
}

///Method identifiers from the `search` module, for use by `rate_limit_status`.
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum SearchMethod {
    ///`search::search`
    Search,
}

///Method identifiers from the `service` module, for use by `rate_limit_status`. Also includes
///`verify_tokens` from the egg-mode top-level methods.
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum ServiceMethod {
    ///`service::terms`
    Terms,
    ///`service::privacy`
    Privacy,
    ///`service::config`
    Config,
    ///`service::rate_limit_status`
    RateLimitStatus,
    ///`verify_tokens`
    VerifyTokens,
}

///Method identifiers from the `tweet` module, for use by `rate_limit_status`.
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum TweetMethod {
    ///`tweet::show`
    Show,
    ///`tweet::lookup`
    Lookup,
    ///`tweet::retweeters_of`
    RetweetersOf,
    ///`tweet::retweets_of`
    RetweetsOf,
    ///`tweet::home_timeline`
    HomeTimeline,
    ///`tweet::mentions_timeline`
    MentionsTimeline,
    ///`tweet::user_timeline`
    UserTimeline,
    ///`tweet::retweets_of_me`
    RetweetsOfMe,
    ///`tweet::liked_by`
    LikedBy,
}

///Method identifiers from the `user` module, for use by `rate_limit_status`.
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum UserMethod {
    ///`user::show`
    Show,
    ///`user::lookup`
    Lookup,
    ///`user::friends_no_retweets`
    FriendsNoRetweets,
    ///`user::relation`
    Relation,
    ///`user::relation_lookup`
    RelationLookup,
    ///`user::search`
    Search,
    ///`user::friends_of`
    FriendsOf,
    ///`user::friends_ids`
    FriendsIds,
    ///`user::followers_of`
    FollowersOf,
    ///`user::followers_ids`
    FollowersIds,
    ///`user::blocks`
    Blocks,
    ///`user::blocks_ids`
    BlocksIds,
    ///`user::mutes`
    Mutes,
    ///`user::mutes_ids`
    MutesIds,
    ///`user::incoming_requests`
    IncomingRequests,
    ///`user::outgoing_requests`
    OutgoingRequests,
    ///`user::report_spam`
    ReportSpam,
}
