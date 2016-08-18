use std::collections::HashMap;
use std::marker::PhantomData;
use super::*;
use auth;
use error;

///Trait to generalize over paginated views of API results.
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

///Represents a paginated list of results, such as the users who follow a specific user or the
///lists owned by that user.
///
///This struct is returned by various functions and is meant to be used as an iterator. This means
///that all the standard iterator adaptors can be used to work with the results:
///
///```rust,no_run
///# let con_token = egg_mode::Token::new("", "");
///# let access_token = egg_mode::Token::new("", "");
///for name in egg_mode::user::followers_of("rustlang", &con_token, &access_token)
///                                        .map(|u| u.unwrap().response.screen_name).take(10) {
///    println!("{}", name);
///}
///```
///
///You can even collect the results, letting you get one set of rate-limit information for the
///entire search setup:
///
///```rust,no_run
///# let con_token = egg_mode::Token::new("", "");
///# let access_token = egg_mode::Token::new("", "");
///use egg_mode::Response;
///use egg_mode::user::TwitterUser;
///use egg_mode::error::Error;
///
///let names: Result<Response<Vec<TwitterUser>>, Error> =
///    egg_mode::user::followers_of("rustlang", &con_token, &access_token).take(10).collect();
///```
///
///`CursorIter` has a couple adaptors of its own that you can use before consuming it.
///`with_page_size` will let you set how many users are pulled in with a single network call, and
///`start_at_page` lets you start your search at a specific page. Calling either of these after
///starting iteration will clear any current results.
///
///(A note about `with_page_size`/`page_size`: While the CursorIter struct always has this method
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
///# let con_token = egg_mode::Token::new("", "");
///# let access_token = egg_mode::Token::new("", "");
///let mut list = egg_mode::user::followers_of("rustlang", &con_token, &access_token).with_page_size(20);
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
    where T: Cursor + FromJson
{
    link: &'static str,
    con_token: &'a auth::Token<'a>,
    access_token: &'a auth::Token<'a>,
    user_id: Option<UserID<'a>>,
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
    iter: Option<ResponseIter<T::Item>>,
    _marker: PhantomData<T>,
}

impl<'a, T> CursorIter<'a, T>
    where T: Cursor + FromJson
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
                link: self.link,
                con_token: self.con_token,
                access_token: self.access_token,
                user_id: self.user_id,
                page_size: Some(page_size),
                previous_cursor: -1,
                next_cursor: -1,
                iter: None,
                _marker: self._marker,
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
    pub fn call(&self) -> Result<Response<T>, error::Error> {
        let mut params = HashMap::new();
        if let Some(ref id) = self.user_id {
            add_name_param(&mut params, id);
        }
        add_param(&mut params, "cursor", self.next_cursor.to_string());
        if let Some(count) = self.page_size {
            add_param(&mut params, "count", count.to_string());
        }

        let mut resp = try!(auth::get(self.link, self.con_token, self.access_token, Some(&params)));

        parse_response(&mut resp)
    }

    ///Creates a new instance of CursorIter, with the given parameters and empty initial results.
    pub fn new(link: &'static str, con_token: &'a auth::Token, access_token: &'a auth::Token,
               user_id: Option<UserID<'a>>, page_size: Option<i32>) -> CursorIter<'a, T> {
        CursorIter {
            link: link,
            con_token: con_token,
            access_token: access_token,
            user_id: user_id,
            page_size: page_size,
            previous_cursor: -1,
            next_cursor: -1,
            iter: None,
            _marker: PhantomData,
        }
    }
}

impl<'a, T> Iterator for CursorIter<'a, T>
    where T: Cursor + FromJson
{
    type Item = Result<Response<T::Item>, error::Error>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ref mut results) = self.iter {
            if let Some(item) = results.next() {
                return Some(Ok(item));
            }
            else if self.next_cursor == 0 {
                return None;
            }
        }

        match self.call() {
            Ok(resp) => {
                self.previous_cursor = resp.response.previous_cursor_id();
                self.next_cursor = resp.response.next_cursor_id();

                let resp = Response {
                    rate_limit: resp.rate_limit,
                    rate_limit_remaining: resp.rate_limit_remaining,
                    rate_limit_reset: resp.rate_limit_reset,
                    response: resp.response.into_inner(),
                };

                let mut iter = resp.into_iter();
                let first = iter.next();
                self.iter = Some(iter);

                match first {
                    Some(item) => Some(Ok(item)),
                    None => None,
                }
            },
            Err(err) => Some(Err(err)),
        }
    }
}
