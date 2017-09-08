// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Types and traits to navigate cursored collections.
//!
//! Much of this module can be considered an implementation detail; the main intended entry point
//! to this code is `CursorIter`, and that can just be used as an iterator to ignore the rest of
//! this module. The rest of it is available to make sure consumers of the API can understand
//! precisely what types come out of functions that return `CursorIter`.

use futures::{Future, Stream, Poll, Async};
use rustc_serialize::json;
use common::*;
use auth;
use error;
use error::Error::InvalidResponse;
use list;
use user;

///Trait to generalize over paginated views of API results.
///
///Types that implement Cursor are used as intermediate steps in [`CursorIter`][]'s Iterator
///implementation, to properly load the data from Twitter. Most of the time you don't need to deal
///with Cursor structs directly, but you can get them via `CursorIter`'s manual paging functionality.
///
///[`CursorIter`]: struct.CursorIter.html
pub trait Cursor {
    ///What type is being returned by the API call?
    type Item;

    ///Returns a numeric reference to the previous page of results.
    fn previous_cursor_id(&self) -> i64;
    ///Returns a numeric reference to the next page of results.
    fn next_cursor_id(&self) -> i64;
    ///Consumes the cursor and returns the collection of results from inside.
    fn into_inner(self) -> Vec<Self::Item>;
}

///Represents a single-page view into a list of users.
///
///This type is intended to be used in the background by [`CursorIter`][] to hold an intermediate
///list of users to iterate over. See that struct's documentation for details.
///
///[`CursorIter`]: struct.CursorIter.html
pub struct UserCursor {
    ///Numeric reference to the previous page of results.
    pub previous_cursor: i64,
    ///Numeric reference to the next page of results.
    pub next_cursor: i64,
    ///The list of users in this page of results.
    pub users: Vec<user::TwitterUser>,
}

impl FromJson for UserCursor {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse("UserCursor received json that wasn't an object", Some(input.to_string())));
        }

        field_present!(input, previous_cursor);
        field_present!(input, next_cursor);
        field_present!(input, users);

        Ok(UserCursor {
            previous_cursor: try!(field(input, "previous_cursor")),
            next_cursor: try!(field(input, "next_cursor")),
            users: try!(field(input, "users")),
        })
    }
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

///Represents a single-page view into a list of user IDs.
///
///This type is intended to be used in the background by [`CursorIter`][] to hold an intermediate
///list of users to iterate over. See that struct's documentation for details.
///
///[`CursorIter`]: struct.CursorIter.html
pub struct IDCursor {
    ///Numeric reference to the previous page of results.
    pub previous_cursor: i64,
    ///Numeric reference to the next page of results.
    pub next_cursor: i64,
    ///The list of user IDs in this page of results.
    pub ids: Vec<u64>,
}

impl FromJson for IDCursor {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse("IDCursor received json that wasn't an object", Some(input.to_string())));
        }

        field_present!(input, previous_cursor);
        field_present!(input, next_cursor);
        field_present!(input, ids);

        Ok(IDCursor {
            previous_cursor: try!(field(input, "previous_cursor")),
            next_cursor: try!(field(input, "next_cursor")),
            ids: try!(field(input, "ids")),
        })
    }
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
pub struct ListCursor {
    ///Numeric reference to the previous page of results.
    pub previous_cursor: i64,
    ///Numeric reference to the next page of results.
    pub next_cursor: i64,
    ///The list of lists in this page of results.
    pub lists: Vec<list::List>,
}

impl FromJson for ListCursor {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse("ListCursor received json that wasn't an object", Some(input.to_string())));
        }

        field_present!(input, previous_cursor);
        field_present!(input, next_cursor);
        field_present!(input, lists);

        Ok(ListCursor {
            previous_cursor: try!(field(input, "previous_cursor")),
            next_cursor: try!(field(input, "next_cursor")),
            lists: try!(field(input, "lists")),
        })
    }
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

///Represents a paginated list of results, such as the users who follow a specific user or the
///lists owned by that user.
///
///This struct is returned by various functions and is meant to be used as an iterator. This means
///that all the standard iterator adaptors can be used to work with the results:
///
///```rust,no_run
///# let token = egg_mode::Token::Access {
///#     consumer: egg_mode::KeyPair::new("", ""),
///#     access: egg_mode::KeyPair::new("", ""),
///# };
///for name in egg_mode::user::followers_of("rustlang", &token)
///                                        .map(|u| u.unwrap().response.screen_name).take(10) {
///    println!("{}", name);
///}
///```
///
///You can even collect the results, letting you get one set of rate-limit information for the
///entire search setup:
///
///```rust,no_run
///# let token = egg_mode::Token::Access {
///#     consumer: egg_mode::KeyPair::new("", ""),
///#     access: egg_mode::KeyPair::new("", ""),
///# };
///use egg_mode::Response;
///use egg_mode::user::TwitterUser;
///use egg_mode::error::Error;
///
///let names: Result<Response<Vec<TwitterUser>>, Error> =
///    egg_mode::user::followers_of("rustlang", &token).take(10).collect();
///```
///
///`CursorIter` has a couple adaptors of its own that you can use before consuming it.
///`with_page_size` will let you set how many users are pulled in with a single network call, and
///`start_at_page` lets you start your search at a specific page. Calling either of these after
///starting iteration will clear any current results.
///
///(A note about `with_page_size`/`page_size`: While the `CursorIter` struct always has this method
///and field available, not every cursored call supports changing page size. Check the individual
///method documentation for notes on what page sizes are allowed.)
///
///The type returned by the iterator is `Result<Response<T::Item>, Error>`, so network errors,
///rate-limit errors and other issues are passed directly through to `next()`. This also means that
///getting an error while iterating doesn't mean you're at the end of the list; you can wait for
///the network connection to return or for the rate limit to refresh before trying again.
///
///## Manual paging
///
///The iterator works by lazily loading a page of results at a time (with size set by
///`with_page_size` or by directly assigning `page_size` for applicable calls) in the background
///whenever you ask for the next result. This can be nice, but it also means that you can lose
///track of when your loop will block for the next page of results. This is where the extra fields
///and methods on `UserSearch` come in. By using `call()`, you can get the cursor struct directly
///from Twitter.  With that you can iterate over the results and page forward and backward as
///needed:
///
///```rust,no_run
///# let token = egg_mode::Token::Access {
///#     consumer: egg_mode::KeyPair::new("", ""),
///#     access: egg_mode::KeyPair::new("", ""),
///# };
///let mut list = egg_mode::user::followers_of("rustlang", &token).with_page_size(20);
///let resp = list.call().unwrap();
///
///for user in resp.response.users {
///    println!("{} (@{})", user.name, user.screen_name);
///}
///
///list.next_cursor = resp.response.next_cursor;
///let resp = list.call().unwrap();
///
///for user in resp.response.users {
///    println!("{} (@{})", user.name, user.screen_name);
///}
///```
#[must_use = "cursor iterators are lazy and do nothing unless consumed"]
pub struct CursorIter<'a, T>
    where T: Cursor + FromJson + 'a
{
    link: &'static str,
    token: &'a auth::Token,
    handle: &'a Handle,
    params_base: Option<ParamList<'a>>,
    ///The number of results returned in one network call.
    ///
    ///Certain calls set their own minimums and maximums for what this value can be. Furthermore,
    ///some calls don't allow you to set the size of the pages at all. Refer to the individual
    ///methods' documentation for specifics.
    pub page_size: Option<i32>,
    ///Numeric reference to the previous page of results.
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
    loader: Option<FutureResponse<'a, T>>,
    iter: Option<ResponseIter<T::Item>>,
}

impl<'a, T> CursorIter<'a, T>
    where T: Cursor + FromJson + 'a
{
    ///Sets the number of results returned in a single network call.
    ///
    ///Certain calls set their own minimums and maximums for what this value can be. Furthermore,
    ///some calls don't allow you to set the size of the pages at all. Refer to the individual
    ///methods' documentation for specifics. If this method is called for a response that does not
    ///accept changing the page size, no change to the underlying struct will occur.
    ///
    ///Calling this function will invalidate any current results, if any were previously loaded.
    pub fn with_page_size(self, page_size: i32) -> CursorIter<'a, T> {
        if self.page_size.is_some() {
            CursorIter {
                page_size: Some(page_size),
                previous_cursor: -1,
                next_cursor: -1,
                loader: None,
                iter: None,
                ..self
            }
        }
        else {
            self
        }
    }

    ///Loads the next page of results.
    ///
    ///This is intended to be used as part of this struct's Iterator implementation. It is provided
    ///as a convenience for those who wish to manage network calls and pagination manually.
    pub fn call(&self) -> FutureResponse<'a, T> {
        let mut params = self.params_base.as_ref().cloned().unwrap_or_default();

        add_param(&mut params, "cursor", self.next_cursor.to_string());
        if let Some(count) = self.page_size {
            add_param(&mut params, "count", count.to_string());
        }

        let req = auth::get(self.link, self.token, Some(&params));

        make_parsed_future(self.handle, req)
    }

    ///Creates a new instance of CursorIter, with the given parameters and empty initial results.
    ///
    ///This is essentially an internal infrastructure function, not meant to be used from consumer
    ///code.
    #[doc(hidden)]
    pub fn new(link: &'static str, token: &'a auth::Token, handle: &'a Handle,
               params_base: Option<ParamList<'a>>, page_size: Option<i32>) -> CursorIter<'a, T> {
        CursorIter {
            link: link,
            token: token,
            handle: handle,
            params_base: params_base,
            page_size: page_size,
            previous_cursor: -1,
            next_cursor: -1,
            loader: None,
            iter: None,
        }
    }
}

impl<'a, T> Stream for CursorIter<'a, T>
    where T: Cursor + FromJson + 'a
{
    type Item = Response<T::Item>;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        if let Some(mut fut) = self.loader.take() {
            match fut.poll() {
                Ok(Async::NotReady) => {
                    self.loader = Some(fut);
                    return Ok(Async::NotReady);
                }
                Ok(Async::Ready(resp)) => {
                    self.previous_cursor = resp.previous_cursor_id();
                    self.next_cursor = resp.next_cursor_id();

                    let resp = Response::map(resp, |r| r.into_inner());

                    let mut iter = resp.into_iter();
                    let first = iter.next();
                    self.iter = Some(iter);

                    match first {
                        Some(item) => return Ok(Async::Ready(Some(item))),
                        None => return Ok(Async::Ready(None)),
                    }
                }
                Err(e) => return Err(e),
            }
        }

        if let Some(ref mut results) = self.iter {
            if let Some(item) = results.next() {
                return Ok(Async::Ready(Some(item)));
            }
            else if self.next_cursor == 0 {
                return Ok(Async::Ready(None));
            }
        }

        self.loader = Some(self.call());
        self.poll()
    }
}
