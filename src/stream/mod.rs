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
use error;
use links;
use tweet::Tweet;

use common::*;

/// Represents the kinds of messages that can be sent over Twitter's Streaming API.
#[derive(Debug)]
pub enum StreamMessage {
    /// A blank line, sent periodically to keep the connection alive.
    Ping,
    /// A new tweet.
    ///
    /// Note that the `entities` inside the `user` field will be empty for tweets received via the
    /// Streaming API.
    Tweet(Tweet),
    /// An unhandled message payload.
    ///
    /// Twitter can add new streaming messages to the API, and egg-mode includes them here so that
    /// they can be used before egg-mode has a chance to handle them.
    Unknown(json::Json),
}

impl FromJson for StreamMessage {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if let Some(_event) = input.find("event") {
        } else {
            if let Ok(tweet) = Tweet::from_json(input) {
                return Ok(StreamMessage::Tweet(tweet));
            }
        }
        Ok(StreamMessage::Unknown(input.clone()))
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
