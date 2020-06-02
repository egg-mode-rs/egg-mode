// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Raw access to request- and response-building primitives used internally by egg-mode.

use hyper::{Body, Request};

use crate::auth::Token;
use crate::cursor;
use crate::stream::TwitterStream;

use crate::tweet::Timeline as TweetTimeline;
use crate::direct::Timeline as DMTimeline;

pub use crate::common::ParamList;
pub use crate::common::Headers;

pub use crate::auth::get as request_get;
pub use crate::auth::post as request_post;
pub use crate::auth::post_json as request_post_json;

/// Assemble a GET request and convert it to a `Timeline` of tweets.
pub fn request_as_tweet_timeline(
    url: &'static str,
    token: &Token,
    params: Option<ParamList>
) -> TweetTimeline {
    TweetTimeline::new(url, params, token)
}

/// Assemble a GET request and convert it to a `Timeline` of direct messages.
pub fn request_as_dm_timeline(
    url: &'static str,
    token: &Token,
    params: Option<ParamList>
) -> DMTimeline {
    DMTimeline::new(url, params, token)
}

/// Assemble a GET request and convert it to a `CursorIter`.
pub fn request_as_cursor_iter<T: cursor::Cursor + serde::de::DeserializeOwned>(
    url: &'static str,
    token: &Token,
    params: Option<ParamList>,
    page_size: Option<i32>
) -> cursor::CursorIter<T> {
    cursor::CursorIter::new(url, token, params, page_size)
}

pub use crate::common::get_response as response_future;
pub use crate::common::raw_request as response_raw_bytes;
pub use crate::common::request_with_json_response as response_json;

/// Converts the given request into a `TwitterStream`.
///
/// This function can be used for endpoints that open a persistent stream, like `GET
/// statuses/sample`. If you know that the messages returned by the stream you're using will look
/// the same as `StreamMessage`, this can be a convenient way to customize a stream if you need to
/// use other endpoints or options not available to `StreamBuilder`.
pub fn response_as_stream(req: Request<Body>) -> TwitterStream {
    TwitterStream::new(req)
}
