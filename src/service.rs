// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Methods to inquire about the Twitter service itself.
//!
//! The functions included in this module are supplementary queries that are less about specific
//! actions, and more about your interaction with the Twitter service as a whole. For example, this
//! module includes methods to load the [Terms of Service][terms] or [Privacy Policy][privacy], or
//! to ask about many methods' [rate-limit status][] or receive information about [various
//! configuration elements][config] for broad service-level values. All the structs and enums
//! contained in this module are connected to one of these methods.
//!
//! [terms]: fn.terms.html
//! [privacy]: fn.privacy.html
//! [rate-limit status]: fn.rate_limit_status.html
//! [config]: fn.config.html

use std::collections::HashMap;
use std::result::Result as StdResult;
use std::str::FromStr;

use serde::de::Error;
use serde::{Deserialize, Deserializer};
use serde_json;

use crate::common::*;
use crate::error::{
    Error::{InvalidResponse, MissingValue},
    Result,
};
use crate::{auth, entities, links};

///Returns a future that resolves to the current Twitter Terms of Service as plain text.
///
///While the official home of Twitter's TOS is <https://twitter.com/tos>, this allows you to obtain a
///plain-text copy of it to display in your application.
pub async fn terms(token: &auth::Token) -> Result<Response<String>> {
    let req = get(links::service::TERMS, token, None);

    let ret = request_with_json_response::<serde_json::Value>(req).await?;

    let tos = ret
        .response
        .get("tos")
        .and_then(|tos| tos.as_str())
        .map(String::from)
        .ok_or(InvalidResponse("Missing field: tos", None))?;
    Ok(Response::map(ret, |_| tos))
}

///Returns a future that resolves to the current Twitter Privacy Policy as plain text.
///
///While the official home of Twitter's Privacy Policy is <https://twitter.com/privacy>, this allows
///you to obtain a plain-text copy of it to display in your application.
pub async fn privacy(token: &auth::Token) -> Result<Response<String>> {
    let req = get(links::service::PRIVACY, token, None);

    let ret = request_with_json_response::<serde_json::Value>(req).await?;

    let privacy = ret
        .response
        .get("privacy")
        .and_then(|tos| tos.as_str())
        .map(String::from)
        .ok_or(InvalidResponse("Missing field: privacy", None))?;
    Ok(Response::map(ret, |_| privacy))
}

///Returns a future that resolves to the list of languages supported by Twitter API.
///
///See the documentation for the [`Lang`][] struct for discussion of what individual fields returned
///in Vec mean.
///
///[`Lang`]: struct.Lang.html
pub async fn langs(token: &auth::Token) -> Result<Response<Vec<Lang>>> {
    let req = get(links::service::LANGS, token, None);
    request_with_json_response::<Vec<Lang>>(req).await
}

///Returns a future that resolves to the current configuration from Twitter, including the maximum
///length of a t.co URL and maximum photo resolutions per size, among others.
///
///From Twitter: "It is recommended applications request this endpoint when they are loaded, but no
///more than once a day."
///
///See the documentation for the [`Configuration`][] struct for a discussion of what individual
///fields returned by this function mean.
///
///[`Configuration`]: struct.Configuration.html
pub async fn config(token: &auth::Token) -> Result<Response<Configuration>> {
    let req = get(links::service::CONFIG, token, None);
    request_with_json_response(req).await
}

///Return the current rate-limit status for all available methods from the authenticated user.
///
///The struct returned by this method is organized by what module in egg-mode a given method
///appears in. Note that not every method's status is available through this method; see the
///documentation for [`RateLimitStatus`][] and its associated enums for more information.
///
///[`RateLimitStatus`]: struct.RateLimitStatus.html
pub async fn rate_limit_status(token: &auth::Token) -> Result<Response<RateLimitStatus>> {
    let req = get(links::service::RATE_LIMIT_STATUS, token, None);
    request_with_json_response(req).await
}

///Like `rate_limit_status`, but returns the raw JSON without processing it. Only intended to
///return the full structure so that new methods can be added to `RateLimitStatus` and its
///associated enums.
#[doc(hidden)]
pub async fn rate_limit_status_raw(token: &auth::Token) -> Result<Response<serde_json::Value>> {
    let req = get(links::service::RATE_LIMIT_STATUS, token, None);
    request_with_json_response(req).await
}

///Represents a single language supported by the Twitter API.
///
///The language `code` may be formatted as ISO 639-1 alpha-2 (en), ISO 639-3 alpha-3 (msa), 
///or ISO 639-1 alpha-2 combined with an ISO 3166-1 alpha-2 localization (zh-tw).
#[derive(Debug, Deserialize)]
pub struct Lang {
    ///Language code such as `en`, `hi`, `en-gb`, etc.
    pub code: String,
    ///Status whether language is in `production` or not.
    pub status: String,
    ///Name of the language such as `Polish`, `Chinese (Simplified)`, etc.
    pub name: String,
}

///Represents a service configuration from Twitter.
///
///The values returned in this struct are various pieces of information that, while they don't
///change often, have the opportunity to change over time and affect things like character counting
///or whether to route a twitter.com URL to a user lookup or a browser.
///
///While tweets themselves still have a fixed 280-character limit, direct messages have had their
///text limit expanded to 10,000 characters, and that length is communicated here, in
///`dm_text_character_limit`.
///
///For `photo_sizes`, note that if your image is smaller than the dimensions given for a particular
///size, that size variant will simply return your source image as-is. If either dimension is
///larger than its corresponding dimension here, it will be scaled according to the included
///`resize` property. In practice this usually means `thumb` will crop to its dimensions, and each
///other variant will resize down, keeping its aspect ratio.
///
///For best ways to handle the `short_url_length` fields, see Twitter's documentation on [t.co
///URLs][]. In short, every URL Twitter detects in a new tweet or direct message gets a new t.co
///URL created for it, which replaces the original URL in the given text. This affects character
///counts for these fields, so if your app is counting characters and detects a URL for these
///fields, treat the whole URL as if it were as long as the number of characters given in this
///struct.
///
///[t.co URLs]: https://developer.twitter.com/en/docs/basics/tco
///
///Finally, loading `non_username_paths` allows you to handle `twitter.com/[name]` links as if they
///were a user mention, while still keeping site-level links working properly.
#[derive(Debug, Deserialize)]
pub struct Configuration {
    ///The character limit in direct messages.
    pub dm_text_character_limit: i32,
    ///The maximum dimensions for each photo size variant.
    pub photo_sizes: entities::MediaSizes,
    ///The maximum length for a t.co URL when given a URL with protocol `http`.
    pub short_url_length: i32,
    ///The maximum length for a t.co URL when given a URL with protocol `https`.
    pub short_url_length_https: i32,
    ///A list of URL slugs that are not valid usernames when in a URL like `twitter.com/[slug]`.
    pub non_username_paths: Vec<String>,
}

/// Represents the current rate-limit status of many Twitter API calls.
///
/// This is organized by module, so for example, if you wanted to see your rate-limit status for
/// `tweet::home_timeline`, you could access it like this:
///
/// ```rust,no_run
/// # use egg_mode::Token;
/// # #[tokio::main]
/// # async fn main() {
/// # let token: Token = unimplemented!();
/// # let status = egg_mode::service::rate_limit_status(&token).await.unwrap();
/// use egg_mode::service::TweetMethod;
/// println!("home_timeline calls remaining: {}",
///          status.tweet[&TweetMethod::HomeTimeline].rate_limit_status.remaining);
/// # }
/// ```
///
/// It's important to note that not every API method is available through this call. Namely, most
/// calls that require a POST under-the-hood (those that add or modify data with the Twitter
/// service) are not shown through this method. For a listing of methods available for rate-limit
/// querying, see the `*Method` enums available in [`egg_mode::service`][].
///
/// [`egg_mode::service`]: index.html
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
    ///The rate-limit status for methods in the `list` module.
    pub list: HashMap<ListMethod, Response<()>>,
}

impl<'de> Deserialize<'de> for RateLimitStatus {
    fn deserialize<D>(ser: D) -> StdResult<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde_json::from_value;

        let input = serde_json::Value::deserialize(ser)?;

        let mut direct = HashMap::new();
        let mut place = HashMap::new();
        let mut search = HashMap::new();
        let mut service = HashMap::new();
        let mut tweet = HashMap::new();
        let mut user = HashMap::new();
        let mut list = HashMap::new();

        let map = input
            .get("resources")
            .ok_or_else(|| D::Error::custom(MissingValue("resources")))?;

        if let Some(map) = map.as_object() {
            for (k, v) in map
                .values()
                .filter_map(|v| v.as_object())
                .flat_map(|v| v.iter())
            {
                if let Ok(method) = k.parse::<Method>() {
                    match method {
                        Method::Direct(m) => {
                            direct.insert(m, from_value(v.clone()).map_err(D::Error::custom)?)
                        }
                        Method::Place(p) => {
                            place.insert(p, from_value(v.clone()).map_err(D::Error::custom)?)
                        }
                        Method::Search(s) => {
                            search.insert(s, from_value(v.clone()).map_err(D::Error::custom)?)
                        }
                        Method::Service(s) => {
                            service.insert(s, from_value(v.clone()).map_err(D::Error::custom)?)
                        }
                        Method::Tweet(t) => {
                            tweet.insert(t, from_value(v.clone()).map_err(D::Error::custom)?)
                        }
                        Method::User(u) => {
                            user.insert(u, from_value(v.clone()).map_err(D::Error::custom)?)
                        }
                        Method::List(l) => {
                            list.insert(l, from_value(v.clone()).map_err(D::Error::custom)?)
                        }
                    };
                }
            }
        } else {
            return Err(D::Error::custom(InvalidResponse(
                "RateLimitStatus field 'resources' wasn't an object",
                Some(input.to_string()),
            )));
        }

        Ok(RateLimitStatus {
            direct,
            place,
            search,
            service,
            tweet,
            user,
            list,
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
    ///A method from the `list` module.
    List(ListMethod),
}

impl FromStr for Method {
    type Err = ();

    fn from_str(s: &str) -> StdResult<Self, ()> {
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
            "/help/languages" => Ok(Method::Service(ServiceMethod::Langs)),
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

            "/lists/show" => Ok(Method::List(ListMethod::Show)),
            "/lists/ownerships" => Ok(Method::List(ListMethod::Ownerships)),
            "/lists/subscriptions" => Ok(Method::List(ListMethod::Subscriptions)),
            "/lists/list" => Ok(Method::List(ListMethod::List)),
            "/lists/members" => Ok(Method::List(ListMethod::Members)),
            "/lists/memberships" => Ok(Method::List(ListMethod::Memberships)),
            "/lists/members/show" => Ok(Method::List(ListMethod::IsMember)),
            "/lists/subscribers" => Ok(Method::List(ListMethod::Subscribers)),
            "/lists/subscribers/show" => Ok(Method::List(ListMethod::IsSubscribed)),
            "/lists/statuses" => Ok(Method::List(ListMethod::Statuses)),

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
    ///`service::langs`
    Langs,
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

///Method identifiers from the `list` module, for use by `rate_limit_status`.
#[derive(Debug, PartialEq, Eq, Hash)]
pub enum ListMethod {
    ///`list::show`
    Show,
    ///`list::ownerships`
    Ownerships,
    ///`list::subscriptions`
    Subscriptions,
    ///`list::list`
    List,
    ///`list::members`
    Members,
    ///`list::memberships`
    Memberships,
    ///`list::is_member`
    IsMember,
    ///`list::subscribers`
    Subscribers,
    ///`list::is_subscribed`
    IsSubscribed,
    ///`list::statuses`
    Statuses,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::common::tests::load_file;

    #[test]
    fn parse_rate_limit() {
        let sample = load_file("sample_payloads/rate_limit_sample.json");
        ::serde_json::from_str::<RateLimitStatus>(&sample).unwrap();
    }

    #[test]
    fn parse_langs() {
        let sample = load_file("sample_payloads/sample-languages.json");
        ::serde_json::from_str::<Vec<Lang>>(&sample).unwrap();
    }
}
