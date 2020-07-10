// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Structs and methods for pulling user information from Twitter.
//!
//! Everything in here acts on users in some way, whether looking up user information, finding the
//! relations between two users, or actions like following or blocking a user.
//!
//! ## Types
//!
//! - `UserID`: used as a generic input to many functions, this enum allows you to refer to a user
//!   by a numeric ID or by their screen name.
//! - `Relationship`/`RelationSource`/`RelationTarget`: returned by `relation`, these types
//!   (`Relationship` contains the other two) show the ways two accounts relate to each other.
//! - `RelationLookup`/`Connection`: returned as part of a collection by `relation_lookup`, these
//!   types (`RelationLookup` contains a `Vec<Connection>`) shows the ways the authenticated user
//!   relates to a specific account.
//! - `TwitterUser`/`UserEntities`/`UserEntityDetail`: returned by many functions in this module,
//!   these types (`TwitterUser` contains the other two) describe the content of a user's profile,
//!   and a handful of settings relating to how their profile is displayed.
//! - `UserSearch`: returned by `search`, this is a stream of search results.
//!
//! ## Functions
//!
//! ### User actions
//!
//! These functions perform actions to the user's account. Their use requires that your application
//! request write access to authenticated accounts.
//!
//! - `block`/`report_spam`/`unblock`
//! - `follow`/`unfollow`/`update_follow`
//! - `mute`/`unmute`
//!
//! ### Direct lookup
//!
//! These functions return single users, or groups of users without having to iterate over the
//! results.
//!
//! - `show`
//! - `lookup`/`lookup_ids`/`lookup_names`
//! - `friends_no_retweets`
//! - `relation`/`relation_lookup`
//!
//! ### Cursored lookup
//!
//! These functions imply that they can return more entries than Twitter is willing to return at
//! once, so they're delivered in pages. This library takes those paginated results and wraps a
//! stream around them that loads the pages as-needed.
//!
//! - `search`
//! - `friends_of`/`friends_ids`
//! - `followers_of`/`followers_ids`
//! - `blocks`/`blocks_ids`
//! - `mutes`/`mutes_ids`
//! - `incoming_requests`/`outgoing_requests`

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::vec::IntoIter as VecIter;

use chrono;
use futures::Stream;
use serde::{Deserialize, Serialize};

use crate::common::*;
use crate::{auth, entities, error, links, tweet};

mod fun;
mod raw;

pub use self::fun::*;

/// Convenience enum to generalize between referring to an account by numeric ID or by screen name.
///
/// Many API calls ask for a user either by either screen name (e.g. `rustlang`) or by a numeric ID
/// assigned to the account (e.g. `165262228`). In egg-mode, these calls are abstracted around this
/// enum, and can take any type that converts into it. This enum has `From` implementations for the
/// following types:
///
/// * `u64`
/// * `&u64` (convenient when used with iterators)
/// * `&str`
/// * `&&str` (convenient when used with iterators)
/// * `&String` (to counteract the fact that deref coercion doesn't work with generics)
/// * `&UserID` (convenient when used with iterators)
///
/// This way, when a function in egg-mode has a paremeter of type `T: Into<UserID>`, you can
/// call it with any of these types, and it will be converted automatically. egg-mode will then use
/// the proper parameter when performing the call to Twitter.
#[derive(Debug, Clone, derive_more::From)]
pub enum UserID {
    /// Referring via the account's numeric ID.
    ID(u64),
    /// Referring via the account's screen name.
    ScreenName(CowStr),
}

impl<'a> From<&'static str> for UserID {
    fn from(name: &'static str) -> UserID {
        UserID::ScreenName(name.into())
    }
}

impl From<String> for UserID {
    fn from(name: String) -> UserID {
        UserID::ScreenName(name.into())
    }
}

round_trip! { raw::RawTwitterUser,
    /// Represents a Twitter user.
    ///
    /// Field-level documentation is mostly ripped wholesale from [Twitter's user
    /// documentation][api-user].
    ///
    /// [api-user]: https://dev.twitter.com/overview/api/users
    ///
    /// The fields present in this struct can be divided up into a few sections: Profile Information and
    /// Settings.
    ///
    /// ## Profile Information
    ///
    /// Information here can be considered part of the user's profile. These fields are the "obvious"
    /// visible portion of a profile view.
    ///
    /// * `id`
    /// * `screen_name`
    /// * `name`
    /// * `verified`
    /// * `protected`
    /// * `description`
    /// * `location`
    /// * `url`
    /// * `statuses_count`
    /// * `friends_count`
    /// * `followers_count`
    /// * `favourites_count`
    /// * `listed_count`
    /// * `profile_image_url`/`profile_image_url_https`
    /// * `profile_banner_url`
    ///
    /// ## Settings Information
    ///
    /// Information here can be used to alter the UI around this user, or to provide further metadata
    /// that may not necessarily be user-facing.
    ///
    /// * `contributors_enabled`
    /// * `created_at`
    /// * `default_profile_image`
    /// * `follow_request_sent`
    /// * `default_profile`, `profile_background_color`, `profile_background_image_url`,
    ///   `profile_background_image_url_https`, `profile_background_tile`, `profile_link_color`,
    ///   `profile_sidebar_border_color`, `profile_sidebar_fill_color`, `profile_text_color`,
    ///   `profile_use_background_image`: These fields can be used to theme a user's profile page to
    ///   look like the settings they've set on the Twitter website.
    /// * `geo_enabled`
    /// * `is_translator`
    /// * `lang`
    /// * `show_all_inline_media`
    /// * `time_zone`/`utc_offset`
    /// * `withheld_in_countries`/`withheld_scope`
    #[derive(Debug, Clone)]
    pub struct TwitterUser {
        /// Indicates this user has an account with "contributor mode" enabled, allowing
        /// for Tweets issued by the user to be co-authored by another account. Rarely `true`.
        pub contributors_enabled: bool,
        /// The UTC timestamp for when this user account was created on Twitter.
        pub created_at: chrono::DateTime<chrono::Utc>,
        /// When true, indicates that this user has not altered the theme or background of
        /// their user profile.
        pub default_profile: bool,
        /// When true, indicates that the user has not uploaded their own avatar and a default
        /// egg avatar is used instead.
        pub default_profile_image: bool,
        /// The user-defined string describing their account.
        pub description: Option<String>,
        /// Link information that has been parsed out of the `url` or `description` fields given by the
        /// user.
        pub entities: UserEntities,
        /// The number of tweets this user has favorited or liked in the account's lifetime.
        /// The term "favourites" and its British spelling are used for historical reasons.
        pub favourites_count: i32,
        /// When true, indicates that the authenticating user has issued a follow request to
        /// this protected account.
        pub follow_request_sent: Option<bool>,
        /// The number of followers this account has.
        ///
        /// In certain server-stress conditions, this may temporarily mistakenly return 0.
        pub followers_count: i32,
        /// The number of users this account follows, aka its "followings".
        ///
        /// In certain server-stress conditions, this may temporarily mistakenly return 0.
        pub friends_count: i32,
        /// Indicates whether this user as enabled their tweets to be geotagged.
        ///
        /// If this is set for the current user, then they can attach geographic data when
        /// posting a new Tweet.
        pub geo_enabled: bool,
        /// Unique identifier for this user.
        pub id: u64,
        /// Indicates whether the user participates in Twitter's translator community.
        pub is_translator: bool,
        /// Language code for the user's self-declared interface language.
        ///
        /// Codes are formatted as a language tag from [BCP 47][]. Only indicates the user's
        /// interface language, not necessarily the content of their Tweets.
        ///
        /// [BCP 47]: https://tools.ietf.org/html/bcp47
        pub lang: Option<String>,
        /// The number of public lists the user is a member of.
        pub listed_count: i32,
        /// The user-entered location field from their profile. Not necessarily parseable
        /// or even a location.
        pub location: Option<String>,
        /// The user-entered display name.
        pub name: String,
        /// The hex color chosen by the user for their profile background.
        pub profile_background_color: String,
        /// A URL pointing to the background image chosen by the user for their profile. Uses
        /// HTTP as the protocol.
        pub profile_background_image_url: Option<String>,
        /// A URL pointing to the background image chosen by the user for their profile. Uses
        /// HTTPS as the protocol.
        pub profile_background_image_url_https: Option<String>,
        /// Indicates whether the user's `profile_background_image_url` should be tiled when
        /// displayed.
        pub profile_background_tile: Option<bool>,
        /// A URL pointing to the banner image chosen by the user. Uses HTTPS as the protocol.
        ///
        /// This is a base URL that a size specifier can be appended onto to get variously
        /// sized images, with size specifiers according to [Profile Images and Banners][profile-img].
        ///
        /// [profile-img]: https://dev.twitter.com/overview/general/user-profile-images-and-banners
        pub profile_banner_url: Option<String>,
        /// A URL pointing to the user's avatar image. Uses HTTP as the protocol. Size
        /// specifiers can be used according to [Profile Images and Banners][profile-img].
        ///
        /// [profile-img]: https://dev.twitter.com/overview/general/user-profile-images-and-banners
        pub profile_image_url: String,
        /// A URL pointing to the user's avatar image. Uses HTTPS as the protocol. Size
        /// specifiers can be used according to [Profile Images and Banners][profile-img].
        ///
        /// [profile-img]: https://dev.twitter.com/overview/general/user-profile-images-and-banners
        pub profile_image_url_https: String,
        /// The hex color chosen by the user to display links in the Twitter UI.
        pub profile_link_color: String,
        /// The hex color chosen by the user to display sidebar borders in the Twitter UI.
        pub profile_sidebar_border_color: String,
        /// The hex color chosen by the user to display sidebar backgrounds in the Twitter UI.
        pub profile_sidebar_fill_color: String,
        /// The hex color chosen by the user to display text in the Twitter UI.
        pub profile_text_color: String,
        /// Indicates whether the user wants their uploaded background image to be used.
        pub profile_use_background_image: bool,
        /// Indicates whether the user is a [protected][] account.
        ///
        /// [protected]: https://support.twitter.com/articles/14016
        pub protected: bool,
        /// The screen name or handle identifying this user.
        ///
        /// Screen names are unique per-user but can be changed. Use `id` for an immutable identifier
        /// for an account.
        ///
        /// Typically a maximum of 15 characters long, but older accounts may exist with longer screen
        /// names.
        pub screen_name: String,
        /// Indicates that the user would like to see media inline. "Somewhat disused."
        pub show_all_inline_media: Option<bool>,
        /// If possible, the most recent tweet or retweet from this user.
        ///
        /// "In some circumstances, this data cannot be provided and this field will be omitted, null,
        /// or empty." Do not depend on this field being filled. Also note that this is actually their
        /// most-recent tweet, not the status pinned to their profile.
        ///
        /// "Perspectival" items within this tweet that depend on the authenticating user
        /// [may not be completely reliable][stale-embed] in this embed.
        ///
        /// [stale-embed]: https://dev.twitter.com/docs/faq/basics/why-are-embedded-objects-stale-or-inaccurate
        pub status: Option<Box<tweet::Tweet>>,
        /// The number of tweets (including retweets) posted by this user.
        pub statuses_count: i32,
        /// The full name of the time zone the user has set their UI preference to.
        pub time_zone: Option<String>,
        /// The website link given by this user in their profile.
        pub url: Option<String>,
        /// The UTC offset of `time_zone` in minutes.
        pub utc_offset: Option<i32>,
        /// Indicates whether this user is a verified account.
        pub verified: bool,
        /// When present, lists the countries this user has been withheld from.
        pub withheld_in_countries: Option<Vec<String>>,
        /// When present, indicates whether the content being withheld is a "status" or "user".
        pub withheld_scope: Option<String>,
    }
}

impl From<raw::RawTwitterUser> for TwitterUser {
    fn from(mut raw: raw::RawTwitterUser) -> TwitterUser {
        if let Some(ref description) = raw.description {
            for entity in &mut raw.entities.description.urls {
                codepoints_to_bytes(&mut entity.range, description);
            }
        }

        if let (&mut Some(ref url), &mut Some(ref mut entities)) =
            (&mut raw.url, &mut raw.entities.url)
        {
            for entity in &mut entities.urls {
                codepoints_to_bytes(&mut entity.range, url);
            }
        }

        TwitterUser {
            contributors_enabled: raw.contributors_enabled,
            created_at: raw.created_at,
            default_profile: raw.default_profile,
            default_profile_image: raw.default_profile_image,
            description: raw.description,
            entities: raw.entities,
            favourites_count: raw.favourites_count,
            follow_request_sent: raw.follow_request_sent,
            followers_count: raw.followers_count,
            friends_count: raw.friends_count,
            geo_enabled: raw.geo_enabled,
            id: raw.id,
            is_translator: raw.is_translator,
            lang: raw.lang,
            listed_count: raw.listed_count,
            location: raw.location,
            name: raw.name,
            profile_background_color: raw.profile_background_color,
            profile_background_image_url: raw.profile_background_image_url,
            profile_background_image_url_https: raw.profile_background_image_url_https,
            profile_background_tile: raw.profile_background_tile,
            profile_banner_url: raw.profile_banner_url,
            profile_image_url: raw.profile_image_url,
            profile_image_url_https: raw.profile_image_url_https,
            profile_link_color: raw.profile_link_color,
            profile_sidebar_border_color: raw.profile_sidebar_border_color,
            profile_sidebar_fill_color: raw.profile_sidebar_fill_color,
            profile_text_color: raw.profile_text_color,
            profile_use_background_image: raw.profile_use_background_image,
            protected: raw.protected,
            screen_name: raw.screen_name,
            show_all_inline_media: raw.show_all_inline_media,
            status: raw.status,
            statuses_count: raw.statuses_count,
            time_zone: raw.time_zone,
            url: raw.url,
            utc_offset: raw.utc_offset,
            verified: raw.verified,
            withheld_in_countries: raw.withheld_in_countries,
            withheld_scope: raw.withheld_scope,
        }
    }
}

/// Container for URL entity information that may be paired with a user's profile.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct UserEntities {
    /// URL information that has been parsed out of the user's `description`. If no URLs were
    /// detected, then the contained Vec will be empty.
    pub description: UserEntityDetail,
    /// Link information for the user's `url`.
    ///
    /// If `url` is present on the user's profile, so will this field. Twitter validates the URL
    /// entered to a user's profile when they save it, so this can be reasonably assumed to have URL
    /// information if it's present.
    pub url: Option<UserEntityDetail>,
}

/// Represents a collection of URL entity information paired with a specific user profile field.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct UserEntityDetail {
    /// Collection of URL entity information.
    ///
    /// There should be one of these per URL in the paired field. In the case of the user's
    /// `description`, if no URLs are present, this field will still be present, but empty.
    pub urls: Vec<entities::UrlEntity>,
}

/// Represents an active user search.
///
/// This struct is returned by [`search`][] and is meant to be used as a `Stream`. That means all
/// the Stream adaptors are available:
///
/// [`search`]: fn.search.html
///
/// ```rust,no_run
/// # use egg_mode::Token;
/// use futures::{Stream, StreamExt, TryStreamExt};
///
/// # #[tokio::main]
/// # async fn main() {
/// # let token: Token = unimplemented!();
/// egg_mode::user::search("rustlang", &token).take(10).try_for_each(|resp| {
///     println!("{}", resp.screen_name);
///     futures::future::ready(Ok(()))
/// }).await.unwrap();
/// # }
/// ```
///
/// You can even collect the results, letting you get one set of rate-limit information for the
/// entire search setup:
///
/// ```rust,no_run
/// # use egg_mode::Token;
/// # #[tokio::main]
/// # async fn main() {
/// # let token: Token = unimplemented!();
/// use futures::{Stream, StreamExt, TryStreamExt};
/// use egg_mode::Response;
/// use egg_mode::user::TwitterUser;
/// use egg_mode::error::Error;
///
/// // Because Streams don't have a FromIterator adaptor, we load all the responses first, then
/// // collect them into the final Vec
/// let names: Result<Vec<TwitterUser>, Error> =
///     egg_mode::user::search("rustlang", &token)
///         .take(10)
///         .try_collect::<Vec<_>>()
///         .await
///         .map(|res| res.into_iter().collect());
/// # }
/// ```
///
/// `UserSearch` has a couple adaptors of its own that you can use before consuming it.
/// `with_page_size` will let you set how many users are pulled in with a single network call, and
/// `start_at_page` lets you start your search at a specific page. Calling either of these after
/// starting iteration will clear any current results.
///
/// The `Stream` implementation yields `Response<TwitterUser>` on a successful iteration, and
/// `Error` for errors, so network errors, rate-limit errors and other issues are passed directly
/// through in `poll()`. The `Stream` implementation will allow you to poll again after an error to
/// re-initiate the late network call; this way, you can wait for your network connection to return
/// or for your rate limit to refresh and try again from the same position.
///
/// ## Manual paging
///
/// The `Stream` implementation works by loading in a page of results (with size set by default or
/// by `with_page_size`/the `page_size` field) when it's polled, and serving the individual
/// elements from that locally-cached page until it runs out. This can be nice, but it also means
/// that your only warning that something involves a network call is that the stream returns
/// `Poll::Pending`, by which time the network call has already started. If you want to know
/// that ahead of time, that's where the `call()` method comes in. By using `call()`, you can get
/// a page of results directly from Twitter. With that you can iterate over the results and page
/// forward and backward as needed:
///
/// ```rust,no_run
/// # use egg_mode::Token;
/// # #[tokio::main]
/// # async fn main() {
/// # let token: Token = unimplemented!();
/// let mut search = egg_mode::user::search("rustlang", &token).with_page_size(20);
/// let resp = search.call().await.unwrap();
///
/// for user in resp.response {
///    println!("{} (@{})", user.name, user.screen_name);
/// }
///
/// search.page_num += 1;
/// let resp = search.call().await.unwrap();
///
/// for user in resp.response {
///    println!("{} (@{})", user.name, user.screen_name);
/// }
/// # }
/// ```
#[must_use = "search iterators are lazy and do nothing unless consumed"]
pub struct UserSearch {
    token: auth::Token,
    query: CowStr,
    /// The current page of results being returned, starting at 1.
    pub page_num: i32,
    /// The number of user records per page of results. Defaults to 10, maximum of 20.
    pub page_size: i32,
    current_loader: Option<FutureResponse<Vec<TwitterUser>>>,
    current_results: Option<VecIter<TwitterUser>>,
}

impl UserSearch {
    /// Sets the page size used for the search query.
    ///
    /// Calling this will invalidate any current search results, making the next call to `next()`
    /// perform a network call.
    pub fn with_page_size(self, page_size: i32) -> Self {
        UserSearch {
            page_size: page_size,
            current_loader: None,
            current_results: None,
            ..self
        }
    }

    /// Sets the starting page number for the search query.
    ///
    /// The search method begins numbering pages at 1. Calling this will invalidate any current
    /// search results, making the next call to `next()` perform a network call.
    pub fn start_at_page(self, page_num: i32) -> Self {
        UserSearch {
            page_num: page_num,
            current_loader: None,
            current_results: None,
            ..self
        }
    }

    /// Performs the search for the current page of results.
    ///
    /// This will automatically be called if you use the `UserSearch` as an iterator. This method is
    /// made public for convenience if you want to manage the pagination yourself. Remember to
    /// change `page_num` between calls.
    pub fn call(&self) -> impl Future<Output = error::Result<Response<Vec<TwitterUser>>>> {
        let params = ParamList::new()
            .add_param("q", self.query.clone())
            .add_param("page", self.page_num.to_string())
            .add_param("count", self.page_size.to_string());

        let req = get(links::users::SEARCH, &self.token, Some(&params));
        request_with_json_response(req)
    }

    /// Returns a new UserSearch with the given query and tokens, with the default page size of 10.
    fn new<S: Into<CowStr>>(query: S, token: &auth::Token) -> UserSearch {
        UserSearch {
            token: token.clone(),
            query: query.into(),
            page_num: 1,
            page_size: 10,
            current_loader: None,
            current_results: None,
        }
    }
}

impl Stream for UserSearch {
    type Item = Result<TwitterUser, error::Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        if let Some(mut fut) = self.current_loader.take() {
            match Pin::new(&mut fut).poll(cx) {
                Poll::Pending => {
                    self.current_loader = Some(fut);
                    return Poll::Pending;
                }
                Poll::Ready(Ok(res)) => self.current_results = Some(res.response.into_iter()),
                Poll::Ready(Err(e)) => {
                    //Invalidate current results so we don't increment the page number again
                    self.current_results = None;
                    return Poll::Ready(Some(Err(e)));
                }
            }
        }

        if let Some(ref mut results) = self.current_results {
            if let Some(user) = results.next() {
                return Poll::Ready(Some(Ok(user)));
            } else if (results.len() as i32) < self.page_size {
                return Poll::Ready(None);
            } else {
                self.page_num += 1;
            }
        }

        self.current_loader = Some(Box::pin(self.call()));
        self.poll_next(cx)
    }
}

/// Represents relationship settings between two Twitter accounts.
#[derive(Debug, Deserialize)]
pub struct Relationship {
    /// Contains settings from the perspective of the target account.
    pub target: RelationTarget,
    /// Contains settings from the perspective of the source account.
    ///
    /// This contains more information than `target` if the source account is the same as the
    /// authenticated user. See the [`RelationSource`][] page for details.
    ///
    /// [`RelationSource`]: struct.RelationSource.html
    pub source: RelationSource,
}

/// Represents relationship settings between two Twitter accounts, from the perspective of the
/// target user.
#[derive(Debug, Deserialize)]
pub struct RelationTarget {
    /// Numeric ID for this account.
    pub id: u64,
    /// Screen name for this account.
    pub screen_name: String,
    /// Indicates whether the source account follows this target account.
    pub followed_by: bool,
    /// Indicates whether this target account follows the source account.
    pub following: bool,
}

/// Represents relationship settings between two Twitter accounts, from the perspective of the
/// source user.
///
/// This struct holds more information than the `RelationTarget` struct, mainly attributes only
/// visible to the user that set them. While you can see relationships between any two arbitrary
/// users, if the "source" account is the same one whose access token you're using, you can see
/// extra information about this relationship.
#[derive(Debug, Deserialize)]
pub struct RelationSource {
    /// Numeric ID for this account.
    pub id: u64,
    /// Screen name for this account.
    pub screen_name: String,
    /// Indicates whether this source account follows the target account.
    pub following: bool,
    /// Indicates whether the target account follows this source account.
    pub followed_by: bool,
    /// Indicates whether this source account can send a direct message to the target account.
    ///
    /// If `followed_by` is false but this is true, that could indicate that the target account has
    /// allowed anyone to direct-message them.
    pub can_dm: bool,
    /// Indicates whether this source account is blocking the target account. If the source account
    /// is not the authenticated user, holds `None` instead.
    pub blocking: Option<bool>,
    /// Indicates whether this source account has reported the target account for spam. If the source
    /// account is not the authenticated user, holds `None` instead.
    pub marked_spam: Option<bool>,
    /// Indicates whether this source account has decided to receive all replies from the target
    /// account. If the source account is not the authenticated user, holds `None` instead.
    ///
    /// Note that there is no mechanism with which to toggle this setting, at least none that this
    /// author could find, either through the API or through the official site.
    all_replies: Option<bool>,
    /// Indicates whether this source account has decided to show retweets from the target account.
    /// If the source account is not the authenticated user, holds `None` instead.
    pub want_retweets: Option<bool>,
    /// Indicates whether this source account has decided to receive mobile notifications for the
    /// target account. If the source account is not the authenticated user, holds `None` instead.
    pub notifications_enabled: Option<bool>,
}

/// Represents the relation the authenticated user has to a given account.
///
/// This is returned by `relation_lookup`, as opposed to `Relationship`, which is returned by
/// `relation`.
#[derive(Debug, Deserialize)]
pub struct RelationLookup {
    /// The display name of the target account.
    pub name: String,
    /// The screen name of the target account.
    pub screen_name: String,
    /// The numeric ID of the target account.
    pub id: u64,
    /// The ways the target account is connected to the authenticated user.
    ///
    /// If the target account has no relation to the authenticated user, this will not be empty; its
    /// only element will be `None`.
    pub connections: Vec<Connection>,
}

/// Represents the ways a target account can be connected to another account.
#[derive(Debug, Deserialize)]
pub enum Connection {
    /// The target account has no relation.
    #[serde(rename = "none")]
    None,
    /// The authenticated user has requested to follow the target account.
    #[serde(rename = "following_requested")]
    FollowingRequested,
    /// The target account has requested to follow the authenticated user.
    #[serde(rename = "following_received")]
    FollowingReceived,
    /// The target account follows the authenticated user.
    #[serde(rename = "followed_by")]
    FollowedBy,
    /// The authenticated user follows the target account.
    #[serde(rename = "following")]
    Following,
    /// The authenticated user has blocked the target account.
    #[serde(rename = "blocking")]
    Blocking,
    /// The authenticated user has muted the target account.
    #[serde(rename = "muting")]
    Muting,
}

#[cfg(test)]
mod tests {
    use super::TwitterUser;
    use crate::common::tests::load_file;

    #[test]
    fn roundtrip_deser() {
        let sample = load_file("sample_payloads/user_array.json");
        let users_src: Vec<TwitterUser> = serde_json::from_str(&sample).unwrap();
        let json1 = serde_json::to_value(users_src).unwrap();
        let users_roundtrip: Vec<TwitterUser> = serde_json::from_value(json1.clone()).unwrap();
        let json2 = serde_json::to_value(users_roundtrip).unwrap();

        assert_eq!(json1, json2);
    }
}
