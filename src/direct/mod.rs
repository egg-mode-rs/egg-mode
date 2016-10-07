//! Structs and methods for working with direct messages.

use common::*;

use rustc_serialize::json;

use auth;
use user;
use entities;
use error;
use error::Error::InvalidResponse;

mod fun;

pub use self::fun::*;

///Represents a single direct message.
pub struct DirectMessage {
    ///Numeric ID for this DM.
    pub id: i64,
    ///UTC timestamp showing when this DM was created, formatted like "Mon Aug 27 17:21:03 +0000
    ///2012".
    pub created_at: String,
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

///Container for URL, hashtag, and user mention information associated with a direct message.
pub struct DMEntities {
    ///Collection of hashtags parsed from the DM.
    pub hashtags: Vec<entities::HashtagEntity>,
    ///Collection of URLs parsed from the DM.
    pub urls: Vec<entities::UrlEntity>,
    ///Collection of user mentions parsed from the DM.
    pub user_mentions: Vec<entities::MentionEntity>,
}

impl FromJson for DirectMessage {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse("DirectMessage received json that wasn't an object",
                                       Some(input.to_string())));
        }

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

        Ok(DMEntities {
            hashtags: try!(field(input, "hashtags")),
            urls: try!(field(input, "urls")),
            user_mentions: try!(field(input, "user_mentions")),
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
/// //TODO: These aren't direct messages! >_>
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
