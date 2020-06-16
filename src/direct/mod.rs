// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Structs and methods for working with direct messages.
//!
//! Note that direct message access requires a special permissions level above regular read/write
//! access. Your app must be configured to have "read, write, and direct message" access to use any
//! function in this module, even the read-only ones.
//!
//! TODO: i'm in the process of rewriting this module, so things are gonna change here real fast

use std::borrow::Cow;
use std::collections::{HashMap, VecDeque};
use std::convert::{TryFrom, TryInto};
use std::future::Future;

use chrono;
use futures::FutureExt;
use futures::stream::{self, Stream, StreamExt, TryStreamExt};
use hyper::{Body, Request};
use serde::{Serialize, Deserialize};

use crate::common::*;
use crate::{auth, entities, error, links};
use crate::tweet::TweetSource;

mod fun;
mod raw;

pub use self::fun::*;

/// Represents a single direct message.
#[derive(Debug)]
pub struct DirectMessage {
    /// Numeric ID for this DM.
    pub id: u64,
    /// UTC timestamp from when this DM was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// The text of the DM.
    pub text: String,
    /// Link, hashtag, and user mention information parsed out of the DM.
    pub entities: DMEntities,
    /// An image, gif, or video attachment, if present.
    pub attachment: Option<entities::MediaEntity>,
    /// A list of "call to action" buttons attached to the DM, if present.
    pub ctas: Option<Vec<Cta>>,
    /// A list of "Quick Replies" sent with this message to request structured input from the other
    /// user.
    pub quick_replies: Option<Vec<QuickReply>>,
    /// The `metadata` accompanying a Quick Reply, if the other user selected a Quick Reply for
    /// their response.
    pub quick_reply_response: Option<String>,
    /// The ID of the user who sent the DM.
    pub sender_id: u64,
    /// The app that sent this direct message.
    ///
    /// Source app information is only available for messages sent by the authorized user. For
    /// received messages written by other users, this field will be `None`.
    pub source_app: Option<TweetSource>,
    /// The ID of the user who received the DM.
    pub recipient_id: u64,
}

impl TryFrom<raw::SingleEvent> for DirectMessage {
    type Error = error::Error;

    fn try_from(ev: raw::SingleEvent) -> error::Result<DirectMessage> {
        let raw::SingleEvent { event, apps } = ev;
        let raw: raw::RawDirectMessage = event.as_message_create().into();
        raw.into_dm(&apps)
    }
}

impl TryFrom<raw::EventCursor> for Vec<DirectMessage> {
    type Error = error::Error;

    fn try_from(evs: raw::EventCursor) -> error::Result<Vec<DirectMessage>> {
        let raw::EventCursor { events, apps, .. } = evs;
        let mut ret = vec![];

        for ev in events {
            let raw: raw::RawDirectMessage = ev.as_message_create().into();
            ret.push(raw.into_dm(&apps)?);
        }

        Ok(ret)
    }
}

/// Container for URL, hashtag, and mention information associated with a direct message.
///
/// As far as entities are concerned, a DM can contain nearly everything a tweet can. The only
/// thing that isn't present here is the "extended media" that would be on the tweet's
/// `extended_entities` field. A user can attach a single picture to a DM, but if that is present,
/// it will be available in the `attachments` field of the original `DirectMessage` struct and not
/// in the entities.
///
/// For all other fields, if the message contains no hashtags, financial symbols ("cashtags"),
/// links, or mentions, those corresponding fields will be empty.
#[derive(Debug, Deserialize)]
pub struct DMEntities {
    /// Collection of hashtags parsed from the DM.
    pub hashtags: Vec<entities::HashtagEntity>,
    /// Collection of financial symbols, or "cashtags", parsed from the DM.
    pub symbols: Vec<entities::HashtagEntity>,
    /// Collection of URLs parsed from the DM.
    pub urls: Vec<entities::UrlEntity>,
    /// Collection of user mentions parsed from the DM.
    pub user_mentions: Vec<entities::MentionEntity>,
}

/// A "call to action" added as a button to a direct message.
#[derive(Debug, Deserialize)]
pub struct Cta {
    /// The label shown to the user for the CTA.
    pub label: String,
    /// The `t.co` URL that the user should navigate to if they click this CTA.
    pub tco_url: String,
    /// The URL given for the CTA, that could be displayed if needed.
    pub url: String,
}

/// A Quick Reply attached to a message to request structured input from a user.
#[derive(Debug, Serialize, Deserialize)]
pub struct QuickReply {
    /// The label shown to the user. When the user selects this Quick Reply, the label will be sent
    /// as the `text` of the reply message.
    pub label: String,
    /// An optional description that accompanies a Quick Reply.
    pub description: Option<String>,
    /// Metadata that accompanies this Quick Reply. Metadata is not shown to the user, but is
    /// available in the `quick_reply_response` when the user selects it.
    pub metadata: String,
}

/// Helper struct to navigate collections of direct messages by tracking the status of Twitter's
/// cursor references.
pub struct Timeline {
    link: &'static str,
    token: auth::Token,
    /// The number of messages to request in a single page. The default is 20; the maximum is 50.
    pub count: u32,
    /// The string ID that can be used to load the next page of results. A value of `None`
    /// indicates that either no messages have been loaded yet, or that the most recently loaded
    /// page is the last page of messages available.
    pub next_cursor: Option<String>,
    /// Whether this `Timeline` has been called yet.
    pub loaded: bool,
}

impl Timeline {
    pub(crate) fn new(link: &'static str, token: auth::Token) -> Timeline {
        Timeline {
            link,
            token,
            count: 20,
            next_cursor: None,
            loaded: false,
        }
    }

    /// Builder function to set the page size. The default value for the page size is 20; the
    /// maximum allowed is 50.
    pub fn with_page_size(self, count: u32) -> Self {
        Timeline {
            count,
            ..self
        }
    }

    /// Clears the saved cursor information on this `Timeline`.
    pub fn reset(&mut self) {
        self.next_cursor = None;
        self.loaded = false;
    }

    fn request(&self, cursor: Option<String>) -> Request<Body> {
        let params = ParamList::new()
            .add_param("count", self.count.to_string())
            .add_opt_param("cursor", cursor);

        get(self.link, &self.token, Some(&params))
    }

    /// Clear the saved cursor information on this timeline, then return the most recent set of
    /// messages.
    pub fn start<'s>(&'s mut self)
        -> impl Future<Output = Result<Response<Vec<DirectMessage>>, error::Error>> + 's
    {
        self.reset();
        self.next_page()
    }

    /// Loads the next page of messages, setting the `next_cursor` to the one received from
    /// Twitter.
    pub fn next_page<'s>(&'s mut self)
        -> impl Future<Output = Result<Response<Vec<DirectMessage>>, error::Error>> + 's
    {
        let next_cursor = self.next_cursor.take();
        let req = self.request(next_cursor);
        let loader = request_with_json_response(req);
        loader.map(
            move |resp: Result<Response<raw::EventCursor>, error::Error>| {
                let mut resp = resp?;
                self.loaded = true;
                self.next_cursor = resp.next_cursor.take();
                Response::try_map(resp, |evs| evs.try_into())
            }
        )
    }

    /// Converts this `Timeline` into a `Stream` of direct messages, which automatically loads the
    /// next page as needed.
    pub fn into_stream(self)
        -> impl Stream<Item = Result<Response<DirectMessage>, error::Error>>
    {
        stream::try_unfold(self, |mut timeline| async move {
            if timeline.loaded && timeline.next_cursor.is_none() {
                Ok::<_, error::Error>(None)
            } else {
                let page = timeline.next_page().await?;
                Ok(Some((page, timeline)))
            }
        }).map_ok(|page| stream::iter(page).map(Ok::<_, error::Error>)).try_flatten()
    }

    /// Loads all the direct messages from this `Timeline` and sorts them into a `DMConversations`
    /// map.
    ///
    /// This adapter is a convenient way to sort all of a user's messages (from the last 30 days)
    /// into a familiar user-interface pattern of a list of conversations between the authenticated
    /// user and a specific other user. This function first pulls all the available messages, then
    /// sorts them into a set of threads by matching them against which user the authenticated user
    /// is messaging.
    pub async fn into_conversations(self) -> Result<DMConversations, error::Error> {
        let dms: Vec<DirectMessage> = self.into_stream().map_ok(|r| r.response).try_collect().await?;
        let mut conversations = HashMap::new();
        let me_id = if let Some(dm) = dms.first() {
            if dm.source_app.is_some() {
                // since the source app info is only populated when the authenticated user sent the
                // message, we know that this message was sent by the authenticated user
                //
                // TODO: is this a valid assumption? i can see this shooting me in the foot in the
                // future
                dm.sender_id
            } else {
                dm.recipient_id
            }
        } else {
            // no messages, nothing to sort
            return Ok(conversations);
        };

        for dm in dms {
            let entry = match (dm.sender_id == me_id, dm.recipient_id == me_id) {
                (true, true) => {
                    // if the sender and recipient are the same - and they match the authenticated
                    // user - then it's the listing of "messages to self"
                    conversations.entry(me_id).or_default()
                }
                (true, false) => {
                    conversations.entry(dm.recipient_id).or_default()
                }
                (false, true) => {
                    conversations.entry(dm.sender_id).or_default()
                }
                (false, false) => {
                    return Err(error::Error::InvalidResponse(
                            "messages activity contains disjoint conversations",
                            None));
                }
            };
            entry.push(dm);
        }

        Ok(conversations)
    }
}

/// Wrapper around a collection of direct messages, sorted by their recipient.
///
/// The mapping exposed here is from a User ID to a listing of direct messages between the
/// authenticated user and that user. It's returned by the `into_conversations` adapter on
/// [`Timeline`].
///
/// [`Timeline`]: struct.Timeline.html
pub type DMConversations = HashMap<u64, Vec<DirectMessage>>;

/// Represents a direct message before it is sent.
///
/// The recipient must allow DMs from the authenticated user for this to be successful. In
/// practice, this means that the recipient must either follow the authenticated user, or they must
/// have the "allow DMs from anyone" setting enabled. As the latter setting has no visibility on
/// the API, there may be situations where you can't verify the recipient's ability to receive the
/// requested DM beforehand.
pub struct DraftMessage {
    text: Cow<'static, str>,
    recipient: u64,
    quick_reply_options: VecDeque<QuickReply>,
}

impl DraftMessage {
    /// Creates a new `DraftMessage` with the given text, to be sent to the given recipient.
    pub fn new(text: impl Into<Cow<'static, str>>, recipient: u64) -> DraftMessage {
        DraftMessage {
            text: text.into(),
            recipient,
            quick_reply_options: VecDeque::new(),
        }
    }

    /// Adds an Option-type Quick Reply to this draft message.
    ///
    /// Quick Replies allow you to request structured input from the other user. They'll have the
    /// opportunity to select from the options you add to the message when you send it. If they
    /// select one of the given options, its `metadata` will be given in the response in the
    /// `quick_reply_response` field.
    ///
    /// Note that while `description` is optional in this call, Twitter will not send the message
    /// if only some of the given Quick Replies have `description` fields.
    ///
    /// The fields here have the following length restrictions:
    ///
    /// * `label` has a maximum of 36 characters, including spaces.
    /// * `metadata` has a maximum of 1000 characters, including spaces.
    /// * `description` has a maximum of 72 characters, including spaces.
    ///
    /// There is a maximum of 20 Quick Reply Options on a single Direct Message. If you try to add
    /// more, the oldest one will be removed.
    ///
    /// Users can only respond to Quick Replies in the Twitter Web Client, and Twitter for
    /// iOS/Android.
    ///
    /// It is not possible to respond to a Quick Reply sent to yourself, though Twitter will
    /// register the options in the message it returns.
    pub fn quick_reply_option(
        mut self,
        label: impl Into<String>,
        metadata: impl Into<String>,
        description: Option<String>
    ) -> Self {
        if self.quick_reply_options.len() == 20 {
            self.quick_reply_options.pop_front();
        }
        self.quick_reply_options.push_back(QuickReply {
            label: label.into(),
            metadata: metadata.into(),
            description,
        });
        self
    }

    /// Sends this direct message using the given `Token`.
    ///
    /// If the message was successfully sent, this function will return the `DirectMessage` that
    /// was just sent.
    pub async fn send(self, token: &auth::Token) -> Result<Response<DirectMessage>, error::Error> {
        let mut message_data = serde_json::json!({
            "text": self.text
        });
        if !self.quick_reply_options.is_empty() {
            message_data.as_object_mut().unwrap().insert("quick_reply".into(), serde_json::json!({
                "type": "options",
                "options": self.quick_reply_options
            }));
        }

        let message = serde_json::json!({
            "event": {
                "type": "message_create",
                "message_create": {
                    "target": {
                        "recipient_id": self.recipient
                    },
                    "message_data": message_data
                }
            }
        });
        let req = post_json(links::direct::SEND, token, message);
        let resp: Response<raw::SingleEvent> = request_with_json_response(req).await?;
        Response::try_map(resp, |ev| ev.try_into())
    }
}
