// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Types and functions to work with Welcome Messages.

use crate::common::*;

use std::collections::HashMap;
use std::future::Future;

use futures::FutureExt;
use futures::stream::{self, Stream, StreamExt, TryStreamExt};
use hyper::{Body, Request};
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

/// Load the list of welcome messages created by this user.
pub fn list(token: &auth::Token) -> Timeline {
    Timeline::new(links::direct::welcome_messages::LIST, token.clone())
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
struct MessageCursor {
    #[serde(default)]
    apps: HashMap<String, TweetSource>,
    welcome_messages: Vec<RawWelcomeMessage>,
    next_cursor: Option<String>,
}

/// Helper struct to navigate a collection of welcome messages.
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
        -> impl Future<Output = Result<Response<Vec<WelcomeMessage>>, error::Error>> + 's
    {
        self.reset();
        self.next_page()
    }

    /// Loads the next page of messages, setting the `next_cursor` to the one received from
    /// Twitter.
    pub fn next_page<'s>(&'s mut self)
        -> impl Future<Output = Result<Response<Vec<WelcomeMessage>>, error::Error>> + 's
    {
        let next_cursor = self.next_cursor.take();
        let req = self.request(next_cursor);
        let loader = request_with_json_response(req);
        loader.map(
            move |resp: Result<Response<MessageCursor>, error::Error>| {
                let mut resp = resp?;
                self.loaded = true;
                self.next_cursor = resp.next_cursor.take();
                Ok(Response::into(resp))
            }
        )
    }

    /// Converts this `Timeline` into a `Stream` of direct messages, which automatically loads the
    /// next page as needed.
    pub fn into_stream(self)
        -> impl Stream<Item = Result<Response<WelcomeMessage>, error::Error>>
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
