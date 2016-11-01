//! Structs and methods for working with direct messages.
//!
//! Note that direct message access requires a special permissions level above regular read/write
//! access. Your app must be configured to have "read, write, and direct message" access to use any
//! function in this module, even the read-only ones.
//!
//! Although the Twitter website and official apps display DMs as threads between the authenticated
//! user and specific other users, the API does not expose them like this. Separate calls to
//! `received` and `sent` are necessary to fully reconstruct a DM thread. You can partition on
//! `sender_id`/`receiver_id` and sort by their `created_at` to weave the streams together.
//!
//! ## Types
//!
//! * `DirectMessage`/`DMEntities`: A single DM and its associated entities. The `DMEntities`
//!   struct contains information about URLs, user mentions, and hashtags in the DM.
//! * `Timeline`: Effectively the same as `tweet::Timeline`, but returns `DirectMessage`s instead.
//!   Returned by functions that traverse collections of DMs.
//!
//! ## Functions
//!
//! ### Lookup
//!
//! These functions pull a user's DMs for viewing. `sent` and `received` can be cursored with
//! sub-views like with tweets, so they return a `Timeline` instance that can be navigated at will.
//!
//! * `sent`
//! * `received`
//! * `show`
//!
//! ### Actions
//!
//! These functions are your basic write access to DMs. As a DM does not carry as much metadata as
//! a tweet, the `send` action does not go through a builder struct like with `DraftTweet`.
//!
//! * `send`
//! * `delete`

use common::*;

use rustc_serialize::json;
use chrono;

use auth;
use user;
use entities;
use error;
use error::Error::InvalidResponse;

mod fun;

pub use self::fun::*;

///Represents a single direct message.
///
///As a DM has far less metadata than a regular tweet, the structure consequently contains far
///fewer fields. The basic fields are `id`, `text`, `entities`, and `created_at`; everything else
///either refers to the sender or receiver in some manner.
pub struct DirectMessage {
    ///Numeric ID for this DM.
    pub id: i64,
    ///UTC timestamp from when this DM was created.
    pub created_at: chrono::DateTime<chrono::UTC>,
    ///The text of the DM.
    pub text: String,
    ///Link, hashtag, and user mention information parsed out of the DM.
    pub entities: DMEntities,
    ///The screen name of the user who sent the DM.
    pub sender_screen_name: String,
    ///The ID of the user who sent the DM.
    pub sender_id: i64,
    ///Full information of the user who sent the DM.
    pub sender: Box<user::TwitterUser>,
    ///The screen name of the user who received the DM.
    pub recipient_screen_name: String,
    ///The ID of the user who received the DM.
    pub recipient_id: i64,
    ///Full information for the user who received the DM.
    pub recipient: Box<user::TwitterUser>,
}

///Container for URL, hashtag, mention, and media information associated with a direct message.
///
///As far as entities are concerned, a DM can contain nearly everything a tweet can. The only thing
///that isn't present here is the "extended media" that would be on the tweet's `extended_entities`
///field. A user can attach a single picture to a DM via the Twitter website or official apps, so
///if that is present, it will be available in `media`. (Note that the functionality to send
///pictures through a DM is unavailable on the public API; only viewing them is possible.)
///
///For all other fields, if the message contains no hashtags, financial symbols ("cashtags"),
///links, or mentions, those corresponding fields will still be present, just empty.
pub struct DMEntities {
    ///Collection of hashtags parsed from the DM.
    pub hashtags: Vec<entities::HashtagEntity>,
    ///Collection of financial symbols, or "cashtags", parsed from the DM.
    pub symbols: Vec<entities::HashtagEntity>,
    ///Collection of URLs parsed from the DM.
    pub urls: Vec<entities::UrlEntity>,
    ///Collection of user mentions parsed from the DM.
    pub user_mentions: Vec<entities::MentionEntity>,
    ///If the message contains any attached media, this contains a collection of media information
    ///from it.
    pub media: Option<Vec<entities::MediaEntity>>,
}

impl FromJson for DirectMessage {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse("DirectMessage received json that wasn't an object",
                                       Some(input.to_string())));
        }

        field_present!(input, id);
        field_present!(input, created_at);
        field_present!(input, text);
        field_present!(input, entities);
        field_present!(input, sender_screen_name);
        field_present!(input, sender_id);
        field_present!(input, sender);
        field_present!(input, recipient_screen_name);
        field_present!(input, recipient_id);
        field_present!(input, recipient);

        Ok(DirectMessage {
            id: try!(field(input, "id")),
            created_at: try!(field(input, "created_at")),
            text: try!(field(input, "text")),
            entities: try!(field(input, "entities")),
            sender_screen_name: try!(field(input, "sender_screen_name")),
            sender_id: try!(field(input, "sender_id")),
            sender: try!(field(input, "sender")),
            recipient_screen_name: try!(field(input, "recipient_screen_name")),
            recipient_id: try!(field(input, "recipient_id")),
            recipient: try!(field(input, "recipient")),
        })
    }
}

impl FromJson for DMEntities {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse("DMEntities received json that wasn't an object",
                                       Some(input.to_string())));
        }

        field_present!(input, hashtags);
        field_present!(input, symbols);
        field_present!(input, urls);
        field_present!(input, user_mentions);

        Ok(DMEntities {
            hashtags: try!(field(input, "hashtags")),
            symbols: try!(field(input, "symbols")),
            urls: try!(field(input, "urls")),
            user_mentions: try!(field(input, "user_mentions")),
            media: try!(field(input, "media")),
        })
    }
}

/// Helper struct to navigate collections of direct messages by requesting DMs older or newer than
/// certain IDs.
///
/// Using a Timeline to navigate collections of DMs allows you to efficiently cursor through a
/// collection and only load in the messages you need.
///
/// To begin, call a method that returns a `Timeline`, optionally set the page size, and call
/// `start` to load the first page of results:
///
/// ```rust,no_run
/// # let con_token = egg_mode::Token::new("", "");
/// # let access_token = egg_mode::Token::new("", "");
/// let mut timeline = egg_mode::direct::received(&con_token, &access_token)
///                                .with_page_size(10);
///
/// for dm in &timeline.start().unwrap().response {
///     println!("<@{}> {}", dm.sender_screen_name, dm.text);
/// }
/// ```
///
/// If you need to load the next set of messages, call `older`, which will automatically update the
/// IDs it tracks:
///
/// ```rust,no_run
/// # let con_token = egg_mode::Token::new("", "");
/// # let access_token = egg_mode::Token::new("", "");
/// # let mut timeline = egg_mode::direct::received(&con_token, &access_token);
/// # timeline.start().unwrap();
/// for dm in &timeline.older(None).unwrap().response {
///     println!("<@{}> {}", dm.sender_screen_name, dm.text);
/// }
/// ```
///
/// ...and similarly for `newer`, which operates in a similar fashion.
///
/// If you want to start afresh and reload the newest set of DMs again, you can call `start` again,
/// which will clear the tracked IDs before loading the newest set of messages. However, if you've
/// been storing these messages as you go, and already know the newest ID you have on hand, you can
/// load only those messages you need like this:
///
/// ```rust,no_run
/// # let con_token = egg_mode::Token::new("", "");
/// # let access_token = egg_mode::Token::new("", "");
/// let mut timeline = egg_mode::direct::received(&con_token, &access_token)
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
/// ID". Since we cleared the tracked IDs with `reset`, that turns into "the newest DMs available
/// that were sent after the given ID". The earlier invocations of `older` with `None` do not place
/// a bound on the DMs it loads. `newer` operates in a similar fashion with its argument, saying
/// "newer than what I just returned, but not newer than this given ID". When called like this,
/// it's possible for these methods to return nothing, which will also clear the `Timeline`'s
/// tracked IDs.
///
/// If you want to manually pull messages between certain IDs, the baseline `call` function can do
/// that for you. Keep in mind, though, that `call` doesn't update the `min_id` or `max_id` fields,
/// so you'll have to set those yourself if you want to follow up with `older` or `newer`.
pub struct Timeline<'a> {
    ///The URL to request DMs from.
    link: &'static str,
    ///The consumer token to authenticate requests with.
    con_token: &'a auth::Token<'a>,
    ///The access token to authenticate requests with.
    access_token: &'a auth::Token<'a>,
    ///Optional set of params to include prior to adding lifetime navigation parameters.
    params_base: Option<ParamList<'a>>,
    ///The maximum number of messages to return in a single call. Twitter doesn't guarantee
    ///returning exactly this number, as suspended or deleted content is removed after retrieving
    ///the initial collection of messages.
    pub count: i32,
    ///The largest/most recent DM ID returned in the last call to `start`, `older`, or `newer`.
    pub max_id: Option<i64>,
    ///The smallest/oldest DM ID returned in the last call to `start`, `older`, or `newer`.
    pub min_id: Option<i64>,
}

impl<'a> Timeline<'a> {
    ///Clear the saved IDs on this timeline.
    pub fn reset(&mut self) {
        self.max_id = None;
        self.min_id = None;
    }

    ///Clear the saved IDs on this timeline, and return the most recent set of messages.
    pub fn start(&mut self) -> WebResponse<Vec<DirectMessage>> {
        self.reset();

        self.older(None)
    }

    ///Return the set of DMs older than the last set pulled, optionally placing a minimum DM ID to
    ///bound with.
    pub fn older(&mut self, since_id: Option<i64>) -> WebResponse<Vec<DirectMessage>> {
        let resp = try!(self.call(since_id, self.min_id.map(|id| id - 1)));

        self.map_ids(&resp.response);

        Ok(resp)
    }

    ///Return the set of DMs newer than the last set pulled, optionally placing a maximum DM ID to
    ///bound with.
    pub fn newer(&mut self, max_id: Option<i64>) -> WebResponse<Vec<DirectMessage>> {
        let resp = try!(self.call(self.max_id, max_id));

        self.map_ids(&resp.response);

        Ok(resp)
    }

    ///Return the set of DMs between the IDs given.
    ///
    ///Note that the range is not fully inclusive; the message ID given by `since_id` will not be
    ///returned, but the message with `max_id` will be returned.
    ///
    ///If the range of DMs given by the IDs would return more than `self.count`, the newest set
    ///of messages will be returned.
    pub fn call(&self, since_id: Option<i64>, max_id: Option<i64>) -> WebResponse<Vec<DirectMessage>> {
        let mut params = self.params_base.as_ref().cloned().unwrap_or_default();
        add_param(&mut params, "count", self.count.to_string());

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

    ///With the returned slice of DMs, set the min_id and max_id on self.
    fn map_ids(&mut self, resp: &[DirectMessage]) {
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
