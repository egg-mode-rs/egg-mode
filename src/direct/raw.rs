// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::common::*;

use std::collections::HashMap;

use chrono;
use serde::Deserialize;

use crate::entities::MediaEntity;
use crate::tweet::TweetSource;

use super::{DMEntities, Cta, QuickReply, DirectMessage};

// n.b. all of the types in this module are re-exported in `raw::types::direct` - these docs are
// public!

/// Minimally-processed form of `DirectMessage`, prior to changing byte indices or loading
/// source-app information.
///
/// The `RawDirectMessage` type is used in the process of converting from `EventCursor` or
/// `SingleEvent` into a `DirectMessage`. They can be directly loaded from a `DMEvent` struct, but
/// require a mapping of source-app IDs to convert fully into a `DirectMessage`. By giving this
/// mapping to the `into_dm` function, you can convert a `RawDirectMessage` into the final
/// `DirectMessage` type.
///
/// Another way `RawDirectMessage` differs from `DirectMessage` is how its entities are stored.
/// Twitter returns entity information based on *codepoint* indices, whereas Rust Strings are
/// indexed using *byte* indices. egg-mode translates these indices for you when returning a
/// processed type, but that translation has not occurred when a `RawDirectMessage` has been
/// created. The `translate_indices` function can be used to perform this translation if the
/// `RawDirectMessage` is being used directly. The `into_dm` conversion function also performs this
/// translation before returning the final `DirectMessage`.
#[derive(Debug, Deserialize)]
#[serde(from = "DMEvent")]
pub struct RawDirectMessage {
    /// Numeric ID for this DM.
    pub id: u64,
    /// UTC timestamp from when this DM was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// The text of the DM.
    pub text: String,
    /// Link, hashtag, and user mention information parsed out of the DM.
    pub entities: DMEntities,
    /// Media attached to the DM, if present.
    pub attachment: Option<MediaEntity>,
    /// A list of "call to action" buttons, if present.
    pub ctas: Option<Vec<Cta>>,
    /// A list of "quick reply" options, if present.
    pub quick_replies: Option<Vec<QuickReply>>,
    /// The `metadata` associated with the Quick Reply chosen by the sender, if present.
    pub quick_reply_response: Option<String>,
    /// The ID of the user who sent the DM.
    pub sender_id: u64,
    /// The string ID associated with the app used to send the DM, if sent by the authenticated
    /// user.
    pub source_app_id: Option<String>,
    /// The ID of the user who received the DM.
    pub recipient_id: u64,
    translated: bool,
}

impl RawDirectMessage {
    /// Translates the codepoint-based indices in this `RawDirectMessage`'s entities into
    /// byte-based ones.
    ///
    /// Note that `into_dm` also performs this conversion, so if you're ultimately planning to
    /// convert this into a `DirectMessage`, you shouldn't need to call this function directly.
    /// `RawDirectMessage` tracks whether this translation has occured, so if you need to access
    /// the fields before converting, the final conversion won't double-translate and leave you
    /// with invalid indices.
    pub fn translate_indices(&mut self) {
        if !self.translated {
            self.translated = true;

            for entity in &mut self.entities.hashtags {
                codepoints_to_bytes(&mut entity.range, &self.text);
            }
            for entity in &mut self.entities.symbols {
                codepoints_to_bytes(&mut entity.range, &self.text);
            }
            for entity in &mut self.entities.urls {
                codepoints_to_bytes(&mut entity.range, &self.text);
            }
            for entity in &mut self.entities.user_mentions {
                codepoints_to_bytes(&mut entity.range, &self.text);
            }
            if let Some(ref mut media) = self.attachment {
                codepoints_to_bytes(&mut media.range, &self.text);
            }
        }
    }

    /// Converts this `RawDirectMessage` into a `DirectMessage`, using the given source-app
    /// mapping.
    ///
    /// If the ID given in `source_app` is not present in the `apps` mapping, the source-app
    /// information is discarded.
    ///
    /// This conversion also calls `translate_indices` before constructing the `DirectMessage`.
    pub fn into_dm(mut self, apps: &HashMap<String, TweetSource>) -> DirectMessage
    {
        self.translate_indices();
        let source_app = self.source_app_id.and_then(|id| apps.get(&id).cloned());

        DirectMessage {
            id: self.id,
            created_at: self.created_at,
            text: self.text,
            entities: self.entities,
            attachment: self.attachment,
            ctas: self.ctas,
            sender_id: self.sender_id,
            source_app,
            recipient_id: self.recipient_id,
            quick_replies: self.quick_replies,
            quick_reply_response: self.quick_reply_response,
        }
    }

    // TODO: provide a conversion that drops source-app information?
}

// DMs received from twitter are structured as events in their activity API, which means they have
// a lot of deep nesting for how they are structured. The types and From impl below convert that
// into a flat object ready for processing/export by egg-mode.

impl From<DMEvent> for RawDirectMessage {
    fn from(ev: DMEvent) -> RawDirectMessage {
        use chrono::TimeZone;
        RawDirectMessage {
            id: ev.id,
            created_at: chrono::Utc.timestamp_millis(ev.created_timestamp),
            text: ev.message_create.message_data.text,
            entities: ev.message_create.message_data.entities,
            attachment: ev.message_create.message_data.attachment.map(|a| a.media),
            ctas: ev.message_create.message_data.ctas,
            sender_id: ev.message_create.sender_id,
            source_app_id: ev.message_create.source_app_id,
            recipient_id: ev.message_create.target.recipient_id,
            quick_replies: ev.message_create.message_data.quick_reply.map(|q| q.options),
            quick_reply_response: ev.message_create.message_data.quick_reply_response.map(|q| q.metadata),
            translated: false,
        }
    }
}

/// Single direct message event.
#[derive(Deserialize)]
pub struct SingleEvent {
    /// Information about the event.
    pub event: EventType,
    /// Mapping of source app ID to information about the app, if this message was sent by the
    /// authenticated user.
    #[serde(default)]
    pub apps: HashMap<String, TweetSource>,
}

/// Listing of direct message events, represented as a cursored page within a larger data set.
#[derive(Deserialize)]
pub struct EventCursor {
    /// The list of events contained on this page.
    pub events: Vec<EventType>,
    /// The mapping of source app IDs to information about the app, if messages on this page were
    /// sent by the authenticated user.
    #[serde(default)]
    pub apps: HashMap<String, TweetSource>,
    /// String ID for the next page of message events, if more exist.
    pub next_cursor: Option<String>,
}

/// Wrapper enum to represent a `DMEvent` in the Account Activity API.
///
/// As direct messages are part of the Account Activity API, they are presented as an event type in
/// a broader event envelope. This enum mainly encapsulates the requirement that direct messages
/// are returned as the `message_create` event type with the proper data structure.
#[derive(Deserialize)]
#[serde(tag="type")]
#[serde(rename_all="snake_case")]
pub enum EventType {
    /// A `message_create` event, representing a direct message.
    ///
    /// The `message_create` event structure is flattened into a `RawDirectMessage` when
    /// deserializing. It should be combined with the `apps` map in a `SingleEvent` or
    /// `EventCursor` when converting into a `DirectMessage`.
    MessageCreate(RawDirectMessage),
}

impl EventType {
    /// Returns the inner `RawDirectMessage` structure from the `message_create` event.
    pub fn as_raw_dm(self) -> RawDirectMessage {
        let EventType::MessageCreate(dm) = self;
        dm
    }
}

/// The root `message_create` event, representing a direct message.
#[derive(Deserialize)]
struct DMEvent {
    /// Numeric ID for the direct message.
    #[serde(with = "serde_via_string")]
    id: u64,
    /// UTC Unix timestamp for when the message was sent, encoded as the number of milliseconds
    /// since the Unix epoch.
    #[serde(with = "serde_via_string")]
    created_timestamp: i64,
    /// Message data for this event.
    message_create: MessageCreateEvent,
}

/// The `message_create` data of a `DMEvent`, containing information about the direct message.
#[derive(Deserialize)]
struct MessageCreateEvent {
    /// The `message_data` portion of this event.
    message_data: MessageData,
    #[serde(with = "serde_via_string")]
    /// The numeric User ID of the sender.
    sender_id: u64,
    /// The string ID of the app used to send the message, if it was sent by the authenticated
    /// user.
    source_app_id: Option<String>,
    /// Information about the recipient of the message.
    target: MessageTarget,
}

/// The `message_data` portion of a `DMEvent`, containing the bulk of information about a direct
/// message.
#[derive(Deserialize)]
struct MessageData {
    /// A list of "call to action" buttons, if present.
    ctas: Option<Vec<Cta>>,
    /// Information about attached media, if present.
    attachment: Option<MessageAttachment>,
    /// Information about URL, hashtag, or user-mention entities used in the message.
    entities: DMEntities,
    /// Information about Quick Reply options, if present.
    quick_reply: Option<RawQuickReply>,
    /// Information about a selected Quick Reply option, if the sender selected one.
    quick_reply_response: Option<QuickReplyResponse>,
    /// The message text.
    text: String,
}

/// Represents attached media information from within a `DMEvent`.
#[derive(Deserialize)]
struct MessageAttachment {
    /// Information about the attached media.
    ///
    /// Note that the indices used within the `MediaEntity` are received from Twitter using
    /// codepoint-based indexing. Using the indices from within this type directly without
    /// translating them may result in string-slicing errors or panics unless you translate the
    /// indices or use `char_indices` and `enumerate` yourself to ensure proper use of the indices.
    media: MediaEntity,
}

/// Represents a list of Quick Reply options from within a `DMEvent`.
#[derive(Deserialize)]
struct RawQuickReply {
    /// The list of Quick Reply options sent with this message.
    options: Vec<QuickReply>,
}

/// Represents the `metadata` from a selected Quick Reply from within a `DMEvent`.
#[derive(Deserialize)]
struct QuickReplyResponse {
    /// The `metadata` field for the Quick Reply option the sender selected.
    metadata: String,
}

/// Represents the message target from within a `DMEvent`.
#[derive(Deserialize)]
struct MessageTarget {
    #[serde(with = "serde_via_string")]
    /// The numeric user ID of the recipient of the message.
    recipient_id: u64,
}
