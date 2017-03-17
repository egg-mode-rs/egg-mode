//! Structs and functions for working with lists.

use common::*;

use rustc_serialize::json;
use chrono;

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
