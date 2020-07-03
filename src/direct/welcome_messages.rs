// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Types and functions to work with Welcome Messages.

use crate::common::*;

use std::collections::HashMap;

use serde::Deserialize;

use crate::{auth, entities, error, links};
use crate::cursor::{self, ActivityCursor};
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

/// Load the list of welcome messages created by this user.
pub fn list(token: &auth::Token) -> cursor::ActivityCursorIter<WelcomeMessage> {
    cursor::ActivityCursorIter::new(links::direct::welcome_messages::LIST, token)
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
        let SingleMessage { apps, welcome_message } = raw;

        welcome_message.into_wm(&apps)
    }
}

impl From<MessageCursor> for Vec<WelcomeMessage> {
    fn from(raw: MessageCursor) -> Vec<WelcomeMessage> {
        let MessageCursor { apps, welcome_messages, .. } = raw;

        welcome_messages.into_iter().map(|wm| wm.into_wm(&apps)).collect()
    }
}

impl From<MessageCursor> for ActivityCursor<WelcomeMessage> {
    fn from(mut raw: MessageCursor) -> ActivityCursor<WelcomeMessage> {
        let next_cursor = raw.next_cursor.take();

        ActivityCursor {
            next_cursor,
            items: raw.into(),
        }
    }
}

impl cursor::ActivityItem for WelcomeMessage {
    type Cursor = MessageCursor;
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

impl RawWelcomeMessage {
    fn into_wm(mut self, apps: &HashMap<String, TweetSource>) -> WelcomeMessage {
        use chrono::TimeZone;

        self.message_data.translate_indices();

        let source_app = self.source_app_id.and_then(|id| apps.get(&id).cloned());

        WelcomeMessage {
            id: self.id,
            name: self.name,
            created_at: chrono::Utc.timestamp_millis(self.created_timestamp),
            text: self.message_data.text,
            entities: self.message_data.entities,
            attachment: self.message_data.attachment.map(|a| a.media),
            ctas: self.message_data.ctas,
            quick_replies: self.message_data.quick_reply.map(|q| q.options),
            source_app,
        }
    }
}

#[derive(Deserialize)]
struct SingleMessage {
    #[serde(default)]
    apps: HashMap<String, TweetSource>,
    welcome_message: RawWelcomeMessage,
}

#[derive(Deserialize)]
#[doc(hidden)] // TODO: move this into a `raw` module and re-export in `raw::types::direct`
pub struct MessageCursor {
    #[serde(default)]
    apps: HashMap<String, TweetSource>,
    welcome_messages: Vec<RawWelcomeMessage>,
    next_cursor: Option<String>,
}
