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

use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};
use std::future::Future;
use std::mem;

use chrono;
use futures::FutureExt;
use futures::stream::{self, Stream, StreamExt, TryStreamExt};
use hyper::{Body, Request};
use serde::{Deserialize, Deserializer};

use crate::common::*;
use crate::{auth, entities, error, user};
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

// impl<'de> Deserialize<'de> for DirectMessage {
//     fn deserialize<D>(deser: D) -> Result<DirectMessage, D::Error>
//     where
//         D: Deserializer<'de>,
//     {
//         let mut raw = raw::RawDirectMessage::deserialize(deser)?;

//         raw.translate_indices();

//         Ok(DirectMessage {
//             id: raw.id,
//             created_at: raw.created_at,
//             text: raw.text,
//             entities: raw.entities,
//             attachment: raw.attachment,
//             ctas: raw.ctas,
//             sender_id: raw.sender_id,
//             recipient_id: raw.recipient_id,
//             quick_replies: raw.quick_replies,
//             quick_reply_response: raw.quick_reply_response,
//         })
//     }
// }

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
#[derive(Debug, Deserialize)]
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
}

/////Wrapper around a collection of direct messages, sorted by their recipient.
/////
/////The mapping exposed here is from a User ID to a listing of direct messages between the
/////authenticated user and that user. For more information, see the docs for [`ConversationTimeline`].
/////
/////[`ConversationTimeline`]: struct.ConversationTimeline.html
//pub type DMConversations = HashMap<u64, Vec<DirectMessage>>;

/////Load the given set of conversations into this set.
//fn merge(this: &mut DMConversations, conversations: DMConversations) {
//    for (id, convo) in conversations {
//        let messages = this.entry(id).or_insert(Vec::new());
//        let cap = convo.len() + messages.len();
//        let old_convo = mem::replace(messages, Vec::with_capacity(cap));

//        //ASSUMPTION: these conversation threads are disjoint
//        if old_convo.first().map(|m| m.id).unwrap_or(0) > convo.first().map(|m| m.id).unwrap_or(0) {
//            messages.extend(old_convo);
//            messages.extend(convo);
//        } else {
//            messages.extend(convo);
//            messages.extend(old_convo);
//        }
//    }
//}

///// Helper struct to load both sent and received direct messages, pre-sorting them into
///// conversations by their recipient.
/////
///// This timeline loader is meant to get around a limitation of the direct message API endpoints:
///// Twitter only gives endpoints to load all the messages *sent* by the authenticated user, or all
///// the messages *received* by the authenticated user. However, the common user interface for DMs
///// is to separate them by the other account in the conversation. This loader is a higher-level
///// wrapper over the direct `sent` and `received` calls to achieve this interface without library
///// users having to implement it themselves.
/////
///// Much like [`Timeline`], simply receiving a `ConversationTimeline` from `conversations` does not
///// load any messages on its own. This is to allow setting the page size before loading the first
///// batch of messages.
/////
///// [`Timeline`]: struct.Timeline.html
/////
///// `ConversationTimeline` keeps a cache of all the messages its loaded, and updates this during
///// calls to Twitter. Any calls on this timeline that generate a `ConversationFuture` will take
///// ownership of the `ConversationTimeline` so that it can update this cache. The Future will
///// return the `ConversationTimeline` on success. To view the current cache, use the
///// `conversations` field.
/////
///// There are two methods to load messages, and they operate by extending the cache by loading
///// messages either older or newer than the extent of the cache.
/////
///// **NOTE**: Twitter has different API limitations for sent versus received messages. You can only
///// load the most recent 200 *received* messages through the public API, but you can load up to 800
///// *sent* messages. This can create some strange user-interface if a user has some old
///// conversations, as they can only see their own side of the conversation this way. If you'd like
///// to load as many messages as possible, both API endpoints have a per-call limit of 200. Setting
///// the page size to 200 prior to loading messages allows you to use one function call to load a
///// fairly-complete view of the user's conversations.
/////
///// # Example
/////
///// ```rust,no_run
///// # use egg_mode::Token;
///// # #[tokio::main]
///// # async fn main() {
///// # let token: Token = unimplemented!();
///// let mut conversations = egg_mode::direct::conversations(&token);
/////
///// // newest() and oldest() consume the Timeline and give it back on success, so assign it back
///// // when it's done
///// conversations = conversations.newest().await.unwrap();
/////
///// for (id, convo) in &conversations.conversations {
/////     let user = egg_mode::user::show(*id, &token).await.unwrap();
/////     println!("Conversation with @{}", user.screen_name);
/////     for msg in convo {
/////         println!("<@{}> {}", msg.sender_screen_name, msg.text);
/////     }
///// }
///// # }
///// ```
//pub struct ConversationTimeline {
//    sent: Timeline,
//    received: Timeline,
//    ///The message ID of the most recent sent message in the current conversation set.
//    pub last_sent: Option<u64>,
//    ///The message ID of the most recent received message in the current conversation set.
//    pub last_received: Option<u64>,
//    ///The message ID of the oldest sent message in the current conversation set.
//    pub first_sent: Option<u64>,
//    ///The message ID of the oldest received message in the current conversation set.
//    pub first_received: Option<u64>,
//    ///The number of messages loaded per API call.
//    pub count: u32,
//    ///The conversation threads that have been loaded so far.
//    pub conversations: DMConversations,
//}

//impl ConversationTimeline {
//    fn new(token: &auth::Token) -> ConversationTimeline {
//        ConversationTimeline {
//            sent: sent(token),
//            received: received(token),
//            last_sent: None,
//            last_received: None,
//            first_sent: None,
//            first_received: None,
//            count: 20,
//            conversations: HashMap::new(),
//        }
//    }

//    fn merge(&mut self, sent: Vec<DirectMessage>, received: Vec<DirectMessage>) {
//        self.last_sent = max_opt(self.last_sent, sent.first().map(|m| m.id));
//        self.last_received = max_opt(self.last_received, received.first().map(|m| m.id));
//        self.first_sent = min_opt(self.first_sent, sent.last().map(|m| m.id));
//        self.first_received = min_opt(self.first_received, received.last().map(|m| m.id));

//        let sender = sent.first().map(|m| m.sender_id);
//        let receiver = received.first().map(|m| m.recipient_id);

//        if let Some(me_id) = sender.or(receiver) {
//            let mut new_convo = HashMap::new();

//            for msg in merge_by(sent, received, |left, right| left.id > right.id) {
//                let recipient = if msg.sender_id == me_id {
//                    msg.recipient_id
//                } else {
//                    msg.sender_id
//                };

//                let thread = new_convo.entry(recipient).or_insert(Vec::new());
//                thread.push(msg);
//            }

//            merge(&mut self.conversations, new_convo);
//        }
//    }

//    ///Builder function to set the number of messages pulled in a single request.
//    pub fn with_page_size(self, page_size: u32) -> ConversationTimeline {
//        ConversationTimeline {
//            count: page_size,
//            ..self
//        }
//    }

//    ///Load messages newer than the currently-loaded set, or the newset set if no messages have
//    ///been loaded yet. The complete conversation set can be viewed from the `ConversationTimeline`
//    ///after it is finished loading.
//    pub async fn newest(self) -> Result<ConversationTimeline, error::Error> {
//        let sent = self.sent.call(self.last_sent, None);
//        let received = self.received.call(self.last_received, None);

//        self.make_future(sent, received).await
//    }

//    ///Load messages older than the currently-loaded set, or the newest set if no messages have
//    ///been loaded. The complete conversation set can be viewed from the `ConversationTimeline`
//    ///after it is finished loading.
//    pub fn next(self) -> impl Future<Output = Result<ConversationTimeline, error::Error>> {
//        let sent = self.sent.call(None, self.first_sent);
//        let received = self.received.call(None, self.first_received);

//        self.make_future(sent, received)
//    }

//    async fn make_future<S, R>(
//        mut self,
//        sent: S,
//        received: R,
//    ) -> Result<ConversationTimeline, error::Error>
//    where
//        S: Future<Output = Result<Response<Vec<DirectMessage>>, error::Error>>,
//        R: Future<Output = Result<Response<Vec<DirectMessage>>, error::Error>>,
//    {
//        let (sent, received) = futures::future::join(sent, received).await;
//        let sent = sent?;
//        let received = received?;
//        self.merge(sent.response, received.response);
//        Ok(self)
//    }
//}
