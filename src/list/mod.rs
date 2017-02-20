//! Structs and functions for working with lists.

use common::*;

use std::collections::HashMap;

use rustc_serialize::json;
use chrono;

use auth;
use cursor;
use cursor::CursorIter;
use user;
use error::Error::InvalidResponse;
use links;
use tweet;
use error;

mod fun;
pub use self::fun::*;

///Convenience enum to refer to a list via its owner and name or via numeric ID.
pub enum ListID<'a> {
    ///Referring via the list's owner and its "slug" or name.
    Slug(user::UserID<'a>, &'a str),
    ///Referring via the list's numeric ID.
    ID(u64)
}

impl<'a> ListID<'a> {
    ///Make a new `ListID` by supplying its owner and name.
    pub fn from_slug<T: Into<user::UserID<'a>>>(owner: T, list_name: &'a str) -> ListID<'a> {
        ListID::Slug(owner.into(), list_name)
    }

    ///Make a new `ListID` by supplying its numeric ID.
    pub fn from_id(list_id: u64) -> ListID<'a> {
        ListID::ID(list_id)
    }
}

///Represents the metadata for a list.
#[derive(Clone)]
pub struct ListInfo {
    ///The name of the list.
    pub name: String,
    ///The "slug" of a list, that can be combined with its creator's `UserID` to refer to the list.
    pub slug: String,
    ///The numeric ID of the list.
    pub id: u64,
    ///The number of accounts "subscribed" to the list, for whom it will appear in their collection
    ///of available lists.
    pub subscriber_count: u64,
    ///The number of accounts added to the list.
    pub member_count: u64,
    ///The full name of the list, preceded by `@`, that can be used to link to the list as part of
    ///a tweet, direct message, or other place on Twitter where @mentions are parsed.
    pub full_name: String,
    ///The description of the list, as entered by its creator.
    pub description: String,
    ///The full name of the list, preceded by `/`, that can be preceded with `https://twitter.com`
    ///to create a link to the list.
    pub uri: String,
    ///UTC timestamp of when the list was created.
    pub created_at: chrono::DateTime<chrono::UTC>,
}

impl FromJson for ListInfo {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse("List received json that wasn't an object", Some(input.to_string())));
        }

        field_present!(input, uri);
        field_present!(input, full_name);
        field_present!(input, description);
        field_present!(input, slug);
        field_present!(input, name);
        field_present!(input, subscriber_count);
        field_present!(input, member_count);
        field_present!(input, id);
        field_present!(input, name);
        field_present!(input, created_at);

        Ok(ListInfo {
            created_at: try!(field(input, "created_at")),
            name: try!(field(input, "name")),
            slug: try!(field(input, "slug")),
            id: try!(field(input, "id")),
            subscriber_count: try!(field(input, "subscriber_count")),
            member_count: try!(field(input, "member_count")),
            full_name: try!(field(input, "full_name")),
            description: try!(field(input, "description")),
            uri: try!(field(input, "uri"))
        })
    }
}

enum ListIterType {
    Ownerships,
    Subscriptions,
    Memberships,
    Lists
}

pub struct ListIter<'a> {
    token: &'a auth::Token<'a>,
    query: &'a user::UserID<'a>,
    ///The current page of results being returned, starting at 1.
    pub page_num: i32,
    ///The number of user records per page of results. Defaults to 10, maximum of 20.
    pub page_size: i32,
    current_results: Option<ResponseIter<ListInfo>>,
    iter_type: ListIterType
}

impl<'a> ListIter<'a> {
    ///Sets the page size used for the search query.
    ///
    ///Calling this will invalidate any current search results, making the next call to `next()`
    ///perform a network call.
    pub fn with_page_size(self, page_size: i32) -> Self {
        ListIter {
            page_size: page_size,
            current_results: None,
            ..self
        }
    }

    ///Sets the starting page number for the search query.
    ///
    ///Calling this will invalidate any current search results, making the next call to `next()`
    ///perform a network call.
    pub fn start_at_page(self, page_num: i32) -> Self {
        ListIter {
            page_num: page_num,
            current_results: None,
            ..self
        }
    }

    ///Performs the search for the current page of results.
    ///
    ///This will automatically be called if you use the `UserSearch` as an iterator. This method is
    ///made public for convenience if you want to manage the pagination yourself. Remember to
    ///change `page_num` between calls.
    fn call_ownerships(&self) -> WebResponse<Vec<ListInfo>> {
        let mut params = HashMap::new();
        add_param(&mut params, "cursor", self.page_num.to_string());
        add_param(&mut params, "count", self.page_size.to_string());
        match self.query {
            &user::UserID::ID(id) => {
                add_param(&mut params, "user_id", id.to_string());
            },
            &user::UserID::ScreenName(screen_name) => {
                add_param(&mut params, "screen_name", screen_name.to_string());
            }
        };
        let mut resp = try!(auth::get(links::lists::LISTS_OWNERSHIPS, self.token, Some(&params)));
        parse_response(&mut resp)
    }

    ///Performs the search for the current page of results.
    ///
    ///This will automatically be called if you use the `UserSearch` as an iterator. This method is
    ///made public for convenience if you want to manage the pagination yourself. Remember to
    ///change `page_num` between calls.
    fn call_subscriptions(&self) -> WebResponse<Vec<ListInfo>> {
        let mut params = HashMap::new();
        add_param(&mut params, "cursor", self.page_num.to_string());
        add_param(&mut params, "count", self.page_size.to_string());
        match self.query {
            &user::UserID::ID(id) => {
                add_param(&mut params, "user_id", id.to_string());
            },
            &user::UserID::ScreenName(screen_name) => {
                add_param(&mut params, "screen_name", screen_name.to_string());
            }
        };
        let mut resp = try!(auth::get(links::lists::LISTS_SUBSCRIPTIONS, self.token, Some(&params)));
        parse_response(&mut resp)
    }

    ///Performs the search for the current page of results.
    ///
    ///This will automatically be called if you use the `UserSearch` as an iterator. This method is
    ///made public for convenience if you want to manage the pagination yourself. Remember to
    ///change `page_num` between calls.
    fn call_memberships(&self) -> WebResponse<Vec<ListInfo>> {
        let mut params = HashMap::new();
        add_param(&mut params, "cursor", self.page_num.to_string());
        add_param(&mut params, "count", self.page_size.to_string());
        match self.query {
            &user::UserID::ID(id) => {
                add_param(&mut params, "user_id", id.to_string());
            },
            &user::UserID::ScreenName(screen_name) => {
                add_param(&mut params, "screen_name", screen_name.to_string());
            }
        };
        let mut resp = try!(auth::get(links::lists::LISTS_MEMBERSHIPS, self.token, Some(&params)));
        parse_response(&mut resp)
    }

    fn call_lists(&self) -> WebResponse<Vec<ListInfo>> {
        let mut params = HashMap::new();
        add_param(&mut params, "cursor", self.page_num.to_string());
        add_param(&mut params, "count", self.page_size.to_string());
        match self.query {
            &user::UserID::ID(id) => {
                add_param(&mut params, "user_id", id.to_string());
            },
            &user::UserID::ScreenName(screen_name) => {
                add_param(&mut params, "screen_name", screen_name.to_string());
            }
        };
        let mut resp = try!(auth::get(links::lists::LISTS_LIST, self.token, Some(&params)));
        parse_response(&mut resp)
    }

    ///Returns a new UserSearch with the given query and tokens, with the default page size of 10.
    fn new(query: &'a user::UserID<'a>, iter_type: ListIterType, token: &'a auth::Token) -> Self {
        ListIter {
            token: token,
            query: query,
            page_num: 1,
            page_size: 10,
            current_results: None,
            iter_type: iter_type
        }
    }
}

impl<'a> Iterator for ListIter<'a> {
    type Item = WebResponse<ListInfo>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ref mut results) = self.current_results {
            if let Some(user) = results.next() {
                return Some(Ok(user));
            }
            else if (results.len() as i32) < self.page_size {
                return None;
            }
            else {
                self.page_num += 1;
            }
        }

        let x = match self.iter_type {
            ListIterType::Subscriptions => self.call_subscriptions(),
            ListIterType::Ownerships => self.call_ownerships(),
            ListIterType::Memberships => self.call_memberships(),
            ListIterType::Lists => self.call_lists()
        };

        match x {
            Ok(resp) => {
                let mut iter = resp.into_iter();
                let first = iter.next();
                self.current_results = Some(iter);
                match first {
                    Some(list) => Some(Ok(list)),
                    None => None,
                }
            },
            Err(err) => {
                //Invalidate current results so we don't increment the page number again
                self.current_results = None;
                Some(Err(err))
            },
        }
    }
}
