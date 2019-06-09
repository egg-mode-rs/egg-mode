// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Access to the Streaming API.

use std::collections::HashMap;
use std::str::FromStr;
use std::{self, io};

use futures::{Async, Future, Poll, Stream};
use hyper::client::ResponseFuture;
use hyper::{Body, Request};
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use serde_json;

use auth::{self, Token};
use error;
use links;
use tweet::Tweet;

use common::*;

// https://developer.twitter.com/en/docs/tweets/filter-realtime/guides/streaming-message-types
/// Represents the kinds of messages that can be sent over Twitter's Streaming API.
#[derive(Debug)]
pub enum StreamMessage {
    /// A blank line, sent periodically to keep the connection alive.
    Ping,
    /// A list of accounts the authenticated user follows, sent at the beginning of the session for
    /// user streams.
    FriendList(Vec<u64>),
    /// A new tweet.
    ///
    /// Note that the `entities` inside the `user` field will be empty for tweets received via the
    /// Streaming API.
    Tweet(Tweet),
    /// Notice given when a user deletes a post.
    ///
    /// Clients are expected to comply with these notices by removing the status "from memory and
    /// any storage or archive, even in the rare case where a deletion message arrives earlier in
    /// the stream than the Tweet it references."
    Delete {
        /// The status that was deleted.
        status_id: u64,
        /// The user that deleted the status.
        user_id: u64,
    },
    /// Notice given when a user removes geolocation information from their profile.
    ///
    /// Clients are expected to comply by deleting cached geolocation information from tweets by
    /// the given user, for any tweets up to and including the given status ID. According to
    /// Twitter's documentation, "These messages may also arrive before a Tweet which falls into
    /// the specified range, although this is rare."
    ScrubGeo {
        /// The user whose geolocation information needs to be scrubbed.
        user_id: u64,
        /// The last status ID to scrub information from.
        up_to_status_id: u64,
    },
    /// Placeholder message used to indicate that a specific tweet has been withheld in certain
    /// countries.
    StatusWithheld {
        /// The status that was withheld.
        status_id: u64,
        /// The user that posted the status.
        user_id: u64,
        /// A list of uppercase two-character country codes listing the countries where the tweet
        /// was withheld.
        withheld_in_countries: Vec<String>,
    },
    /// Placeholder message used to indicate that a specific user's content has been withheld in
    /// certain countries.
    UserWithheld {
        /// The user whose content was withheld.
        user_id: u64,
        /// A list of uppercase two-character country codes listing the countries where the content
        /// was withheld.
        withheld_in_countries: Vec<String>,
    },
    /// An error message that may be delivered immediately prior to Twitter disconnecting the
    /// stream.
    ///
    /// Note that if the stream is disconnected due to network issues or the client reading
    /// messages too slowly, it's possible that this message may not be received.
    ///
    /// The enclosed values are an error code and error description. A non-exhaustive list of error
    /// codes and their associated reasons are available on [Twitter's stream
    /// docmentation][stream-doc], under "Disconnect messages (disconnect)".
    ///
    /// [stream-doc]: https://developer.twitter.com/en/docs/tweets/filter-realtime/guides/streaming-message-types
    Disconnect(u64, String),
    /// An unhandled message payload.
    ///
    /// Twitter can add new streaming messages to the API, and egg-mode includes them here so that
    /// they can be used before egg-mode has a chance to handle them.
    Unknown(serde_json::Value),
    //TODO: stall warnings? "follows over limit" warnings? (other warnings?)
}

impl<'de> Deserialize<'de> for StreamMessage {
    fn deserialize<D>(deser: D) -> Result<StreamMessage, D::Error>
    where
        D: Deserializer<'de>,
    {
        macro_rules! fetch {
            ($input: ident, $key: expr) => {
                $input
                    .get($key)
                    .and_then(|val| serde_json::from_value(val.clone()).ok())
                    .ok_or_else(|| D::Error::custom("Failed"))
            };
        }

        let input = serde_json::Value::deserialize(deser)?;
        let msg = if let Some(del) = input.get("delete").and_then(|d| d.get("status")) {
            StreamMessage::Delete {
                status_id: fetch!(del, "id")?,
                user_id: fetch!(del, "user_id")?,
            }
        } else if let Some(scrub) = input.get("scrub_geo") {
            StreamMessage::ScrubGeo {
                user_id: fetch!(scrub, "user_id")?,
                up_to_status_id: fetch!(scrub, "up_to_status_id")?,
            }
        } else if let Some(tweet) = input.get("status_withheld") {
            StreamMessage::StatusWithheld {
                status_id: fetch!(tweet, "id")?,
                user_id: fetch!(tweet, "user_id")?,
                withheld_in_countries: fetch!(tweet, "withheld_in_countries")?,
            }
        } else if let Some(user) = input.get("user_withheld") {
            StreamMessage::UserWithheld {
                user_id: fetch!(user, "id")?,
                withheld_in_countries: fetch!(user, "withheld_in_countries")?,
            }
        } else if let Some(err) = input.get("disconnect") {
            StreamMessage::Disconnect(fetch!(err, "code")?, fetch!(err, "reason")?)
        } else if let Some(friends) = input.get("friends") {
            StreamMessage::FriendList(
                serde_json::from_value(friends.clone())
                    .map_err(|e| D::Error::custom(format!("{}", e)))?,
            )
        // TODO remove clone?
        } else if let Ok(tweet) = serde_json::from_value::<Tweet>(input.clone()) {
            StreamMessage::Tweet(tweet)
        } else {
            StreamMessage::Unknown(input.clone())
        };
        Ok(msg)
    }
}

impl FromStr for StreamMessage {
    type Err = error::Error;
    fn from_str(input: &str) -> Result<Self, error::Error> {
        let input = input.trim();
        if input.is_empty() {
            Ok(StreamMessage::Ping)
        } else {
            Ok(serde_json::from_str(input)?)
        }
    }
}

/// A `Stream` that represents a connection to the Twitter Streaming API.
#[must_use = "Streams are lazy and do nothing unless polled"]
pub struct TwitterStream {
    buf: Vec<u8>,
    request: Option<Request<Body>>,
    response: Option<ResponseFuture>,
    body: Option<Body>,
}

impl TwitterStream {
    fn new(request: Request<Body>) -> TwitterStream {
        TwitterStream {
            buf: vec![],
            request: Some(request),
            response: None,
            body: None,
        }
    }
}

impl Stream for TwitterStream {
    type Item = StreamMessage;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        if let Some(req) = self.request.take() {
            self.response = Some(get_response(req)?);
        }

        if let Some(mut resp) = self.response.take() {
            match resp.poll() {
                Err(e) => return Err(e.into()),
                Ok(Async::NotReady) => {
                    self.response = Some(resp);
                    return Ok(Async::NotReady);
                }
                Ok(Async::Ready(resp)) => {
                    let status = resp.status();
                    if !status.is_success() {
                        //TODO: should i try to pull the response regardless?
                        return Err(error::Error::BadStatus(status));
                    }

                    self.body = Some(resp.into_body());
                }
            }
        }

        if let Some(mut body) = self.body.take() {
            loop {
                match body.poll() {
                    Err(e) => {
                        self.body = Some(body);
                        return Err(e.into());
                    }
                    Ok(Async::NotReady) => {
                        self.body = Some(body);
                        return Ok(Async::NotReady);
                    }
                    Ok(Async::Ready(None)) => {
                        //TODO: introduce a new error for this?
                        return Err(error::Error::FutureAlreadyCompleted);
                    }
                    Ok(Async::Ready(Some(chunk))) => {
                        self.buf.extend(&*chunk);

                        if let Some(pos) = self.buf.windows(2).position(|w| w == b"\r\n") {
                            self.body = Some(body);
                            let pos = pos + 2;
                            let resp = if let Ok(msg_str) = std::str::from_utf8(&self.buf[..pos]) {
                                StreamMessage::from_str(msg_str)
                            } else {
                                Err(io::Error::new(
                                    io::ErrorKind::InvalidData,
                                    "stream did not contain valid UTF-8",
                                )
                                .into())
                            };

                            self.buf.drain(..pos);
                            return Ok(Async::Ready(Some(resp?)));
                        }
                    }
                }
            }
        } else {
            Err(error::Error::FutureAlreadyCompleted)
        }
    }
}

/// Represents the amount of filtering that can be done to streams on Twitter's side.
///
/// According to Twitter's documentation, "When displaying a stream of Tweets to end users
/// (dashboards or live feeds at a presentation or conference, for example) it is suggested that
/// you set this value to medium."
#[derive(Copy, Clone, Debug, Deserialize)]
pub enum FilterLevel {
    /// No filtering.
    #[serde(rename = "none")]
    None,
    /// A light amount of filtering.
    #[serde(rename = "low")]
    Low,
    /// A medium amount of filtering.
    #[serde(rename = "medium")]
    Medium,
}

/// `Display` impl to turn `FilterLevel` variants into the form needed for stream parameters. This
/// is basically "the variant name, in lowercase".
// TODO Probably can remove this somehow
impl ::std::fmt::Display for FilterLevel {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match *self {
            FilterLevel::None => write!(f, "none"),
            FilterLevel::Low => write!(f, "low"),
            FilterLevel::Medium => write!(f, "medium"),
        }
    }
}

/// Represents a `TwitterStream` before it is started.
pub struct StreamBuilder {
    url: &'static str,
    follow: Vec<u64>,
    track: Vec<String>,
    language: Vec<String>,
    filter_level: Option<FilterLevel>,
}

impl StreamBuilder {
    fn new(url: &'static str) -> Self {
        StreamBuilder {
            url: url,
            follow: Vec::new(),
            track: Vec::new(),
            language: Vec::new(),
            filter_level: None,
        }
    }

    /// Filter stream to only return Tweets from given user IDs.
    pub fn follow(mut self, to_follow: &[u64]) -> Self {
        self.follow.extend(to_follow.into_iter());
        self
    }

    /// Filter stream to only return Tweets containing given phrases.
    ///
    /// A phrase may be one or more terms separated by spaces, and a phrase will match if all
    /// of the terms in the phrase are present in the Tweet, regardless of order and ignoring case.
    pub fn track<I: IntoIterator<Item = S>, S: AsRef<str>>(mut self, to_track: I) -> Self {
        self.track
            .extend(to_track.into_iter().map(|s| s.as_ref().to_string()));
        self
    }

    /// Filter stream to only return Tweets that have been detected as being written
    /// in the specified languages.
    ///
    /// Languages are specified as an list of
    /// [BCP 47](http://tools.ietf.org/html/bcp47) language identifiers
    /// corresponding to any of the languages listed on Twitter’s
    /// [advanced search](https://twitter.com/search-advancedpage) page.
    ///
    /// *Note* This library does not validate the language codes
    pub fn language<I: IntoIterator<Item = S>, S: AsRef<str>>(mut self, languages: I) -> Self {
        self.language
            .extend(languages.into_iter().map(|s| s.as_ref().to_string()));
        self
    }

    /// Applies the given `FilterLevel` to the stream. Tweets with a `filter_level` below the given
    /// value will not be shown in the stream.
    ///
    /// According to Twitter's documentation, "When displaying a stream of Tweets to end users
    /// (dashboards or live feeds at a presentation or conference, for example) it is suggested
    /// that you set this value to medium."
    pub fn filter_level(self, filter_level: FilterLevel) -> StreamBuilder {
        StreamBuilder {
            filter_level: Some(filter_level),
            ..self
        }
    }

    /// Finalizes the stream parameters and returns the resulting `TwitterStream`.
    ///
    /// *Note*: Stream will fail at point of connection if neither a `track` nor a `follow`
    /// filter has been specified
    pub fn start(self, token: &Token) -> TwitterStream {
        let mut params = HashMap::new();

        if let Some(filter_level) = self.filter_level {
            add_param(&mut params, "filter_level", filter_level.to_string());
        }

        if !self.follow.is_empty() {
            let to_follow = self
                .follow
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<String>>()
                .join(",");
            add_param(&mut params, "follow", to_follow);
        }

        if !self.track.is_empty() {
            let to_track = self.track.join(",");
            add_param(&mut params, "track", to_track);
        }

        if !self.language.is_empty() {
            let langs = self.language.join(",");
            add_param(&mut params, "language", langs);
        }

        let req = auth::post(self.url, token, Some(&params));

        TwitterStream::new(req)
    }
}

/// Begins building a request to a filtered public stream.
pub fn filter() -> StreamBuilder {
    StreamBuilder::new(links::stream::FILTER)
}

/// Opens a `TwitterStream` returning "a small random sample of all public statuses".
///
/// As sample streams don't have the same configuration options as filter streams,
/// this directly returns a `TwitterStream`, rather than going through a [`StreamBuilder`]. To apply
/// filter options on the public stream, start with [`filter`] and add parameters to the
/// [`StreamBuilder`] returned there.
///
/// [`StreamBuilder`]: struct.StreamBuilder.html
/// [`filter`]: fn.filter.html
pub fn sample(token: &Token) -> TwitterStream {
    let req = auth::get(links::stream::SAMPLE, token, None);
    TwitterStream::new(req)
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::tests::load_file;

    fn load_stream(path: &str) -> StreamMessage {
        let sample = load_file(path);
        ::serde_json::from_str(&sample).unwrap()
    }

    #[test]
    fn parse_tweet_stream() {
        let msg = load_stream("src/stream/sample.json");
        if let StreamMessage::Tweet(_tweet) = msg {
            // OK
        } else {
            panic!("Not a tweet")
        }
    }

    #[test]
    fn parse_empty_stream() {
        let msg = StreamMessage::from_str("").unwrap();
        if let StreamMessage::Ping = msg {
            // OK
        } else {
            panic!("Not a ping")
        }
    }
}
