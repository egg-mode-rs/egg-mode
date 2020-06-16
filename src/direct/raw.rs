// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::common::*;

use chrono;
use serde::Deserialize;

use crate::entities::MediaEntity;

use super::DMEntities;

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
    ///The ID of the user who sent the DM.
    pub sender_id: u64,
    ///The ID of the user who received the DM.
    pub recipient_id: u64,
}

// DMs received from twitter are structured as events in their activity API, which means they have
// a lot of deep nesting for how they are structured. The types and From impl below convert that
// into a flat object ready for processing/export by egg-mode.

impl From<DMEvent> for RawDirectMessage {
    fn from(ev: DMEvent) -> RawDirectMessage {
        use chrono::TimeZone;
        RawDirectMessage {
            id: ev.id,
            created_at: chrono::Utc.timestamp_millis(ev.created_at),
            text: ev.message_create.message_data.text,
            entities: ev.message_create.message_data.entities,
            attachment: ev.message_create.message_data.attachment.map(|a| a.media),
            sender_id: ev.message_create.sender_id,
            recipient_id: ev.message_create.target.recipient_id,
        }
    }
}

#[derive(Deserialize)]
struct DMEvent {
    #[serde(deserialize_with = "deser_from_string")]
    id: u64,
    #[serde(deserialize_with = "deser_from_string")]
    #[serde(rename = "created_timestamp")]
    created_at: i64,
    message_create: MessageCreateEvent,
}

#[derive(Deserialize)]
struct MessageCreateEvent {
    message_data: MessageData,
    #[serde(deserialize_with = "deser_from_string")]
    sender_id: u64,
    target: MessageTarget,
}

#[derive(Deserialize)]
struct MessageData {
    attachment: Option<MessageAttachment>,
    entities: DMEntities,
    text: String,
}

#[derive(Deserialize)]
struct MessageAttachment {
    media: MediaEntity,
}

#[derive(Deserialize)]
struct MessageTarget {
    #[serde(deserialize_with = "deser_from_string")]
    recipient_id: u64,
}
