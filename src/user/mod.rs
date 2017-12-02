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

use std::borrow::Cow;
use std::collections::HashMap;

use futures::{Future, Stream, Poll, Async};
use rustc_serialize::json;
use chrono;

use auth;
use common::*;
use entities;
use error;
use error::Error::InvalidResponse;
use links;
use tweet;

mod fun;

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
/// This way, when a function in egg-mode has a paremeter of type `T: Into<UserID<'a>>`, you can
/// call it with any of these types, and it will be converted automatically. egg-mode will then use
/// the proper parameter when performing the call to Twitter.
#[derive(Debug, Copy, Clone)]
pub enum UserID<'a> {
    /// Referring via the account's numeric ID.
    ID(u64),
    /// Referring via the account's screen name.
    ScreenName(&'a str),
}

impl<'a> From<u64> for UserID<'a> {
    fn from(id: u64) -> UserID<'a> {
        UserID::ID(id)
    }
}

impl<'a> From<&'a u64> for UserID<'a> {
    fn from(id: &'a u64) -> UserID<'a> {
        UserID::ID(*id)
    }
}

impl<'a> From<&'a str> for UserID<'a> {
    fn from(name: &'a str) -> UserID<'a> {
        UserID::ScreenName(name)
    }
}

impl<'a, 'b> From<&'b &'a str> for UserID<'a> {
    fn from(name: &'b &'a str) -> UserID<'a> {
        UserID::ScreenName(*name)
    }
}

impl<'a> From<&'a String> for UserID<'a> {
    fn from(name: &'a String) -> UserID<'a> {
        UserID::ScreenName(name.as_str())
    }
}

impl<'a> From<&'a UserID<'a>> for UserID<'a> {
    fn from(id: &'a UserID<'a>) -> UserID<'a> {
        *id
    }
}

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
    /// Indicates whether the authenticating user is following this account. Deprecated
    /// (and thus hidden) due to increasing error conditions where this returns None.
    following: Option<bool>,
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
    pub lang: String,
    /// The number of public lists the user is a member of.
    pub listed_count: i32,
    /// The user-entered location field from their profile. Not necessarily parseable
    /// or even a location.
    pub location: Option<String>,
    /// The user-entered display name.
    pub name: String,
    /// Indicates whether the authenticated user has chosen to received this user's tweets
    /// via SMS. Deprecated (and thus hidden) due to bugs where this incorrectly returns
    /// false.
    notifications: Option<bool>,
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

/// Container for URL entity information that may be paired with a user's profile.
#[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub struct UserEntityDetail {
    /// Collection of URL entity information.
    ///
    /// There should be one of these per URL in the paired field. In the case of the user's
    /// `description`, if no URLs are present, this field will still be present, but empty.
    pub urls: Vec<entities::UrlEntity>,
}

impl FromJson for TwitterUser {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse("TwitterUser received json that wasn't an object", Some(input.to_string())));
        }

        field_present!(input, contributors_enabled);
        field_present!(input, created_at);
        field_present!(input, default_profile);
        field_present!(input, default_profile_image);
        field_present!(input, entities);
        field_present!(input, favourites_count);
        field_present!(input, followers_count);
        field_present!(input, friends_count);
        field_present!(input, geo_enabled);
        field_present!(input, id);
        field_present!(input, is_translator);
        field_present!(input, lang);
        field_present!(input, listed_count);
        field_present!(input, name);
        field_present!(input, profile_background_color);
        field_present!(input, profile_image_url);
        field_present!(input, profile_image_url_https);
        field_present!(input, profile_link_color);
        field_present!(input, profile_sidebar_border_color);
        field_present!(input, profile_sidebar_fill_color);
        field_present!(input, profile_text_color);
        field_present!(input, profile_use_background_image);
        field_present!(input, protected);
        field_present!(input, screen_name);
        field_present!(input, statuses_count);
        field_present!(input, verified);

        let description: Option<String> = try!(field(input, "description"));
        let url: Option<String> = try!(field(input, "url"));
        let mut entities: UserEntities = try!(field(input, "entities"));

        if let Some(ref text) = description {
            for entity in entities.description.urls.iter_mut() {
                codepoints_to_bytes(&mut entity.range, &text);
            }
        }
        if let (&Some(ref text), &mut Some(ref mut entities)) = (&url, &mut entities.url) {
            for entity in entities.urls.iter_mut() {
                codepoints_to_bytes(&mut entity.range, &text);
            }
        }

        Ok(TwitterUser {
            contributors_enabled: field(input, "contributors_enabled").unwrap_or(false),
            created_at: try!(field(input, "created_at")),
            default_profile: try!(field(input, "default_profile")),
            default_profile_image: try!(field(input, "default_profile_image")),
            description: description,
            entities: entities,
            favourites_count: try!(field(input, "favourites_count")),
            follow_request_sent: try!(field(input, "follow_request_sent")),
            following: try!(field(input, "following")),
            followers_count: try!(field(input, "followers_count")),
            friends_count: try!(field(input, "friends_count")),
            geo_enabled: try!(field(input, "geo_enabled")),
            id: try!(field(input, "id")),
            is_translator: try!(field(input, "is_translator")),
            lang: try!(field(input, "lang")),
            listed_count: try!(field(input, "listed_count")),
            location: try!(field(input, "location")),
            name: try!(field(input, "name")),
            notifications: try!(field(input, "notifications")),
            profile_background_color: try!(field(input, "profile_background_color")),
            profile_background_image_url: try!(field(input, "profile_background_image_url")),
            profile_background_image_url_https: try!(field(input, "profile_background_image_url_https")),
            profile_background_tile: try!(field(input, "profile_background_tile")),
            profile_banner_url: try!(field(input, "profile_banner_url")),
            profile_image_url: try!(field(input, "profile_image_url")),
            profile_image_url_https: try!(field(input, "profile_image_url_https")),
            profile_link_color: try!(field(input, "profile_link_color")),
            profile_sidebar_border_color: try!(field(input, "profile_sidebar_border_color")),
            profile_sidebar_fill_color: try!(field(input, "profile_sidebar_fill_color")),
            profile_text_color: try!(field(input, "profile_text_color")),
            profile_use_background_image: try!(field(input, "profile_use_background_image")),
            protected: try!(field(input, "protected")),
            screen_name: try!(field(input, "screen_name")),
            show_all_inline_media: try!(field(input, "show_all_inline_media")),
            status: try!(field(input, "status")),
            statuses_count: try!(field(input, "statuses_count")),
            time_zone: try!(field(input, "time_zone")),
            url: url,
            utc_offset: try!(field(input, "utc_offset")),
            verified: try!(field(input, "verified")),
            withheld_in_countries: input.find("withheld_in_countries").and_then(|f| f.as_array())
                                        .and_then(|arr| arr.iter().map(|x| x.as_string().map(|x| x.to_string()))
                                                           .collect::<Option<Vec<String>>>()),
            withheld_scope: try!(field(input, "withheld_scope")),
        })
    }
}

impl FromJson for UserEntities {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse("UserEntities received json that wasn't an object", Some(input.to_string())));
        }

        field_present!(input, description);

        Ok(UserEntities {
            description: try!(field(input, "description")),
            url: try!(field(input, "url")),
        })
    }
}

impl FromJson for UserEntityDetail {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse("UserEntityDetail received json that wasn't an object", Some(input.to_string())));
        }

        Ok(UserEntityDetail {
            urls: try!(field(input, "urls")),
        })
    }
}

/// Represents an active user search.
///
/// This struct is returned by [`search`][] and is meant to be used as a `Stream`. That means all
/// the Stream adaptors are available:
///
/// [`search`]: fn.search.html
///
/// ```rust,no_run
/// # extern crate egg_mode; extern crate tokio_core; extern crate futures;
/// # use egg_mode::Token; use tokio_core::reactor::{Core, Handle};
/// use futures::Stream;
///
/// # fn main() {
/// # let (token, mut core, handle): (Token, Core, Handle) = unimplemented!();
/// core.run(egg_mode::user::search("rustlang", &token, &handle).take(10).for_each(|resp| {
///     println!("{}", resp.screen_name);
///     Ok(())
/// })).unwrap();
/// # }
/// ```
///
/// You can even collect the results, letting you get one set of rate-limit information for the
/// entire search setup:
///
/// ```rust,no_run
/// # extern crate egg_mode; extern crate tokio_core; extern crate futures;
/// # use egg_mode::Token; use tokio_core::reactor::{Core, Handle};
/// # fn main() {
/// # let (token, mut core, handle): (Token, Core, Handle) = unimplemented!();
/// use futures::Stream;
/// use egg_mode::Response;
/// use egg_mode::user::TwitterUser;
/// use egg_mode::error::Error;
///
/// // Because Streams don't have a FromIterator adaptor, we load all the responses first, then
/// // collect them into the final Vec
/// let names: Result<Response<Vec<TwitterUser>>, Error> =
///     core.run(egg_mode::user::search("rustlang", &token, &handle).take(10).collect())
///         .map(|resp| resp.into_iter().collect());
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
/// `Ok(Async::NotReady)`, by which time the network call has already started. If you want to know
/// that ahead of time, that's where the `call()` method comes in. By using `call()`, you can get
/// a page of results directly from Twitter. With that you can iterate over the results and page
/// forward and backward as needed:
///
/// ```rust,no_run
/// # extern crate egg_mode; extern crate tokio_core;
/// # use egg_mode::Token; use tokio_core::reactor::{Core, Handle};
/// # fn main() {
/// # let (token, mut core, handle): (Token, Core, Handle) = unimplemented!();
/// let mut search = egg_mode::user::search("rustlang", &token, &handle).with_page_size(20);
/// let resp = core.run(search.call()).unwrap();
///
/// for user in resp.response {
///    println!("{} (@{})", user.name, user.screen_name);
/// }
///
/// search.page_num += 1;
/// let resp = core.run(search.call()).unwrap();
///
/// for user in resp.response {
///    println!("{} (@{})", user.name, user.screen_name);
/// }
/// # }
/// ```
#[must_use = "search iterators are lazy and do nothing unless consumed"]
pub struct UserSearch<'a> {
    token: auth::Token,
    handle: Handle,
    query: Cow<'a, str>,
    /// The current page of results being returned, starting at 1.
    pub page_num: i32,
    /// The number of user records per page of results. Defaults to 10, maximum of 20.
    pub page_size: i32,
    current_loader: Option<FutureResponse<Vec<TwitterUser>>>,
    current_results: Option<ResponseIter<TwitterUser>>,
}

impl<'a> UserSearch<'a> {
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
    pub fn call(&self) -> FutureResponse<Vec<TwitterUser>> {
        let mut params = HashMap::new();
        add_param(&mut params, "q", self.query.clone());
        add_param(&mut params, "page", self.page_num.to_string());
        add_param(&mut params, "count", self.page_size.to_string());

        let req = auth::get(links::users::SEARCH, &self.token, Some(&params));

        make_parsed_future(&self.handle, req)
    }

    /// Returns a new UserSearch with the given query and tokens, with the default page size of 10.
    fn new<S: Into<Cow<'a, str>>>(query: S, token: &auth::Token, handle: &Handle)
        -> UserSearch<'a>
    {
        UserSearch {
            token: token.clone(),
            handle: handle.clone(),
            query: query.into(),
            page_num: 1,
            page_size: 10,
            current_loader: None,
            current_results: None,
        }
    }
}

impl<'a> Stream for UserSearch<'a> {
    type Item = Response<TwitterUser>;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        if let Some(mut fut) = self.current_loader.take() {
            match fut.poll() {
                Ok(Async::NotReady) => {
                    self.current_loader = Some(fut);
                    return Ok(Async::NotReady);
                }
                Ok(Async::Ready(res)) => self.current_results = Some(res.into_iter()),
                Err(e) => {
                    //Invalidate current results so we don't increment the page number again
                    self.current_results = None;
                    return Err(e);
                }
            }
        }

        if let Some(ref mut results) = self.current_results {
            if let Some(user) = results.next() {
                return Ok(Async::Ready(Some(user)));
            } else if (results.len() as i32) < self.page_size {
                return Ok(Async::Ready(None));
            } else {
                self.page_num += 1;
            }
        }

        self.current_loader = Some(self.call());
        self.poll()
    }
}

/// Represents relationship settings between two Twitter accounts.
#[derive(Debug)]
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

impl FromJson for Relationship {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse("Relationship received json that wasn't an object", Some(input.to_string())));
        }

        if let Some(relation) = input.find("relationship") {
            field_present!(relation, target);
            field_present!(relation, source);

            Ok(Relationship {
                target: try!(field(relation, "target")),
                source: try!(field(relation, "source")),
            })
        } else {
            Err(error::Error::MissingValue("relationship"))
        }
    }
}

/// Represents relationship settings between two Twitter accounts, from the perspective of the
/// target user.
#[derive(Debug)]
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

impl FromJson for RelationTarget {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse("RelationTarget received json that wasn't an object", Some(input.to_string())));
        }

        field_present!(input, id);
        field_present!(input, screen_name);
        field_present!(input, followed_by);
        field_present!(input, following);

        Ok(RelationTarget {
            id: try!(field(input, "id")),
            screen_name: try!(field(input, "screen_name")),
            followed_by: try!(field(input, "followed_by")),
            following: try!(field(input, "following")),
        })
    }
}

/// Represents relationship settings between two Twitter accounts, from the perspective of the
/// source user.
///
/// This struct holds more information than the `RelationTarget` struct, mainly attributes only
/// visible to the user that set them. While you can see relationships between any two arbitrary
/// users, if the "source" account is the same one whose access token you're using, you can see
/// extra information about this relationship.
#[derive(Debug)]
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

impl FromJson for RelationSource {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse("RelationSource received json that wasn't an object", Some(input.to_string())));
        }

        field_present!(input, id);
        field_present!(input, screen_name);
        field_present!(input, following);
        field_present!(input, followed_by);
        field_present!(input, can_dm);

        Ok(RelationSource {
            id: try!(field(input, "id")),
            screen_name: try!(field(input, "screen_name")),
            following: try!(field(input, "following")),
            followed_by: try!(field(input, "followed_by")),
            can_dm: try!(field(input, "can_dm")),
            blocking: try!(field(input, "blocking")),
            marked_spam: try!(field(input, "marked_spam")),
            all_replies: try!(field(input, "all_replies")),
            want_retweets: try!(field(input, "want_retweets")),
            notifications_enabled: try!(field(input, "notifications_enabled")),
        })
    }
}

/// Represents the relation the authenticated user has to a given account.
///
/// This is returned by `relation_lookup`, as opposed to `Relationship`, which is returned by
/// `relation`.
#[derive(Debug)]
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

impl FromJson for RelationLookup {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse("RelationLookup received json that wasn't an object", Some(input.to_string())));
        }

        field_present!(input, name);
        field_present!(input, screen_name);
        field_present!(input, id);
        field_present!(input, connections);

        Ok(RelationLookup {
            name: try!(field(input, "name")),
            screen_name: try!(field(input, "screen_name")),
            id: try!(field(input, "id")),
            connections: try!(field(input, "connections")),
        })
    }
}

/// Represents the ways a target account can be connected to another account.
#[derive(Debug)]
pub enum Connection {
    /// The target account has no relation.
    None,
    /// The authenticated user has requested to follow the target account.
    FollowingRequested,
    /// The target account has requested to follow the authenticated user.
    FollowingReceived,
    /// The target account follows the authenticated user.
    FollowedBy,
    /// The authenticated user follows the target account.
    Following,
    /// The authenticated user has blocked the target account.
    Blocking,
    /// The authenticated user has muted the target account.
    Muting,
}

impl FromJson for Connection {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if let Some(text) = input.as_string() {
            match text {
                "none" => Ok(Connection::None),
                "following_requested" => Ok(Connection::FollowingRequested),
                "following_received" => Ok(Connection::FollowingReceived),
                "followed_by" => Ok(Connection::FollowedBy),
                "following" => Ok(Connection::Following),
                "blocking" => Ok(Connection::Blocking),
                "muting" => Ok(Connection::Muting),
                _ => Err(InvalidResponse("unexpected string for Connection", Some(text.to_string()))),
            }
        } else {
            Err(InvalidResponse("Connection received json that wasn't a string", Some(input.to_string())))
        }
    }
}
