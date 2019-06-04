// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Structs and functions for working with statuses and timelines.
//!
//! In this module, you can find various structs and methods to load and interact with tweets and
//! their metadata. This also includes loading a user's timeline, posting a new tweet, or liking or
//! retweeting another tweet. However, this does *not* include searching for tweets; that
//! functionality is in the [`search`][] module.
//!
//! [`search`]: ../search/index.html
//!
//! ## Types
//!
//! - `Tweet`/`TweetEntities`/`ExtendedTweetEntities`: At the bottom of it all, this is the struct
//!   that represents a single tweet. The `*Entities` structs contain information about media,
//!   links, and hashtags within their parent tweet.
//! - `DraftTweet`: This is what you use to post a new tweet. At present, not all available options
//!   are supported, but basics like marking the tweet as a reply and attaching a location
//!   coordinate are available.
//! - `Timeline`: Returned by several functions in this module, this is how you cursor through a
//!   collection of tweets. See the struct-level documentation for details.
//!
//! ## Functions
//!
//! ### User actions
//!
//! These functions perform actions on their given tweets. They require write access to the
//! authenticated user's account.
//!
//! - `delete` (for creating a tweet, see `DraftTweet`)
//! - `like`/`unlike`
//! - `retweet`/`unretweet`
//!
//! ### Metadata lookup
//!
//! These functions either perform some direct lookup of specific tweets, or provide some metadata
//! about the given tweet in a direct (non-`Timeline`) fashion.
//!
//! - `show`
//! - `lookup`/`lookup_map` (for the differences between these functions, see their respective
//!   documentations.)
//! - `retweeters_of`
//! - `retweets_of`
//!
//! ### `Timeline` cursors
//!
//! These functions return `Timeline`s and can be cursored around in the same way. See the
//! documentation for `Timeline` to learn how to navigate these return values. This correspond to a
//! user's own view of Twitter, or with feeds you might see attached to a user's profile page.
//!
//! - `home_timeline`/`mentions_timeline`/`retweets_of_me`
//! - `user_timeline`/`liked_by`

use std::borrow::Cow;
use std::collections::HashMap;
use std::str::FromStr;

use chrono;
use futures::{Async, Future, Poll};
use hyper::{Body, Request};
use regex::Regex;
use serde::de::Error;
use serde::{Deserialize, Deserializer};

use auth;
use common::*;
use entities;
use error;
use error::Error::InvalidResponse;
use links;
use place;
use stream::FilterLevel;
use user;

mod fun;
mod raw;

pub use self::fun::*;

///Represents a single status update.
///
///The fields present in this struct can be mainly split up based on the context they're present
///for.
///
///## Base Tweet Info
///
///This information is the basic information inherent to all tweets, regardless of context.
///
///* `text`
///* `id`
///* `created_at`
///* `user`
///* `source`
///* `favorite_count`/`retweet_count`
///* `lang`, though third-party clients usually don't surface this at a user-interface level.
///  Twitter Web uses this to create machine-translations of the tweet.
///* `coordinates`/`place`
///* `display_text_range`
///* `truncated`
///
///## Perspective-based data
///
///This information depends on the authenticated user who called the data. These are left as
///Options because certain contexts where the information is pulled either don't have an
///authenticated user to compare with, or don't have to opportunity to poll the user's interactions
///with the tweet.
///
///* `favorited`
///* `retweeted`
///* `current_user_retweet`
///
///## Replies
///
///This information is only present when the tweet in question is marked as being a reply to
///another tweet, or when it's threaded into a chain from the same user.
///
///* `in_reply_to_user_id`/`in_reply_to_screen_name`
///* `in_reply_to_status_id`
///
///## Retweets and Quote Tweets
///
///This information is only present when the tweet in question is a native retweet or is a "quote
///tweet" that references another tweet by linking to it. These fields allow you to reference the
///parent tweet without having to make another call to `show`.
///
///* `retweeted_status`
///* `quoted_status`/`quoted_status_id`
///
///## Media
///
///As a tweet can attach an image, GIF, or video, these fields allow you to access information
///about the attached media. Note that polls are not surfaced to the Public API at the time of this
///writing (2016-09-01). For more information about how to use attached media, see the
///documentation for [`MediaEntity`][].
///
///[`MediaEntity`]: ../entities/struct.MediaEntity.html
///
///* `entities` (note that this also contains information about hyperlinks, user mentions, and
///  hashtags in addition to a picture/thumbnail)
///* `extended_entities`: This field is only present for tweets with attached media, and houses
///  more complete media information, in the case of a photo set, video, or GIF. For videos and
///  GIFs, note that `entities` will only contain a thumbnail, and the actual video links will be
///  in this field. For tweets with more than one photo attached, `entities` will only contain the
///  first photo, and this field will contain all of them.
///* `possibly_sensitive`
///* `withheld_copyright`
///* `withheld_in_countries`
///* `withheld_scope`
#[derive(Debug, Clone)]
pub struct Tweet {
    //If the user has contributors enabled, this will show which accounts contributed to this
    //tweet.
    //pub contributors: Option<Contributors>,
    ///If present, the location coordinate attached to the tweet, as a (latitude, longitude) pair.
    pub coordinates: Option<(f64, f64)>,
    ///UTC timestamp from when the tweet was posted.
    pub created_at: chrono::DateTime<chrono::Utc>,
    ///If the authenticated user has retweeted this tweet, contains the ID of the retweet.
    pub current_user_retweet: Option<u64>,
    ///If this tweet is an extended tweet with "hidden" metadata and entities, contains the byte
    ///offsets between which the "displayable" tweet text is.
    pub display_text_range: Option<(usize, usize)>,
    ///Link, hashtag, and user mention information extracted from the tweet text.
    pub entities: TweetEntities,
    ///Extended media information attached to the tweet, if media is available.
    ///
    ///If a tweet has a photo, set of photos, gif, or video attached to it, this field will be
    ///present and contain the real media information. The information available in the `media`
    ///field of `entities` will only contain the first photo of a set, or a thumbnail of a gif or
    ///video.
    pub extended_entities: Option<ExtendedTweetEntities>,
    ///"Approximately" how many times this tweet has been liked by users.
    pub favorite_count: i32,
    ///Indicates whether the authenticated user has liked this tweet.
    pub favorited: Option<bool>,
    ///Indicates the maximum `FilterLevel` parameter that can be applied to a stream and still show
    ///this tweet.
    pub filter_level: Option<FilterLevel>,
    ///Numeric ID for this tweet.
    pub id: u64,
    ///If the tweet is a reply, contains the ID of the user that was replied to.
    pub in_reply_to_user_id: Option<u64>,
    ///If the tweet is a reply, contains the screen name of the user that was replied to.
    pub in_reply_to_screen_name: Option<String>,
    ///If the tweet is a reply, contains the ID of the tweet that was replied to.
    pub in_reply_to_status_id: Option<u64>,
    ///Can contain a language ID indicating the machine-detected language of the text, or "und" if
    ///no language could be detected.
    pub lang: Option<String>,
    ///When present, the `Place` that this tweet is associated with (but not necessarily where it
    ///originated from).
    pub place: Option<place::Place>,
    ///If the tweet has a link, indicates whether the link may contain content that could be
    ///identified as sensitive.
    pub possibly_sensitive: Option<bool>,
    ///If this tweet is quoting another by link, contains the ID of the quoted tweet.
    pub quoted_status_id: Option<u64>,
    ///If this tweet is quoting another by link, contains the quoted tweet.
    pub quoted_status: Option<Box<Tweet>>,
    //"A set of key-value pairs indicating the intended contextual delivery of the containing
    //Tweet. Currently used by Twitter’s Promoted Products."
    //pub scopes: Option<Scopes>,
    ///The number of times this tweet has been retweeted (with native retweets).
    pub retweet_count: i32,
    ///Indicates whether the authenticated user has retweeted this tweet.
    pub retweeted: Option<bool>,
    ///If this tweet is a retweet, then this field contains the original status information.
    ///
    ///The separation between retweet and original is so that retweets can be recalled by deleting
    ///the retweet, and so that liking a retweet results in an additional notification to the user
    ///who retweeted the status, as well as the original poster.
    pub retweeted_status: Option<Box<Tweet>>,
    ///The application used to post the tweet.
    pub source: TweetSource,
    ///The text of the tweet. For "extended" tweets, opening reply mentions and/or attached media
    ///or quoted tweet links do not count against character count, so this could be longer than 140
    ///characters in those situations.
    pub text: String,
    ///Indicates whether this tweet is a truncated "compatibility" form of an extended tweet whose
    ///full text is longer than 140 characters.
    pub truncated: bool,
    ///The user who posted this tweet. This field will be absent on tweets included as part of a
    ///`TwitterUser`.
    pub user: Option<Box<user::TwitterUser>>,
    ///If present and `true`, indicates that this tweet has been withheld due to a DMCA complaint.
    pub withheld_copyright: bool,
    ///If present, contains two-letter country codes indicating where this tweet is being withheld.
    ///
    ///The following special codes exist:
    ///
    ///- `XX`: Withheld in all countries
    ///- `XY`: Withheld due to DMCA complaint.
    pub withheld_in_countries: Option<Vec<String>>,
    ///If present, indicates whether the content being withheld is the `status` or the `user`.
    pub withheld_scope: Option<String>,
}

impl<'de> Deserialize<'de> for Tweet {
    fn deserialize<D>(deser: D) -> Result<Tweet, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut raw = raw::RawTweet::deserialize(deser)?;
        let text = raw
            .full_text
            .or(raw.extended_tweet.map(|xt| xt.full_text))
            .or(raw.text)
            .ok_or_else(|| D::Error::custom("Tweet missing text field"))?;
        let current_user_retweet = raw.current_user_retweet.map(|cur| cur.id);

        if let Some(ref mut range) = raw.display_text_range {
            codepoints_to_bytes(range, &text);
        }
        for entity in &mut raw.entities.hashtags {
            codepoints_to_bytes(&mut entity.range, &text);
        }
        for entity in &mut raw.entities.symbols {
            codepoints_to_bytes(&mut entity.range, &text);
        }
        for entity in &mut raw.entities.urls {
            codepoints_to_bytes(&mut entity.range, &text);
        }
        for entity in &mut raw.entities.user_mentions {
            codepoints_to_bytes(&mut entity.range, &text);
        }
        if let Some(ref mut media) = raw.entities.media {
            for entity in media.iter_mut() {
                codepoints_to_bytes(&mut entity.range, &text);
            }
        }
        if let Some(ref mut entities) = raw.extended_entities {
            for entity in entities.media.iter_mut() {
                codepoints_to_bytes(&mut entity.range, &text);
            }
        }

        Ok(Tweet {
            coordinates: raw.coordinates.map(|coords| coords.coordinates),
            created_at: raw.created_at,
            display_text_range: raw.display_text_range,
            entities: raw.entities,
            extended_entities: raw.extended_entities,
            favorite_count: raw.favorite_count,
            favorited: raw.favorited,
            filter_level: raw.filter_level,
            id: raw.id,
            in_reply_to_user_id: raw.in_reply_to_user_id,
            in_reply_to_screen_name: raw.in_reply_to_screen_name,
            in_reply_to_status_id: raw.in_reply_to_status_id,
            lang: raw.lang,
            place: raw.place,
            possibly_sensitive: raw.possibly_sensitive,
            quoted_status_id: raw.quoted_status_id,
            quoted_status: raw.quoted_status,
            retweet_count: raw.retweet_count,
            retweeted: raw.retweeted,
            retweeted_status: raw.retweeted_status,
            source: raw.source,
            truncated: raw.truncated,
            user: raw.user,
            withheld_copyright: raw.withheld_copyright,
            withheld_in_countries: raw.withheld_in_countries,
            withheld_scope: raw.withheld_scope,
            text,
            current_user_retweet,
        })
    }
}

///Represents the app from which a specific tweet was posted.
///
///This struct is parsed out of the HTML anchor tag that Twitter returns as part of each tweet.
///This way you can format the source link however you like without having to parse the values out
///yourself.
///
///Note that if you're going to reconstruct a link from this, the source URL has `rel="nofollow"`
///in the anchor tag.
#[derive(Debug, Clone, Deserialize)]
pub struct TweetSource {
    ///The name of the app, given by its developer.
    pub name: String,
    ///The URL for the app, given by its developer.
    pub url: String,
}

impl FromStr for TweetSource {
    type Err = error::Error;

    fn from_str(full: &str) -> Result<TweetSource, error::Error> {
        if full == "web" {
            return Ok(TweetSource {
                name: "Twitter Web Client".to_string(),
                url: "https://twitter.com".to_string(),
            });
        }

        lazy_static! {
            static ref RE_URL: Regex = Regex::new("href=\"(.*?)\"").unwrap();
            static ref RE_NAME: Regex = Regex::new(">(.*)</a>").unwrap();
        }

        let url = if let Some(cap) = RE_URL.captures(full) {
            let mut buf = String::new();
            cap.expand("$1", &mut buf);
            buf
        } else {
            return Err(InvalidResponse(
                "TweetSource had no link href",
                Some(full.to_string()),
            ));
        };

        let name = if let Some(cap) = RE_NAME.captures(full) {
            let mut buf = String::new();
            cap.expand("$1", &mut buf);
            buf
        } else {
            return Err(InvalidResponse(
                "TweetSource had no link text",
                Some(full.to_string()),
            ));
        };

        Ok(TweetSource {
            name: name,
            url: url,
        })
    }
}

fn deserialize_tweet_source<'de, D>(ser: D) -> Result<TweetSource, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(ser)?;
    Ok(TweetSource::from_str(&s).map_err(|e| D::Error::custom(e))?)
}

///Container for URL, hashtag, mention, and media information associated with a tweet.
///
///If a tweet has no hashtags, financial symbols ("cashtags"), links, or mentions, those respective
///Vecs will be empty. If there is no media attached to the tweet, that field will be `None`.
///
///Note that for media attached to a tweet, this struct will only contain the first image of a
///photo set, or a thumbnail of a video or GIF. Full media information is available in the tweet's
///`extended_entities` field.
#[derive(Debug, Clone, Deserialize)]
pub struct TweetEntities {
    ///Collection of hashtags parsed from the tweet.
    pub hashtags: Vec<entities::HashtagEntity>,
    ///Collection of financial symbols, or "cashtags", parsed from the tweet.
    pub symbols: Vec<entities::HashtagEntity>,
    ///Collection of URLs parsed from the tweet.
    pub urls: Vec<entities::UrlEntity>,
    ///Collection of user mentions parsed from the tweet.
    pub user_mentions: Vec<entities::MentionEntity>,
    ///If the tweet contains any attached media, this contains a collection of media information
    ///from the tweet.
    pub media: Option<Vec<entities::MediaEntity>>,
}

///Container for extended media information for a tweet.
///
///If a tweet has a photo, set of photos, gif, or video attached to it, this field will be present
///and contain the real media information. The information available in the `media` field of
///`entities` will only contain the first photo of a set, or a thumbnail of a gif or video.
#[derive(Debug, Clone, Deserialize)]
pub struct ExtendedTweetEntities {
    ///Collection of extended media information attached to the tweet.
    pub media: Vec<entities::MediaEntity>,
}

/// Helper struct to navigate collections of tweets by requesting tweets older or newer than certain
/// IDs.
///
/// Using a Timeline to navigate collections of tweets (like a user's timeline, their list of likes,
/// etc) allows you to efficiently cursor through a collection and only load in tweets you need.
///
/// To begin, call a method that returns a `Timeline`, optionally set the page size, and call
/// `start` to load the first page of results:
///
/// ```rust,no_run
/// # extern crate egg_mode; extern crate tokio; extern crate futures;
/// # use egg_mode::Token;
/// use tokio::runtime::current_thread::block_on_all;
/// # fn main() {
/// # let token: Token = unimplemented!();
/// let timeline = egg_mode::tweet::home_timeline(&token).with_page_size(10);
///
/// let (timeline, feed) = block_on_all(timeline.start()).unwrap();
/// for tweet in &feed {
///     println!("<@{}> {}", tweet.user.as_ref().unwrap().screen_name, tweet.text);
/// }
/// # }
/// ```
///
/// If you need to load the next set of tweets, call `older`, which will automatically update the
/// tweet IDs it tracks:
///
/// ```rust,no_run
/// # extern crate egg_mode; extern crate tokio; extern crate futures;
/// # use egg_mode::Token;
/// use tokio::runtime::current_thread::block_on_all;
/// # fn main() {
/// # let token: Token = unimplemented!();
/// # let timeline = egg_mode::tweet::home_timeline(&token);
/// # let (timeline, _) = block_on_all(timeline.start()).unwrap();
/// let (timeline, feed) = block_on_all(timeline.older(None)).unwrap();
/// for tweet in &feed {
///     println!("<@{}> {}", tweet.user.as_ref().unwrap().screen_name, tweet.text);
/// }
/// # }
/// ```
///
/// ...and similarly for `newer`, which operates in a similar fashion.
///
/// If you want to start afresh and reload the newest set of tweets again, you can call `start`
/// again, which will clear the tracked tweet IDs before loading the newest set of tweets. However,
/// if you've been storing these tweets as you go, and already know the newest tweet ID you have on
/// hand, you can load only those tweets you need like this:
///
/// ```rust,no_run
/// # extern crate egg_mode; extern crate tokio; extern crate futures;
/// # use egg_mode::Token;
/// use tokio::runtime::current_thread::block_on_all;
/// # fn main() {
/// # let token: Token = unimplemented!();
/// let timeline = egg_mode::tweet::home_timeline(&token)
///                                .with_page_size(10);
///
/// let (timeline, _feed) = block_on_all(timeline.start()).unwrap();
///
/// //keep the max_id for later
/// let reload_id = timeline.max_id.unwrap();
///
/// //simulate scrolling down a little bit
/// let (timeline, _feed) = block_on_all(timeline.older(None)).unwrap();
/// let (mut timeline, _feed) = block_on_all(timeline.older(None)).unwrap();
///
/// //reload the timeline with only what's new
/// timeline.reset();
/// let (timeline, _new_posts) = block_on_all(timeline.older(Some(reload_id))).unwrap();
/// # }
/// ```
///
/// Here, the argument to `older` means "older than what I just returned, but newer than the given
/// ID". Since we cleared the tracked IDs with `reset`, that turns into "the newest tweets
/// available that were posted after the given ID". The earlier invocations of `older` with `None`
/// do not place a bound on the tweets it loads. `newer` operates in a similar fashion with its
/// argument, saying "newer than what I just returned, but not newer than this given ID". When
/// called like this, it's possible for these methods to return nothing, which will also clear the
/// `Timeline`'s tracked IDs.
///
/// If you want to manually pull tweets between certain IDs, the baseline `call` function can do
/// that for you. Keep in mind, though, that `call` doesn't update the `min_id` or `max_id` fields,
/// so you'll have to set those yourself if you want to follow up with `older` or `newer`.
pub struct Timeline<'a> {
    ///The URL to request tweets from.
    link: &'static str,
    ///The token to authorize requests with.
    token: auth::Token,
    ///Optional set of params to include prior to adding timeline navigation parameters.
    params_base: Option<ParamList<'a>>,
    ///The maximum number of tweets to return in a single call. Twitter doesn't guarantee returning
    ///exactly this number, as suspended or deleted content is removed after retrieving the initial
    ///collection of tweets.
    pub count: i32,
    ///The largest/most recent tweet ID returned in the last call to `start`, `older`, or `newer`.
    pub max_id: Option<u64>,
    ///The smallest/oldest tweet ID returned in the last call to `start`, `older`, or `newer`.
    pub min_id: Option<u64>,
}

impl<'a> Timeline<'a> {
    ///Clear the saved IDs on this timeline.
    pub fn reset(&mut self) {
        self.max_id = None;
        self.min_id = None;
    }

    ///Clear the saved IDs on this timeline, and return the most recent set of tweets.
    pub fn start(mut self) -> TimelineFuture<'a> {
        self.reset();

        self.older(None)
    }

    ///Return the set of tweets older than the last set pulled, optionally placing a minimum tweet
    ///ID to bound with.
    pub fn older(self, since_id: Option<u64>) -> TimelineFuture<'a> {
        let req = self.request(since_id, self.min_id.map(|id| id - 1));
        let loader = make_parsed_future(req);

        TimelineFuture {
            timeline: Some(self),
            loader: loader,
        }
    }

    ///Return the set of tweets newer than the last set pulled, optionall placing a maximum tweet
    ///ID to bound with.
    pub fn newer(self, max_id: Option<u64>) -> TimelineFuture<'a> {
        let req = self.request(self.max_id, max_id);
        let loader = make_parsed_future(req);

        TimelineFuture {
            timeline: Some(self),
            loader: loader,
        }
    }

    ///Return the set of tweets between the IDs given.
    ///
    ///Note that the range is not fully inclusive; the tweet ID given by `since_id` will not be
    ///returned, but the tweet ID in `max_id` will be returned.
    ///
    ///If the range of tweets given by the IDs would return more than `self.count`, the newest set
    ///of tweets will be returned.
    pub fn call(&self, since_id: Option<u64>, max_id: Option<u64>) -> FutureResponse<Vec<Tweet>> {
        make_parsed_future(self.request(since_id, max_id))
    }

    ///Helper function to construct a `Request` from the current state.
    fn request(&self, since_id: Option<u64>, max_id: Option<u64>) -> Request<Body> {
        let mut params = self.params_base.as_ref().cloned().unwrap_or_default();
        add_param(&mut params, "count", self.count.to_string());
        add_param(&mut params, "tweet_mode", "extended");
        add_param(&mut params, "include_ext_alt_text", "true");

        if let Some(id) = since_id {
            add_param(&mut params, "since_id", id.to_string());
        }

        if let Some(id) = max_id {
            add_param(&mut params, "max_id", id.to_string());
        }

        auth::get(self.link, &self.token, Some(&params))
    }

    ///Helper builder function to set the page size.
    pub fn with_page_size(self, page_size: i32) -> Self {
        Timeline {
            count: page_size,
            ..self
        }
    }

    ///With the returned slice of Tweets, set the min_id and max_id on self.
    fn map_ids(&mut self, resp: &[Tweet]) {
        self.max_id = resp.first().map(|status| status.id);
        self.min_id = resp.last().map(|status| status.id);
    }

    ///Create an instance of `Timeline` with the given link and tokens.
    #[doc(hidden)]
    pub fn new(
        link: &'static str,
        params_base: Option<ParamList<'a>>,
        token: &auth::Token,
    ) -> Self {
        Timeline {
            link: link,
            token: token.clone(),
            params_base: params_base,
            count: 20,
            max_id: None,
            min_id: None,
        }
    }
}

/// `Future` which represents loading from a `Timeline`.
///
/// When this future completes, it will either return the tweets given by Twitter (after having
/// updated the IDs in the parent `Timeline`) or the error encountered when loading or parsing the
/// response.
#[must_use = "futures do nothing unless polled"]
pub struct TimelineFuture<'timeline> {
    timeline: Option<Timeline<'timeline>>,
    loader: FutureResponse<Vec<Tweet>>,
}

impl<'timeline> Future for TimelineFuture<'timeline> {
    type Item = (Timeline<'timeline>, Response<Vec<Tweet>>);
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.loader.poll() {
            Err(e) => Err(e),
            Ok(Async::NotReady) => Ok(Async::NotReady),
            Ok(Async::Ready(resp)) => {
                if let Some(mut timeline) = self.timeline.take() {
                    timeline.map_ids(&resp.response);
                    Ok(Async::Ready((timeline, resp)))
                } else {
                    Err(error::Error::FutureAlreadyCompleted)
                }
            }
        }
    }
}

/// Represents an in-progress tweet before it is sent.
///
/// This is your entry point to posting new tweets to Twitter. To begin, make a new `DraftTweet` by
/// calling `new` with your desired status text:
///
/// ```rust,no_run
/// use egg_mode::tweet::DraftTweet;
///
/// let draft = DraftTweet::new("This is an example status!");
/// ```
///
/// As-is, the draft won't do anything until you call `send` to post it:
///
/// ```rust,no_run
/// # extern crate egg_mode; extern crate tokio; extern crate futures;
/// # use egg_mode::Token;
/// use tokio::runtime::current_thread::block_on_all;
/// # fn main() {
/// # let token: Token = unimplemented!();
/// # use egg_mode::tweet::DraftTweet;
/// # let draft = DraftTweet::new("This is an example status!");
///
/// block_on_all(draft.send(&token)).unwrap();
/// # }
/// ```
///
/// Right now, the options for adding metadata to a post are pretty sparse. See the adaptor
/// functions below to see what metadata can be set. For example, you can use `in_reply_to` to
/// create a reply-chain like this:
///
/// ```rust,no_run
/// # extern crate egg_mode; extern crate tokio; extern crate futures;
/// # use egg_mode::Token;
/// use tokio::runtime::current_thread::block_on_all;
/// # fn main() {
/// # let token: Token = unimplemented!();
/// use egg_mode::tweet::DraftTweet;
///
/// let draft = DraftTweet::new("I'd like to start a thread here.");
/// let tweet = block_on_all(draft.send(&token)).unwrap();
///
/// let draft = DraftTweet::new("You see, I have a lot of things to say.")
///                        .in_reply_to(tweet.id);
/// let tweet = block_on_all(draft.send(&token)).unwrap();
///
/// let draft = DraftTweet::new("Thank you for your time.")
///                        .in_reply_to(tweet.id);
/// let tweet = block_on_all(draft.send(&token)).unwrap();
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct DraftTweet<'a> {
    ///The text of the draft tweet.
    pub text: Cow<'a, str>,
    ///If present, the ID of the tweet this draft is replying to.
    pub in_reply_to: Option<u64>,
    ///If present, whether to automatically fill reply mentions from the metadata of the
    ///`in_reply_to` tweet.
    pub auto_populate_reply_metadata: Option<bool>,
    ///If present, the list of user IDs to exclude from the automatically-populated metadata pulled
    ///when `auto_populate_reply_metadata` is true.
    pub exclude_reply_user_ids: Option<Cow<'a, [u64]>>,
    ///If present, the tweet link to quote or a [DM deep link][] to include in the tweet's
    ///attachment metadata.
    ///
    ///Note that if this link is not a tweet link or a [DM deep link][], Twitter will return an
    ///error when the draft is sent.
    ///
    ///[DM deep link]: https://business.twitter.com/en/help/campaign-editing-and-optimization/public-to-private-conversation.html
    pub attachment_url: Option<Cow<'a, str>>,
    ///If present, the latitude/longitude coordinates to attach to the draft.
    pub coordinates: Option<(f64, f64)>,
    ///If present (and if `coordinates` is present), indicates whether to display a pin on the
    ///exact coordinate when the eventual tweet is displayed.
    pub display_coordinates: Option<bool>,
    ///If present the Place to attach to this draft.
    pub place_id: Option<Cow<'a, str>>,
    ///List of media entities associated with tweet.
    ///
    ///A tweet can have one video, one GIF, or up to four images attached to it. When attaching
    ///them to a tweet, they're represented by a media ID, given through the upload process. (See
    ///[the `media` module] for more information on how to upload media.)
    ///
    ///[the `media` module]: ../media/index.html
    ///
    ///`DraftTweet` treats zeros in this array as if the media were not present.
    pub media_ids: [u64; 4],
    ///States whether the media attached with `media_ids` should be labeled as "possibly
    ///sensitive", to mask the media by default.
    pub possibly_sensitive: Option<bool>,
}

impl<'a> DraftTweet<'a> {
    ///Creates a new `DraftTweet` with the given status text.
    pub fn new<S: Into<Cow<'a, str>>>(text: S) -> Self {
        DraftTweet {
            text: text.into(),
            in_reply_to: None,
            auto_populate_reply_metadata: None,
            exclude_reply_user_ids: None,
            attachment_url: None,
            coordinates: None,
            display_coordinates: None,
            place_id: None,
            media_ids: [0; 4],
            possibly_sensitive: None,
        }
    }

    ///Marks this draft tweet as replying to the given status ID.
    ///
    ///Note that this will only properly take effect if the user who posted the given status is
    ///@mentioned in the status text, or if the given status was posted by the authenticated user.
    pub fn in_reply_to(self, in_reply_to: u64) -> Self {
        DraftTweet {
            in_reply_to: Some(in_reply_to),
            ..self
        }
    }

    ///Tells Twitter whether or not to automatically fill reply mentions from the tweet linked in
    ///`in_reply_to`.
    ///
    ///This parameter will have no effect if `in_reply_to` is absent.
    ///
    ///If you set this to true, you can strip out the mentions from the beginning of the tweet text
    ///if they were also in the reply mentions of the parent tweet. To remove handles from the list
    ///of reply mentions, hand their user IDs to `exclude_reply_user_ids`.
    pub fn auto_populate_reply_metadata(self, auto_populate: bool) -> Self {
        DraftTweet {
            auto_populate_reply_metadata: Some(auto_populate),
            ..self
        }
    }

    ///Tells Twitter to exclude the given list of user IDs from the automatically-populated reply
    ///mentions.
    ///
    ///This parameter will have no effect if `auto_populate_reply_metadata` is absent or false.
    ///
    ///Note that you cannot use this parameter to remove the author of the parent tweet from the
    ///reply list. Twitter will silently ignore the author's ID in that scenario.
    pub fn exclude_reply_user_ids<V: Into<Cow<'a, [u64]>>>(self, user_ids: V) -> Self {
        DraftTweet {
            exclude_reply_user_ids: Some(user_ids.into()),
            ..self
        }
    }

    ///Attaches the given tweet URL or [DM deep link][] to the tweet draft, which lets it be used
    ///outside the 140 character text limit.
    ///
    ///Note that if this link is not a tweet URL or a DM deep link, then Twitter will return an
    ///error when this draft is sent.
    ///
    ///[DM deep link]: https://business.twitter.com/en/help/campaign-editing-and-optimization/public-to-private-conversation.html
    pub fn attachment_url<S: Into<Cow<'a, str>>>(self, url: S) -> Self {
        DraftTweet {
            attachment_url: Some(url.into()),
            ..self
        }
    }

    ///Attach a lat/lon coordinate to this tweet, and mark whether a pin should be placed on the
    ///exact coordinate when the tweet is displayed.
    ///
    ///If coordinates are given through this method and no `place_id` is attached, Twitter will
    ///effectively call `place::reverse_geocode` with the given coordinate and attach that Place to
    ///the eventual tweet.
    ///
    ///Location fields will be ignored unless the user has enabled geolocation from their profile.
    pub fn coordinates(self, latitude: f64, longitude: f64, display: bool) -> Self {
        DraftTweet {
            coordinates: Some((latitude, longitude)),
            display_coordinates: Some(display),
            ..self
        }
    }

    ///Attach a Place to this tweet. This field will take precedence over `coordinates` in terms of
    ///what location is displayed with the tweet.
    ///
    ///Location fields will be ignored unless the user has enabled geolocation from their profile.
    pub fn place_id<S: Into<Cow<'a, str>>>(self, place_id: S) -> Self {
        DraftTweet {
            place_id: Some(place_id.into()),
            ..self
        }
    }

    ///Attaches the given media ID(s) to this tweet. If more than four IDs are in this slice, only
    ///the first four will be attached. Note that Twitter will only allow one GIF, one video, or up
    ///to four images to be attached to a single tweet.
    ///
    ///Note that if this is called multiple times, only the last set of IDs will be kept.
    pub fn media_ids(self, media_ids: &[u64]) -> Self {
        DraftTweet {
            media_ids: {
                let mut ret = [0; 4];
                let len = ::std::cmp::min(media_ids.len(), 4);
                ret[..len].copy_from_slice(&media_ids[..len]);
                ret
            },
            ..self
        }
    }

    ///Marks the media attached with `media_ids` as being sensitive, so it can be hidden by
    ///default.
    pub fn possibly_sensitive(self, sensitive: bool) -> Self {
        DraftTweet {
            possibly_sensitive: Some(sensitive),
            ..self
        }
    }

    ///Send the assembled tweet as the authenticated user.
    pub fn send(&self, token: &auth::Token) -> FutureResponse<Tweet> {
        let mut params = HashMap::new();
        add_param(&mut params, "status", self.text.clone());

        if let Some(reply) = self.in_reply_to {
            add_param(&mut params, "in_reply_to_status_id", reply.to_string());
        }

        if let Some(auto_populate) = self.auto_populate_reply_metadata {
            add_param(
                &mut params,
                "auto_populate_reply_metadata",
                auto_populate.to_string(),
            );
        }

        if let Some(ref exclude) = self.exclude_reply_user_ids {
            let list = exclude
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(",");
            add_param(&mut params, "exclude_reply_user_ids", list);
        }

        if let Some(ref url) = self.attachment_url {
            add_param(&mut params, "attachment_url", url.clone());
        }

        if let Some((lat, long)) = self.coordinates {
            add_param(&mut params, "lat", lat.to_string());
            add_param(&mut params, "long", long.to_string());
        }

        if let Some(display) = self.display_coordinates {
            add_param(&mut params, "display_coordinates", display.to_string());
        }

        if let Some(ref place_id) = self.place_id {
            add_param(&mut params, "place_id", place_id.clone());
        }

        let media = self
            .media_ids
            .iter()
            .filter(|&&id| id != 0)
            .map(|id| id.to_string())
            .collect::<Vec<String>>()
            .join(",");
        if !media.is_empty() {
            add_param(&mut params, "media_ids", media);
        }

        if let Some(sensitive) = self.possibly_sensitive {
            add_param(&mut params, "possibly_sensitive", sensitive.to_string());
        }

        let req = auth::post(links::statuses::UPDATE, token, Some(&params));
        make_parsed_future(req)
    }
}

#[cfg(test)]
mod tests {
    use super::Tweet;
    use common::tests::load_file;

    use chrono::{Datelike, Timelike, Weekday};

    fn load_tweet(path: &str) -> Tweet {
        let sample = load_file(path);
        ::serde_json::from_str(&sample).unwrap()
    }

    #[test]
    fn parse_basic() {
        let sample = load_tweet("sample_payloads/sample-extended-onepic.json");

        assert_eq!(sample.text,
                   ".@Serrayak said he’d use what-ev-er I came up with as his Halloween avatar so I’m just making sure you all know he said that https://t.co/MvgxCwDwSa");
        assert!(sample.user.is_some());
        assert_eq!(sample.user.unwrap().screen_name, "0xabad1dea");
        assert_eq!(sample.id, 782349500404862976);
        assert_eq!(sample.source.name, "Tweetbot for iΟS"); //note that's an omicron, not an O
        assert_eq!(sample.source.url, "http://tapbots.com/tweetbot");
        assert_eq!(sample.created_at.weekday(), Weekday::Sat);
        assert_eq!(sample.created_at.year(), 2016);
        assert_eq!(sample.created_at.month(), 10);
        assert_eq!(sample.created_at.day(), 1);
        assert_eq!(sample.created_at.hour(), 22);
        assert_eq!(sample.created_at.minute(), 40);
        assert_eq!(sample.created_at.second(), 30);
        assert_eq!(sample.favorite_count, 20);
        assert_eq!(sample.retweet_count, 0);
        assert_eq!(sample.lang, Some("en".into()));
        assert_eq!(sample.coordinates, None);
        assert!(sample.place.is_none());

        assert_eq!(sample.favorited, Some(false));
        assert_eq!(sample.retweeted, Some(false));
        assert!(sample.current_user_retweet.is_none());

        assert!(sample
            .entities
            .user_mentions
            .iter()
            .any(|m| m.screen_name == "Serrayak"));
        assert!(sample.extended_entities.is_some());
        assert_eq!(sample.extended_entities.unwrap().media.len(), 1);

        //text contains extended link, which is outside of display_text_range
        let range = sample.display_text_range.unwrap();
        assert_eq!(&sample.text[range.0..range.1],
                   ".@Serrayak said he’d use what-ev-er I came up with as his Halloween avatar so I’m just making sure you all know he said that"
        );
        assert_eq!(sample.truncated, false);
    }

    #[test]
    fn parse_samples() {
        // Just check we can parse them without error, taken from
        // https://github.com/twitterdev/tweet-updates/tree/686982b586dcc87d669151e89532ffea7e29e0d8/samples/initial
        load_tweet("sample_payloads/compatibilityplus_classic_13994.json");
        load_tweet("sample_payloads/compatibilityplus_classic_hidden_13797.json");
        load_tweet("sample_payloads/compatibilityplus_extended_13997.json");
        load_tweet("sample_payloads/extended_classic_14002.json");
        load_tweet("sample_payloads/extended_classic_hidden_13761.json");
        load_tweet("sample_payloads/extended_extended_14001.json");
        load_tweet("sample_payloads/nullable_user_mention.json");
    }

    #[test]
    fn parse_reply() {
        let sample = load_tweet("sample_payloads/sample-reply.json");

        assert_eq!(
            sample.in_reply_to_screen_name,
            Some("QuietMisdreavus".to_string())
        );
        assert_eq!(sample.in_reply_to_user_id, Some(2977334326));
        assert_eq!(sample.in_reply_to_status_id, Some(782643731665080322));
    }

    #[test]
    fn parse_quote() {
        let sample = load_tweet("sample_payloads/sample-quote.json");

        assert_eq!(sample.quoted_status_id, Some(783004145485840384));
        assert!(sample.quoted_status.is_some());
        assert_eq!(sample.quoted_status.unwrap().text,
                   "@chalkboardsband hot damn i should call up my friends in austin, i might actually be able to make one of these now :D");
    }

    #[test]
    fn parse_retweet() {
        let sample = load_tweet("sample_payloads/sample-retweet.json");

        assert!(sample.retweeted_status.is_some());
        assert_eq!(sample.retweeted_status.unwrap().text,
                   "it's working: follow @andrewhuangbot for a random lyric of mine every hour. we'll call this version 0.1.0. wanna get line breaks in there");
    }

    #[test]
    fn parse_image_alt_text() {
        let sample = load_tweet("sample_payloads/sample-image-alt-text.json");
        let extended_entities = sample.extended_entities.unwrap();

        assert_eq!(
            extended_entities.media[0].ext_alt_text,
            Some("test alt text for the image".to_string())
        );
    }

}
