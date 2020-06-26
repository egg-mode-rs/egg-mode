// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::common::*;

use std::collections::HashMap;

use chrono;
use serde::Deserialize;

use crate::entities::MediaEntity;
use crate::error;
use crate::tweet::TweetSource;

use super::{DMEntities, Cta, QuickReply, DirectMessage};

#[derive(Debug, Deserialize)]
#[serde(from = "DMEvent")]
pub struct RawDirectMessage {
    ///Numeric ID for this DM.
    pub id: u64,
    ///UTC timestamp from when this DM was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    ///The text of the DM.
    pub text: String,
    ///Link, hashtag, and user mention information parsed out of the DM.
    pub entities: DMEntities,
    ///Media attached to the DM, if present.
    pub attachment: Option<MediaEntity>,
    ///A list of "call to action" buttons, if present.
    pub ctas: Option<Vec<Cta>>,
    pub quick_replies: Option<Vec<QuickReply>>,
    pub quick_reply_response: Option<String>,
    ///The ID of the user who sent the DM.
    pub sender_id: u64,
    pub source_app_id: Option<String>,
    ///The ID of the user who received the DM.
    pub recipient_id: u64,
}

impl RawDirectMessage {
    pub fn translate_indices(&mut self) {
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

    pub fn into_dm(mut self, apps: &HashMap<String, TweetSource>)
        -> error::Result<DirectMessage>
    {
        self.translate_indices();
        let source_app = self.source_app_id.and_then(|id| apps.get(&id).cloned());

        Ok(DirectMessage {
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
        })
    }
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
        }
    }
}

#[derive(Deserialize)]
pub struct SingleEvent {
    pub event: EventType,
    #[serde(default)]
    pub apps: HashMap<String, TweetSource>,
}

#[derive(Deserialize)]
pub struct EventCursor {
    pub events: Vec<EventType>,
    #[serde(default)]
    pub apps: HashMap<String, TweetSource>,
    pub next_cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(tag="type")]
#[serde(rename_all="snake_case")]
pub enum EventType {
    MessageCreate(DMEvent),
}

impl EventType {
    pub fn as_message_create(self) -> DMEvent {
        let EventType::MessageCreate(ev) = self;
        ev
    }
}

#[derive(Deserialize)]
pub struct DMEvent {
    #[serde(deserialize_with = "deser_from_string")]
    id: u64,
    #[serde(deserialize_with = "deser_from_string")]
    created_timestamp: i64,
    message_create: MessageCreateEvent,
}

#[derive(Deserialize)]
struct MessageCreateEvent {
    message_data: MessageData,
    #[serde(deserialize_with = "deser_from_string")]
    sender_id: u64,
    source_app_id: Option<String>,
    target: MessageTarget,
}

#[derive(Deserialize)]
struct MessageData {
    ctas: Option<Vec<Cta>>,
    attachment: Option<MessageAttachment>,
    entities: DMEntities,
    quick_reply: Option<RawQuickReply>,
    quick_reply_response: Option<QuickReplyResponse>,
    text: String,
}

#[derive(Deserialize)]
struct MessageAttachment {
    media: MediaEntity,
}

#[derive(Deserialize)]
struct RawQuickReply {
    options: Vec<QuickReply>,
}

#[derive(Deserialize)]
struct QuickReplyResponse {
    metadata: String,
}

#[derive(Deserialize)]
struct MessageTarget {
    #[serde(deserialize_with = "deser_from_string")]
    recipient_id: u64,
}
