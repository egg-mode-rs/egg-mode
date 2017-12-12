// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Access to the Streaming API.

use std::{self, io};

use futures::{Future, Stream, Poll, Async};
use hyper::Body;
use hyper::client::{Request, FutureResponse};
use rustc_serialize::json;

use auth::{self, Token};
use direct::DirectMessage;
use error;
use links;
use tweet::Tweet;

use common::*;

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
    /// A direct message.
    DirectMessage(DirectMessage),
    /// Notice given when a user deletes a post. Clients are expected to comply with these notices
    /// by removing the status "from memory and any storage or archive, even in the rare case where
    /// a deletion message arrives earlier in the stream than the Tweet it references."
    Delete {
        /// The status that was deleted.
        status_id: u64,
        /// The user that deleted the status.
        user_id: u64
    },
    /// Notice given when a user removes geolocation information from their profile. Clients are
    /// expected to comply by deleting cached geolocation information from tweets by the given
    /// user, for any tweets up to and including the given status ID. According to Twitter's
    /// documentation, "These messages may also arrive before a Tweet which falls into the
    /// specified range, although this is rare."
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
    /// stream. Note that if the stream is disconnected due to network issues or the client reading
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
    Unknown(json::Json),
}

impl FromJson for StreamMessage {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if let Some(_event) = input.find("event") {
            //TODO: all the event types -_-
            Ok(StreamMessage::Unknown(input.clone()))
        } else if let Some(del) = input.find_path(&["delete", "status"]) {
            Ok(StreamMessage::Delete {
                status_id: try!(field(del, "id")),
                user_id: try!(field(del, "user_id")),
            })
        } else if let Some(scrub) = input.find("scrub_geo") {
            Ok(StreamMessage::ScrubGeo {
                user_id: try!(field(scrub, "user_id")),
                up_to_status_id: try!(field(scrub, "up_to_status_id")),
            })
        } else if let Some(tweet) = input.find("status_withheld") {
            Ok(StreamMessage::StatusWithheld {
                status_id: try!(field(tweet, "id")),
                user_id: try!(field(tweet, "user_id")),
                withheld_in_countries: try!(field(tweet, "withheld_in_countries")),
            })
        } else if let Some(user) = input.find("user_withheld") {
            Ok(StreamMessage::UserWithheld {
                user_id: try!(field(user, "id")),
                withheld_in_countries: try!(field(user, "withheld_in_countries")),
            })
        } else if let Some(err) = input.find("disconnect") {
            Ok(StreamMessage::Disconnect(try!(field(err, "code")), try!(field(err, "reason"))))
        } else if let Some(friends) = input.find("friends") {
            Ok(StreamMessage::FriendList(try!(Vec::<u64>::from_json(friends))))
        } else if let Some(dm) = input.find("direct_message") {
            Ok(StreamMessage::DirectMessage(try!(DirectMessage::from_json(dm))))
        } else if let Ok(tweet) = Tweet::from_json(input) {
            Ok(StreamMessage::Tweet(tweet))
        } else {
            Ok(StreamMessage::Unknown(input.clone()))
        }
    }

    fn from_str(input: &str) -> Result<Self, error::Error> {
        if input.trim().is_empty() {
            Ok(StreamMessage::Ping)
        } else {
            let json = try!(json::Json::from_str(input.trim()));

            StreamMessage::from_json(&json)
        }
    }
}

/// A `Stream` that represents a connection to the Twitter Streaming API.
pub struct TwitterStream {
    buf: Vec<u8>,
    handle: Handle,
    request: Option<Request>,
    response: Option<FutureResponse>,
    body: Option<Body>,
}

impl TwitterStream {
    fn new(handle: &Handle, request: Request) -> TwitterStream {
        TwitterStream {
            buf: vec![],
            handle: handle.clone(),
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
            self.response = Some(try!(get_response(&self.handle, req)));
        }

        if let Some(mut resp) = self.response.take() {
            match resp.poll() {
                Err(e) => return Err(e.into()),
                Ok(Async::NotReady) => {
                    self.response = Some(resp);
                    return Ok(Async::NotReady);
                },
                Ok(Async::Ready(resp)) => {
                    let status = resp.status();
                    if !status.is_success() {
                        //TODO: should i try to pull the response regardless?
                        return Err(error::Error::BadStatus(status));
                    }

                    self.body = Some(resp.body());
                },
            }
        }

        if let Some(mut body) = self.body.take() {
            loop {
                match body.poll() {
                    Err(e) => {
                        self.body = Some(body);
                        return Err(e.into());
                    },
                    Ok(Async::NotReady) => {
                        self.body = Some(body);
                        return Ok(Async::NotReady);
                    },
                    Ok(Async::Ready(None)) => {
                        //TODO: introduce a new error for this?
                        return Err(error::Error::FutureAlreadyCompleted);
                    },
                    Ok(Async::Ready(Some(chunk))) => {
                        self.buf.extend(&*chunk);

                        if let Some(pos) = self.buf.windows(2).position(|w| w == b"\r\n") {
                            self.body = Some(body);
                            let pos = pos + 2;
                            let resp = if let Ok(msg_str) = std::str::from_utf8(&self.buf[..pos]) {
                                StreamMessage::from_str(msg_str)
                            } else {
                                Err(io::Error::new(io::ErrorKind::InvalidData,
                                                   "stream did not contain valid UTF-8").into())
                            };

                            self.buf.drain(..pos);
                            return Ok(Async::Ready(Some(try!(resp))));
                        }
                    },
                }
            }
        } else {
            Err(error::Error::FutureAlreadyCompleted)
        }
    }
}

/// Opens a `TwitterStream` to the authenticated user's home stream.
pub fn user(handle: &Handle, token: &Token) -> TwitterStream {
    let req = auth::get(links::stream::USER, token, None);

    TwitterStream::new(handle, req)
}
