//! Structs and functions for working with lists.
//!
//! A list is a way to group accounts together, either as a way to highlight those accounts, or to
//! view as a subset of or supplement to your main timeline. A list can be viewed in the same way
//! as the user's own timelines, loading in pages with the same [`Timeline`] interface.
//!
//! [`Timeline`]: ../tweet/struct.Timeline.html
//!
//! Lists can be private or public. If a list if public, then other users can view the list's
//! members and the statuses of those members. (Statuses by protected accounts can still only be
//! read by approved followers.)
//!
//! A list has "members", those accounts that are tracked as part of the list. When you call
//! `statuses` on a given list, you're looking at the recent posts by its members. Members can be
//! added and removed either one at a time, or in batches. Protected accounts can only be added to
//! a list if the user is an approved follower of that account.
//!
//! A user can "subscribe" to another user's list as a sort of bookmark. Doing this places the list
//! in the "lists" section of their profile, and in the `subscriptions` and `list` sets available
//! from the API. Note that you don't need to subscribe to a list to view the statuses or users in
//! it; you can do that to any public or personally owned list anyway. Subscribing merely allows
//! quick access in case you want to keep track of a list someone else made.
//!
//! If a list is public, then all metadata about that list is public. Protected accounts can be
//! seen as members of a list, but their statuses are still only visible to approved followers.
//! None of the user-focused queries in this module assume that they're about the authenticated
//! user; they can all be called in reference to any user.
//!
//! ## Types
//!
//! - `List`: This is the list metadata returned from Twitter when requesting information about the
//!   list itself, or when performing some modification to one.
//! - `ListID`: There are two ways to reference a list in the Twitter API: Either via a unique
//!   numeric ID, or with its "slug" combined with a reference to the user who created it. This
//!   enum wraps that distinction into one type that all the methods take when they need to
//!   reference a list like this. See the enum's documentation for details on how to create one.
//! - `ListUpdate`: When updating a list's metadata, all the fields that can be updated are
//!   optional, so the `update` function returns this builder struct so you don't have to provide
//!   all the parameters if you don't need to.
//!
//! ## Functions
//!
//! ### Basic actions
//!
//! These functions perform basic write actions on lists as a whole. These all require write access
//! to the authenticated user's account.
//!
//! - `create`/`delete`
//! - `update` (see `ListUpdate` for full details)
//! - `subscribe`/`unsubscribe`
//! - `add_member`/`remove_member`
//! - `add_member_list`/`remove_member_list`
//!
//! ### Basic queries
//!
//! These functions let you query information about lists, or related to lists somehow.
//!
//! - `ownerships`/`subscriptions`/`list`: Note that `list` will only return the most recent 100
//!   lists in the `ownerships`/`subscriptions` sets.
//! - `memberships`
//! - `members`/`is_member`
//! - `subscribers`/`is_subscriber`
//! - `show`
//! - `statuses`

use std::collections::HashMap;

use common::*;

use rustc_serialize::json;
use chrono;

use auth;
use links;
use user;
use error::Error::InvalidResponse;
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
#[derive(Clone, Debug)]
pub struct List {
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

impl FromJson for List {
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

        Ok(List {
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

///Represents a pending update to a list's metadata.
pub struct ListUpdate<'a> {
    list: ListID<'a>,
    name: Option<&'a str>,
    public: Option<bool>,
    desc: Option<&'a str>,
}

impl<'a> ListUpdate<'a> {
    ///Updates the name of the list.
    pub fn name(self, name: &'a str) -> ListUpdate<'a> {
        ListUpdate {
            name: Some(name),
            ..self
        }
    }

    ///Sets whether the list is public.
    pub fn public(self, public: bool) -> ListUpdate<'a> {
        ListUpdate {
            public: Some(public),
            ..self
        }
    }

    ///Updates the description of the list.
    pub fn desc(self, desc: &'a str) -> ListUpdate<'a> {
        ListUpdate {
            desc: Some(desc),
            ..self
        }
    }

    ///Sends the update request to Twitter.
    pub fn send(self, token: &auth::Token) -> WebResponse<List> {
        let mut params = HashMap::new();
        add_list_param(&mut params, &self.list);

        if let Some(name) = self.name {
            add_param(&mut params, "name", name);
        }

        if let Some(public) = self.public {
            if public {
                add_param(&mut params, "mode", "public");
            }
            else {
                add_param(&mut params, "mode", "private");
            }
        }

        if let Some(desc) = self.desc {
            add_param(&mut params, "description", desc);
        }

        let mut resp = try!(auth::post(links::lists::UPDATE, token, Some(&params)));

        parse_response(&mut resp)
    }
}
