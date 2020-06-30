// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Types and functions to work with Welcome Messages.

use crate::common::*;

use std::collections::HashMap;

use serde::Deserialize;

use crate::{auth, entities, error, links};
use crate::tweet::TweetSource;

use super::raw;

/// Load the given welcome message.
pub async fn show(id: u64, token: &auth::Token) -> Result<Response<WelcomeMessage>, error::Error> {
    let params = ParamList::new()
        .add_param("id", id.to_string());
    let req = get(links::direct::welcome_messages::SHOW, token, Some(&params));
    let resp: Response<SingleMessage> = request_with_json_response(req).await?;
    Ok(Response::into(resp))
}

/// A message that can be used to greet users to a DM conversation.
#[derive(Debug)]
pub struct WelcomeMessage {
    /// Numeric ID for this welcome message.
    pub id: u64,
    /// The name given when creating this welcome message, if present.
    pub name: Option<String>,
    /// A timestamp for when this welcome message was created.
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// The text of the message.
    pub text: String,
    /// Information about any user-mentions, hashtags, and URLs present in the message text.
    pub entities: super::DMEntities,
    /// A piece of media attached to this message, if present.
    pub attachment: Option<entities::MediaEntity>,
    /// A list of "call to action" buttons to send with this message, if present.
    pub ctas: Option<Vec<super::Cta>>,
    /// A list of "quick reply" response options to send with this message, if present.
    pub quick_replies: Option<Vec<super::QuickReply>>,
    /// The app that created this welcome message.
    pub source_app: Option<TweetSource>,
}

impl From<SingleMessage> for WelcomeMessage {
    fn from(raw: SingleMessage) -> WelcomeMessage {
        use chrono::TimeZone;

        let SingleMessage { apps, mut welcome_message } = raw;

        for entity in &mut welcome_message.message_data.entities.hashtags {
            codepoints_to_bytes(&mut entity.range, &welcome_message.message_data.text);
        }
        for entity in &mut welcome_message.message_data.entities.symbols {
            codepoints_to_bytes(&mut entity.range, &welcome_message.message_data.text);
        }
        for entity in &mut welcome_message.message_data.entities.urls {
            codepoints_to_bytes(&mut entity.range, &welcome_message.message_data.text);
        }
        for entity in &mut welcome_message.message_data.entities.user_mentions {
            codepoints_to_bytes(&mut entity.range, &welcome_message.message_data.text);
        }
        if let Some(ref mut attachment) = welcome_message.message_data.attachment {
            codepoints_to_bytes(&mut attachment.media.range, &welcome_message.message_data.text);
        }

        let source_app = welcome_message.source_app_id.and_then(|id| apps.get(&id).cloned());

        WelcomeMessage {
            id: welcome_message.id,
            name: welcome_message.name,
            created_at: chrono::Utc.timestamp_millis(welcome_message.created_timestamp),
            text: welcome_message.message_data.text,
            entities: welcome_message.message_data.entities,
            attachment: welcome_message.message_data.attachment.map(|a| a.media),
            ctas: welcome_message.message_data.ctas,
            quick_replies: welcome_message.message_data.quick_reply.map(|q| q.options),
            source_app,
        }
    }
}

#[derive(Deserialize)]
struct RawWelcomeMessage {
    #[serde(with = "serde_via_string")]
    id: u64,
    #[serde(with = "serde_via_string")]
    created_timestamp: i64,
    name: Option<String>,
    message_data: raw::MessageData,
    source_app_id: Option<String>,
}

#[derive(Deserialize)]
struct SingleMessage {
    #[serde(default)]
    apps: HashMap<String, TweetSource>,
    welcome_message: RawWelcomeMessage,
}
