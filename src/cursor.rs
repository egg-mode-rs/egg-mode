// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Types and traits to navigate cursored collections.
//!
//! Much of this module can be considered an implementation detail; the main intended entry point
//! to this code is `CursorIter`, and that can just be used as a stream to ignore the rest of this
//! module. The rest of it is available to make sure consumers of the API can understand precisely
//! what types come out of functions that return `CursorIter`.

use hyper::{Body, Request};
use futures::FutureExt;
use futures::stream::{self, Stream, StreamExt, TryStreamExt};
use serde::{de::DeserializeOwned, Deserialize};
use std::collections::HashMap;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::common::*;
use crate::error::{self, Result};
use crate::{auth, direct, list, user};

// TODO: refactor the Cursor trait into a CursorItem trait to echo ActivityCursor

///Trait to generalize over paginated views of API results.
///
///Types that implement Cursor are used as intermediate steps in [`CursorIter`][]'s Stream
///implementation, to properly load the data from Twitter. Most of the time you don't need to deal
///with Cursor structs directly, but you can get them via `CursorIter`'s manual paging
///functionality.
///
///[`CursorIter`]: struct.CursorIter.html
pub trait Cursor {
    ///What type is being returned by the API call?
    type Item;

    ///Returns a numeric reference to the previous page of results.
    fn previous_cursor_id(&self) -> i64;
    ///Returns a numeric reference to the next page of results.
    fn next_cursor_id(&self) -> i64;
    ///Unwraps the cursor, returning the collection of results from inside.
    fn into_inner(self) -> Vec<Self::Item>;
}

///Represents a single-page view into a list of users.
///
///This type is intended to be used in the background by [`CursorIter`][] to hold an intermediate
///list of users to iterate over. See that struct's documentation for details.
///
///[`CursorIter`]: struct.CursorIter.html
#[derive(Deserialize)]
pub struct UserCursor {
    ///Numeric reference to the previous page of results.
    pub previous_cursor: i64,
    ///Numeric reference to the next page of results.
    pub next_cursor: i64,
    ///The list of users in this page of results.
    pub users: Vec<user::TwitterUser>,
}

impl Cursor for UserCursor {
    type Item = user::TwitterUser;

    fn previous_cursor_id(&self) -> i64 {
        self.previous_cursor
    }

    fn next_cursor_id(&self) -> i64 {
        self.next_cursor
    }

    fn into_inner(self) -> Vec<Self::Item> {
        self.users
    }
}

///Represents a single-page view into a list of IDs.
///
///This type is intended to be used in the background by [`CursorIter`][] to hold an intermediate
///list of IDs to iterate over. See that struct's documentation for details.
///
///[`CursorIter`]: struct.CursorIter.html
#[derive(Deserialize)]
pub struct IDCursor {
    ///Numeric reference to the previous page of results.
    pub previous_cursor: i64,
    ///Numeric reference to the next page of results.
    pub next_cursor: i64,
    ///The list of user IDs in this page of results.
    pub ids: Vec<u64>,
}

impl Cursor for IDCursor {
    type Item = u64;

    fn previous_cursor_id(&self) -> i64 {
        self.previous_cursor
    }

    fn next_cursor_id(&self) -> i64 {
        self.next_cursor
    }

    fn into_inner(self) -> Vec<Self::Item> {
        self.ids
    }
}

///Represents a single-page view into a list of lists.
///
///This type is intended to be used in the background by [`CursorIter`][] to hold an intermediate
///list of lists to iterate over. See that struct's documentation for details.
///
///[`CursorIter`]: struct.CursorIter.html
#[derive(Deserialize)]
pub struct ListCursor {
    ///Numeric reference to the previous page of results.
    pub previous_cursor: i64,
    ///Numeric reference to the next page of results.
    pub next_cursor: i64,
    ///The list of lists in this page of results.
    pub lists: Vec<list::List>,
}

impl Cursor for ListCursor {
    type Item = list::List;

    fn previous_cursor_id(&self) -> i64 {
        self.previous_cursor
    }

    fn next_cursor_id(&self) -> i64 {
        self.next_cursor
    }

    fn into_inner(self) -> Vec<Self::Item> {
        self.lists
    }
}

/// Represents a paginated list of results, such as the users who follow a specific user or the
/// lists owned by that user.
///
/// This struct is given by several methods in this library, whenever Twitter would return a
/// cursored list of items. It implements the `Stream` trait, loading items in batches so that
/// several can be immedately returned whenever a single network call completes.
///
/// ```rust,no_run
/// # use egg_mode::Token;
/// use futures::{StreamExt, TryStreamExt};
///
/// # #[tokio::main]
/// # async fn main() {
/// # let token: Token = unimplemented!();
/// egg_mode::user::followers_of("rustlang", &token).take(10).try_for_each(|resp| {
///     println!("{}", resp.screen_name);
///     futures::future::ok(())
/// }).await.unwrap();
/// # }
/// ```
///
/// You can even collect the results, letting you get one set of rate-limit information for the
/// entire search setup:
///
/// ```rust,no_run
/// # use egg_mode::Token;
/// # #[tokio::main]
/// # async fn main() {
/// # let token: Token = unimplemented!();
/// use futures::{StreamExt, TryStreamExt};
/// use egg_mode::Response;
/// use egg_mode::user::TwitterUser;
/// use egg_mode::error::Result;
///
/// // Because Streams don't have a FromIterator adaptor, we load all the responses first, then
/// // collect them into the final Vec
/// let names: Result<Vec<TwitterUser>> =
///     egg_mode::user::followers_of("rustlang", &token)
///         .take(10)
///         .map_ok(|r| r.response)
///         .try_collect::<Vec<_>>()
///         .await;
/// # }
/// ```
///
/// `CursorIter` has an adaptor of its own, `with_page_size`, that you can use before consuming it.
/// `with_page_size` will let you set how many users are pulled in with a single network call.
/// Calling it after starting iteration will clear any current results.
///
/// (A note about `with_page_size`/`page_size`: While the `CursorIter` struct always has this method
/// and field available, not every cursored call supports changing page size. Check the individual
/// method documentation for notes on what page sizes are allowed.)
///
/// The `Stream` implementation yields `Response<T::Item>` on a successful iteration, and `Error`
/// for errors, so network errors, rate-limit errors and other issues are passed directly through
/// in `poll()`. The `Stream` implementation will allow you to poll again after an error to
/// re-initiate the late network call; this way, you can wait for your network connection to return
/// or for your rate limit to refresh and try again with the same state.
///
/// ## Manual paging
///
/// The `Stream` implementation works by loading in a page of results (with size set by the
/// method's default or by `with_page_size`/the `page_size` field) when it's polled, and serving
/// the individual elements from that locally-cached page until it runs out. This can be nice, but
/// it also means that your only warning that something involves a network call is that the stream
/// returns `Poll::Pending`, by which time the network call has already started. If you want
/// to know that ahead of time, that's where the `call()` method comes in. By using `call()`, you
/// can get the cursor struct directly from Twitter. With that you can iterate over the results and
/// page forward and backward as needed:
///
/// ```rust,no_run
/// # use egg_mode::Token;
/// # #[tokio::main]
/// # async fn main() {
/// # let token: Token = unimplemented!();
/// let mut list = egg_mode::user::followers_of("rustlang", &token).with_page_size(20);
/// let resp = list.call().await.unwrap();
///
/// for user in resp.response.users {
///     println!("{} (@{})", user.name, user.screen_name);
/// }
///
/// list.next_cursor = resp.response.next_cursor;
/// let resp = list.call().await.unwrap();
///
/// for user in resp.response.users {
///     println!("{} (@{})", user.name, user.screen_name);
/// }
/// # }
/// ```
#[must_use = "cursor iterators are lazy and do nothing unless consumed"]
pub struct CursorIter<T>
where
    T: Cursor + DeserializeOwned,
{
    link: &'static str,
    token: auth::Token,
    params_base: Option<ParamList>,
    ///The number of results returned in one network call.
    ///
    ///Certain calls set their own minimums and maximums for what this value can be. Furthermore,
    ///some calls don't allow you to set the size of the pages at all. Refer to the individual
    ///methods' documentation for specifics.
    pub page_size: Option<i32>,
    ///Numeric reference to the previous page of results. A value of zero indicates that the
    ///current page of results is the first page of the cursor.
    ///
    ///This value is intended to be automatically set and used as part of this struct's Iterator
    ///implementation. It is made available for those who wish to manually manage network calls and
    ///pagination.
    pub previous_cursor: i64,
    ///Numeric reference to the next page of results. A value of zero indicates that the current
    ///page of results is the last page of the cursor.
    ///
    ///This value is intended to be automatically set and used as part of this struct's Iterator
    ///implementation. It is made available for those who wish to manually manage network calls and
    ///pagination.
    pub next_cursor: i64,
    loader: Option<FutureResponse<T>>,
    iter: Option<Box<dyn Iterator<Item = Response<T::Item>> + Send>>,
}

impl<T> CursorIter<T>
where
    T: Cursor + DeserializeOwned,
{
    ///Sets the number of results returned in a single network call.
    ///
    ///Certain calls set their own minimums and maximums for what this value can be. Furthermore,
    ///some calls don't allow you to set the size of the pages at all. Refer to the individual
    ///methods' documentation for specifics. If this method is called for a response that does not
    ///accept changing the page size, no change to the underlying struct will occur.
    ///
    ///Calling this function will invalidate any current results, if any were previously loaded.
    pub fn with_page_size(self, page_size: i32) -> CursorIter<T> {
        if self.page_size.is_some() {
            CursorIter {
                page_size: Some(page_size),
                previous_cursor: -1,
                next_cursor: -1,
                loader: None,
                iter: None,
                ..self
            }
        } else {
            self
        }
    }

    ///Loads the next page of results.
    ///
    ///This is intended to be used as part of this struct's Iterator implementation. It is provided
    ///as a convenience for those who wish to manage network calls and pagination manually.
    pub fn call(&self) -> impl Future<Output = Result<Response<T>>> {
        let params = ParamList::from(self.params_base.as_ref().cloned().unwrap_or_default())
            .add_param("cursor", self.next_cursor.to_string())
            .add_opt_param("count", self.page_size.map_string());

        let req = get(self.link, &self.token, Some(&params));
        request_with_json_response(req)
    }

    ///Creates a new instance of CursorIter, with the given parameters and empty initial results.
    ///
    ///This is essentially an internal infrastructure function, not meant to be used from consumer
    ///code.
    pub(crate) fn new(
        link: &'static str,
        token: &auth::Token,
        params_base: Option<ParamList>,
        page_size: Option<i32>,
    ) -> CursorIter<T> {
        CursorIter {
            link: link,
            token: token.clone(),
            params_base: params_base,
            page_size: page_size,
            previous_cursor: -1,
            next_cursor: -1,
            loader: None,
            iter: None,
        }
    }
}

impl<T> Stream for CursorIter<T>
where
    T: Cursor + DeserializeOwned + 'static,
    T::Item: Unpin + Send,
{
    type Item = Result<Response<T::Item>>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Option<Self::Item>> {
        if let Some(mut fut) = self.loader.take() {
            match Pin::new(&mut fut).poll(cx) {
                Poll::Pending => {
                    self.loader = Some(fut);
                    return Poll::Pending;
                }
                Poll::Ready(Ok(resp)) => {
                    self.previous_cursor = resp.previous_cursor_id();
                    self.next_cursor = resp.next_cursor_id();

                    let resp = Response::map(resp, |r| r.into_inner());
                    let rate = resp.rate_limit_status;

                    let mut iter = Box::new(resp.response.into_iter().map(move |item| Response {
                        rate_limit_status: rate,
                        response: item,
                    }));
                    let first = iter.next();
                    self.iter = Some(iter);

                    match first {
                        Some(item) => return Poll::Ready(Some(Ok(item))),
                        None => return Poll::Ready(None),
                    }
                }
                Poll::Ready(Err(e)) => return Poll::Ready(Some(Err(e))),
            }
        }

        if let Some(ref mut results) = self.iter {
            if let Some(item) = results.next() {
                return Poll::Ready(Some(Ok(item)));
            } else if self.next_cursor == 0 {
                return Poll::Ready(None);
            }
        }

        self.loader = Some(Box::pin(self.call()));
        self.poll_next(cx)
    }
}

/// Represents a page of results in an `ActivityCursorIter`.
///
/// This type is used internally by the `ActivityCursorIter` implementation to collect a page of
/// results.
pub struct ActivityCursor<T> {
    /// String key representing the next page of results, if present.
    pub next_cursor: Option<String>,
    /// The list of items in this page of results.
    pub items: Vec<T>,
}

/// Trait for items which can be returned by `ActivityCursorIter`.
pub trait ActivityItem: Sized {
    /// What is the cursor type that `ActivityCursorIter` can deserialize into?
    type Cursor: DeserializeOwned + Into<ActivityCursor<Self>>;
}

/// Represents a handle to a paginated list of items on the Account Activity API.
///
/// Paginated APIs in the Account Activity API, used by Direct Message endpoints, differ from APIs
/// like `tweet::home_timeline` in that Twitter returns a "cursor" ID instead of paging through
/// results by asking for messages before or after a certain ID. It's not a strict `CursorIter`,
/// though, in that there is no "previous cursor" ID given by Twitter; messages are loaded one-way,
/// from newest to oldest.
///
/// To start using an `ActivityCursorIter`, call a function that returns one to set one up, like
/// `direct::list`. Before starting, you can call `with_page_size` to set how many messages to ask
/// for at once. Then use `start` and `next_page` to load messages one page at a time.
///
/// ```no_run
/// # #[tokio::main]
/// # async fn main() {
/// # let token: egg_mode::Token = unimplemented!();
/// let timeline = egg_mode::direct::list(&token).with_page_size(50);
/// let mut messages = timeline.start().await.unwrap();
///
/// while timeline.next_cursor.is_some() {
///     let next_page = timeline.next_page().await.unwrap();
///     messages.extend(next_page.response);
/// }
/// # }
/// ```
///
/// An adapter is provided which converts a `ActivityCursorIter` into a `futures::stream::Stream`
/// which yields one message at a time and lazily loads each page as needed. As the stream's `Item`
/// is a `Result` which can express the error caused by loading the next page, it also implements
/// `futures::stream::TryStream` as well. The previous example can also be expressed like this:
///
/// ```no_run
/// use egg_mode::Response;
/// use egg_mode::direct::DirectMessage;
/// use futures::stream::TryStreamExt;
/// # #[tokio::main]
/// # async fn main() {
/// # let token: egg_mode::Token = unimplemented!();
/// let timeline = egg_mode::direct::list(&token).with_page_size(50);
/// let messages = timeline.into_stream()
///                        .try_collect::<Vec<Response<DirectMessage>>>()
///                        .await
///                        .unwrap();
/// # }
/// ```
///
/// In addition, an adapter is available for `ActivityCursorIter`s over direct messages which loads
/// all available messages and sorts them into "conversations" between the authenticated user and
/// other users. The `into_conversations` adapter loads all available messages and returns a
/// [`DMConversations`] map after sorting them.
///
/// [`DMConversations`]: ../direct/type.DMConversations.html
pub struct ActivityCursorIter<CursorItem> {
    url: &'static str,
    token: auth::Token,
    /// String key representing the next page of results, if present. If this is `None`, calling
    /// `next_page` will return the first page of results.
    pub next_cursor: Option<String>,
    /// The number of items to request per-page. The default is 20, the maximum is 50.
    pub count: u32,
    /// Whether this `ActivityCursorIter` has loaded a page yet. Used to determine whether a `None`
    /// value in `next_cursor` means that the results haven't been loaded at all, or whether it
    /// means that the results have been loaded in full.
    pub loaded: bool,
    _marker: PhantomData<CursorItem>,
}

impl<CursorItem: ActivityItem> ActivityCursorIter<CursorItem> {
    pub(crate) fn new(url: &'static str, token: &auth::Token) -> Self {
        ActivityCursorIter {
            url,
            token: token.clone(),
            next_cursor: None,
            count: 20,
            loaded: false,
            _marker: PhantomData,
        }
    }

    /// Sets the number of items loaded per page of results as a builder-style constructor.
    ///
    /// Note that calling this function will reset the saved cursor information, causing the next
    /// call to `next_page` to return the first page of results. This is meant to be used when you
    /// first receive the `ActivityCursorIter`.
    pub fn with_page_size(self, count: u32) -> Self {
        ActivityCursorIter {
            count,
            next_cursor: None,
            loaded: false,
            ..self
        }
    }

    /// Clears the saved cursor information on this `ActivityCursorIter`.
    pub fn reset(&mut self) {
        self.next_cursor = None;
        self.loaded = false;
    }

    fn request(&self, cursor: Option<String>) -> Request<Body> {
        let params = ParamList::new()
            .add_param("count", self.count.to_string())
            .add_opt_param("cursor", cursor);

        get(self.url, &self.token, Some(&params))
    }

    /// Loads the next page of messages, setting the `next_cursor` to the one received from
    /// Twitter.
    pub fn next_page<'s>(&'s mut self)
        -> impl Future<Output = Result<Response<Vec<CursorItem>>>> + 's
    {
        let next_cursor = self.next_cursor.take();
        let req = self.request(next_cursor);
        let loader = request_with_json_response(req);
        loader.map(
            move |resp: Result<Response<CursorItem::Cursor>>| {
                let mut resp = Response::into(resp?);
                self.loaded = true;
                self.next_cursor = resp.response.next_cursor.take();
                Ok(Response::map(resp, |curs| curs.items))
            }
        )
    }

    /// Converts this `ActivityCursorIter` into a `Stream` of items, which automatically loads the
    /// next page as needed.
    pub fn into_stream(self)
        -> impl Stream<Item = Result<Response<CursorItem>>>
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

impl ActivityCursorIter<direct::DirectMessage> {
    /// Loads all the direct messages from this `ActivityCursorIter` and sorts them into a
    /// `DMConversations` map.
    ///
    /// This adapter is a convenient way to sort all of a user's messages (from the last 30 days)
    /// into a familiar user-interface pattern of a list of conversations between the authenticated
    /// user and a specific other user. This function first pulls all the available messages, then
    /// sorts them into a set of threads by matching them against which user the authenticated user
    /// is messaging.
    ///
    /// If there are more messages available than can be loaded without hitting the rate limit (15
    /// calls to the `list` endpoint per 15 minutes), then this function will stop once it receives
    /// a rate-limit error and sort the messages it received.
    pub async fn into_conversations(mut self) -> Result<direct::DMConversations> {
        let mut dms: Vec<direct::DirectMessage> = vec![];
        while !self.loaded || self.next_cursor.is_some() {
            match self.next_page().await {
                Ok(page) => dms.extend(page.into_iter().map(|r| r.response)),
                Err(error::Error::RateLimit(_)) => break,
                Err(e) => return Err(e),
            }
        }
        let mut conversations = HashMap::new();
        let me_id = if let Some(dm) = dms.first() {
            if dm.source_app.is_some() {
                // since the source app info is only populated when the authenticated user sent the
                // message, we know that this message was sent by the authenticated user
                dm.sender_id
            } else {
                dm.recipient_id
            }
        } else {
            // no messages, nothing to sort
            return Ok(conversations);
        };

        for dm in dms {
            let entry = match (dm.sender_id == me_id, dm.recipient_id == me_id) {
                (true, true) => {
                    // if the sender and recipient are the same - and they match the authenticated
                    // user - then it's the listing of "messages to self"
                    conversations.entry(me_id).or_default()
                }
                (true, false) => {
                    conversations.entry(dm.recipient_id).or_default()
                }
                (false, true) => {
                    conversations.entry(dm.sender_id).or_default()
                }
                (false, false) => {
                    return Err(error::Error::InvalidResponse(
                            "messages activity contains disjoint conversations",
                            None));
                }
            };
            entry.push(dm);
        }

        Ok(conversations)
    }
}
