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

use std::collections::HashMap;

use rustc_serialize::json;

use auth;
use links;
use user;
use error;
use error::Error::InvalidResponse;
use entities;
use place;
use common::*;

mod fun;

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
#[derive(Debug)]
pub struct Tweet {
    //If the user has contributors enabled, this will show which accounts contributed to this
    //tweet.
    //pub contributors: Option<Contributors>,
    ///If present, the location coordinate attached to the tweet, as a (latitude, longitude) pair.
    pub coordinates: Option<(f64, f64)>,
    ///UTC timestamp showing when the tweet was posted, formatted like "Wed Aug 27 13:08:45 +0000
    ///2008".
    pub created_at: String,
    ///If the authenticated user has retweeted this tweet, contains the ID of the retweet.
    pub current_user_retweet: Option<i64>,
    ///If this tweet is an extended tweet with "hidden" metadata and entities, contains the code
    ///point indices where the "displayable" tweet text is.
    pub display_text_range: Option<(i32, i32)>,
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
    //Indicates the maximum `FilterLevel` parameter that can be applied to a stream and still show
    //this tweet.
    //pub filter_level: FilterLevel,
    ///Numeric ID for this tweet.
    pub id: i64,
    ///If the tweet is a reply, contains the ID of the user that was replied to.
    pub in_reply_to_user_id: Option<i64>,
    ///If the tweet is a reply, contains the screen name of the user that was replied to.
    pub in_reply_to_screen_name: Option<String>,
    ///If the tweet is a reply, contains the ID of the tweet that was replied to.
    pub in_reply_to_status_id: Option<i64>,
    ///Can contain a language ID indicating the machine-detected language of the text, or "und" if
    ///no language could be detected.
    pub lang: String,
    ///When present, the `Place` that this tweet is associated with (but not necessarily where it
    ///originated from).
    pub place: Option<place::Place>,
    ///If the tweet has a link, indicates whether the link may contain content that could be
    ///identified as sensitive.
    pub possibly_sensitive: Option<bool>,
    ///If this tweet is quoting another by link, contains the ID of the quoted tweet.
    pub quoted_status_id: Option<i64>,
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
    ///The application used to post the tweet, as an HTML anchor tag containing the app's URL and
    ///name.
    pub source: String, //TODO: this is html, i want to parse this eventually
    ///The text of the tweet. For "extended" tweets, opening reply mentions and/or attached media
    ///or quoted tweet links do not count against character count, so this could be longer than 140
    ///characters in those situations.
    pub text: String,
    ///Indicates whether this tweet is a truncated "compatibility" form of an extended tweet whose
    ///full text is longer than 140 characters.
    pub truncated: bool,
    ///The user who posted this tweet.
    pub user: Box<user::TwitterUser>,
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

impl FromJson for Tweet {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse("Tweet received json that wasn't an object", Some(input.to_string())));
        }

        //TODO: when i start building streams, i want to extract "extended_tweet" and use its
        //fields here

        let coords = field(input, "coordinates").ok();

        Ok(Tweet {
            //contributors: Option<Contributors>,
            coordinates: coords.map(|(lon, lat)| (lat, lon)),
            created_at: try!(field(input, "created_at")),
            current_user_retweet: try!(current_user_retweet(input, "current_user_retweet")),
            display_text_range: field(input, "display_text_range").ok(),
            entities: try!(field(input, "entities")),
            extended_entities: field(input, "extended_entities").ok(),
            favorite_count: field(input, "favorite_count").unwrap_or(0),
            favorited: field(input, "favorited").ok(),
            //filter_level: FilterLevel,
            id: try!(field(input, "id")),
            in_reply_to_user_id: field(input, "in_reply_to_user_id").ok(),
            in_reply_to_screen_name: field(input, "in_reply_to_screen_name").ok(),
            in_reply_to_status_id: field(input, "in_reply_to_status_id").ok(),
            lang: try!(field(input, "lang")),
            place: field(input, "place").ok(),
            possibly_sensitive: field(input, "possibly_sensitive").ok(),
            quoted_status_id: field(input, "quoted_status_id").ok(),
            quoted_status: field(input, "quoted_status").map(Box::new).ok(),
            //scopes: Option<Scopes>,
            retweet_count: try!(field(input, "retweet_count")),
            retweeted: field(input, "retweeted").ok(),
            retweeted_status: field(input, "retweeted_status").map(Box::new).ok(),
            source: try!(field(input, "source")),
            text: try!(field(input, "full_text").or(field(input, "text"))),
            truncated: try!(field(input, "truncated")),
            user: try!(field(input, "user").map(Box::new)),
            withheld_copyright: field(input, "withheld_copyright").unwrap_or(false),
            withheld_in_countries: field(input, "withheld_in_countries").ok(),
            withheld_scope: field(input, "withheld_scope").ok(),
        })
    }
}

fn current_user_retweet(input: &json::Json, field: &'static str) -> Result<Option<i64>, error::Error> {
    if let Some(obj) = input.find(field).and_then(|f| f.as_object()) {
        match obj.get("id").and_then(|o| o.as_i64()) {
            Some(id) => Ok(Some(id)),
            None => Err(InvalidResponse("invalid structure inside current_user_retweet", None)),
        }
    }
    else {
        Ok(None)
    }
}

///Container for URL, hashtag, mention, and media information associated with a tweet.
///
///If a tweet has no hashtags, financial symbols ("cashtags"), links, or mentions, those respective
///Vecs will be empty. If there is no media attached to the tweet, that field will be `None`.
///
///Note that for media attached to a tweet, this struct will only contain the first image of a
///photo set, or a thumbnail of a video or GIF. Full media information is available in the tweet's
///`extended_entities` field.
#[derive(Debug)]
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

impl FromJson for TweetEntities {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse("TweetEntities received json that wasn't an object", Some(input.to_string())));
        }

        Ok(TweetEntities {
            hashtags: try!(field(input, "hashtags")),
            symbols: try!(field(input, "symbols")),
            urls: try!(field(input, "urls")),
            user_mentions: try!(field(input, "user_mentions")),
            media: field(input, "media").ok(),
        })
    }
}

///Container for extended media information for a tweet.
///
///If a tweet has a photo, set of photos, gif, or video attached to it, this field will be present
///and contain the real media information. The information available in the `media` field of
///`entities` will only contain the first photo of a set, or a thumbnail of a gif or video.
#[derive(Debug)]
pub struct ExtendedTweetEntities {
    ///Collection of extended media information attached to the tweet.
    pub media: Vec<entities::MediaEntity>,
}

impl FromJson for ExtendedTweetEntities {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse("ExtendedTweetEntities received json that wasn't an object", Some(input.to_string())));
        }

        Ok(ExtendedTweetEntities {
            media: try!(field(input, "media")),
        })
    }
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
/// # let con_token = egg_mode::Token::new("", "");
/// # let access_token = egg_mode::Token::new("", "");
/// let mut timeline = egg_mode::tweet::home_timeline(&con_token, &access_token)
///                                .with_page_size(10);
///
/// for tweet in &timeline.start().unwrap().response {
///     println!("<@{}> {}", tweet.user.screen_name, tweet.text);
/// }
/// ```
///
/// If you need to load the next set of tweets, call `older`, which will automatically update the
/// tweet IDs it tracks:
///
/// ```rust,no_run
/// # let con_token = egg_mode::Token::new("", "");
/// # let access_token = egg_mode::Token::new("", "");
/// # let mut timeline = egg_mode::tweet::home_timeline(&con_token, &access_token);
/// # timeline.start().unwrap();
/// for tweet in &timeline.older(None).unwrap().response {
///     println!("<@{}> {}", tweet.user.screen_name, tweet.text);
/// }
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
/// # let con_token = egg_mode::Token::new("", "");
/// # let access_token = egg_mode::Token::new("", "");
/// let mut timeline = egg_mode::tweet::home_timeline(&con_token, &access_token)
///                                .with_page_size(10);
///
/// timeline.start().unwrap();
///
/// //keep the max_id for later
/// let reload_id = timeline.max_id.unwrap();
///
/// //simulate scrolling down a little bit
/// timeline.older(None).unwrap();
/// timeline.older(None).unwrap();
///
/// //reload the timeline with only what's new
/// timeline.reset();
/// timeline.older(Some(reload_id)).unwrap();
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
    ///The consumer token to authenticate requests with.
    con_token: &'a auth::Token<'a>,
    ///The access token to authenticate requests with.
    access_token: &'a auth::Token<'a>,
    ///Optional set of params to include prior to adding lifetime navigation parameters.
    params_base: Option<ParamList<'a>>,
    ///The maximum number of tweets to return in a single call. Twitter doesn't guarantee returning
    ///exactly this number, as suspended or deleted content is removed after retrieving the initial
    ///collection of tweets.
    pub count: i32,
    ///The largest/most recent tweet ID returned in the last call to `start`, `older`, or `newer`.
    pub max_id: Option<i64>,
    ///The smallest/oldest tweet ID returned in the last call to `start`, `older`, or `newer`.
    pub min_id: Option<i64>,
}

impl<'a> Timeline<'a> {
    ///Clear the saved IDs on this timeline.
    pub fn reset(&mut self) {
        self.max_id = None;
        self.min_id = None;
    }

    ///Clear the saved IDs on this timeline, and return the most recent set of tweets.
    pub fn start(&mut self) -> WebResponse<Vec<Tweet>> {
        self.reset();

        self.older(None)
    }

    ///Return the set of tweets older than the last set pulled, optionally placing a minimum tweet
    ///ID to bound with.
    pub fn older(&mut self, since_id: Option<i64>) -> WebResponse<Vec<Tweet>> {
        let resp = try!(self.call(since_id, self.min_id.map(|id| id - 1)));

        self.map_ids(&resp.response);

        Ok(resp)
    }

    ///Return the set of tweets newer than the last set pulled, optionall placing a maximum tweet
    ///ID to bound with.
    pub fn newer(&mut self, max_id: Option<i64>) -> WebResponse<Vec<Tweet>> {
        let resp = try!(self.call(self.max_id, max_id));

        self.map_ids(&resp.response);

        Ok(resp)
    }

    ///Return the set of tweets between the IDs given.
    ///
    ///Note that the range is not fully inclusive; the tweet ID given by `since_id` will not be
    ///returned, but the tweet ID in `max_id` will be returned.
    ///
    ///If the range of tweets given by the IDs would return more than `self.count`, the newest set
    ///of tweets will be returned.
    pub fn call(&self, since_id: Option<i64>, max_id: Option<i64>) -> WebResponse<Vec<Tweet>> {
        let mut params = self.params_base.as_ref().cloned().unwrap_or_default();
        add_param(&mut params, "count", self.count.to_string());
        add_param(&mut params, "tweet_mode", "extended");

        if let Some(id) = since_id {
            add_param(&mut params, "since_id", id.to_string());
        }

        if let Some(id) = max_id {
            add_param(&mut params, "max_id", id.to_string());
        }

        let mut resp = try!(auth::get(self.link, self.con_token, self.access_token, Some(&params)));

        parse_response(&mut resp)
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
    fn new(link: &'static str, params_base: Option<ParamList<'a>>,
               con_token: &'a auth::Token, access_token: &'a auth::Token) -> Self {
        Timeline {
            link: link,
            con_token: con_token,
            access_token: access_token,
            params_base: params_base,
            count: 20,
            max_id: None,
            min_id: None,
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
/// # let con_token = egg_mode::Token::new("", "");
/// # let access_token = egg_mode::Token::new("", "");
/// # use egg_mode::tweet::DraftTweet;
/// # let draft = DraftTweet::new("This is an example status!");
/// draft.send(&con_token, &access_token).unwrap();
/// ```
///
/// Right now, the options for adding metadata to a post are pretty sparse. See the adaptor
/// functions below to see what metadata can be set. For example, you can use `in_reply_to` to
/// create a reply-chain like this:
///
/// ```rust,no_run
/// # let con_token = egg_mode::Token::new("", "");
/// # let access_token = egg_mode::Token::new("", "");
/// use egg_mode::tweet::DraftTweet;
///
/// let draft = DraftTweet::new("I'd like to start a thread here.");
/// let tweet = draft.send(&con_token, &access_token).unwrap();
///
/// let draft = DraftTweet::new("You see, I have a lot of things to say.")
///                        .in_reply_to(tweet.response.id);
/// let tweet = draft.send(&con_token, &access_token).unwrap();
///
/// let draft = DraftTweet::new("Thank you for your time.")
///                        .in_reply_to(tweet.response.id);
/// let tweet = draft.send(&con_token, &access_token).unwrap();
/// ```
#[derive(Debug)]
pub struct DraftTweet<'a> {
    ///The text of the draft tweet.
    pub text: &'a str,
    ///If present, the ID of the tweet this draft is replying to.
    pub in_reply_to: Option<i64>,
    ///If present, whether to automatically fill reply mentions from the metadata of the
    ///`in_reply_to` tweet.
    pub auto_populate_reply_metadata: Option<bool>,
    ///If present, the list of user IDs to exclude from the automatically-populated metadata pulled
    ///when `auto_populate_reply_metadata` is true.
    pub exclude_reply_user_ids: Option<&'a [i64]>,
    ///If present, the tweet link to quote or a [DM deep link][] to include in the tweet's
    ///attachment metadata.
    ///
    ///Note that if this link is not a tweet link or a [DM deep link][], Twitter will return an
    ///error when the draft is sent.
    ///
    ///[DM deep link]: https://business.twitter.com/en/help/campaign-editing-and-optimization/public-to-private-conversation.html
    pub attachment_url: Option<&'a str>,
    ///If present, the latitude/longitude coordinates to attach to the draft.
    pub coordinates: Option<(f64, f64)>,
    ///If present (and if `coordinates` is present), indicates whether to display a pin on the
    ///exact coordinate when the eventual tweet is displayed.
    pub display_coordinates: Option<bool>,
    ///If present the Place to attach to this draft.
    pub place_id: Option<&'a str>,
}

impl<'a> DraftTweet<'a> {
    ///Creates a new `DraftTweet` with the given status text.
    pub fn new(text: &'a str) -> Self {
        DraftTweet {
            text: text,
            in_reply_to: None,
            auto_populate_reply_metadata: None,
            exclude_reply_user_ids: None,
            attachment_url: None,
            coordinates: None,
            display_coordinates: None,
            place_id: None,
        }
    }

    ///Marks this draft tweet as replying to the given status ID.
    ///
    ///Note that this will only properly take effect if the user who posted the given status is
    ///@mentioned in the status text, or if the given status was posted by the authenticated user.
    pub fn in_reply_to(self, in_reply_to: i64) -> Self {
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
    pub fn exclude_reply_user_ids(self, user_ids: &'a [i64]) -> Self {
        DraftTweet {
            exclude_reply_user_ids: Some(user_ids),
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
    pub fn attachment_url(self, url: &'a str) -> Self {
        DraftTweet {
            attachment_url: Some(url),
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
    pub fn place_id(self, place_id: &'a str) -> Self {
        DraftTweet {
            place_id: Some(place_id),
            ..self
        }
    }

    ///Send the assembled tweet as the authenticated user.
    pub fn send(&self, con_token: &auth::Token, access_token: &auth::Token) -> WebResponse<Tweet> {
        let mut params = HashMap::new();
        add_param(&mut params, "status", self.text);

        if let Some(reply) = self.in_reply_to {
            add_param(&mut params, "in_reply_to_status_id", reply.to_string());
        }

        if let Some(auto_populate) = self.auto_populate_reply_metadata {
            add_param(&mut params, "auto_populate_reply_metadata", auto_populate.to_string());
        }

        if let Some(exclude) = self.exclude_reply_user_ids {
            let list = exclude.iter().map(|id| id.to_string()).collect::<Vec<_>>().join(",");
            add_param(&mut params, "exclude_reply_user_ids", list);
        }

        if let Some(url) = self.attachment_url {
            add_param(&mut params, "attachment_url", url);
        }

        if let Some((lat, long)) = self.coordinates {
            add_param(&mut params, "lat", lat.to_string());
            add_param(&mut params, "long", long.to_string());
        }

        if let Some(display) = self.display_coordinates {
            add_param(&mut params, "display_coordinates", display.to_string());
        }

        if let Some(place_id) = self.place_id {
            add_param(&mut params, "place_id", place_id);
        }

        let mut resp = try!(auth::post(links::statuses::UPDATE, con_token, access_token, Some(&params)));
        parse_response(&mut resp)
    }
}
