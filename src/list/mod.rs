use std::collections::HashMap;
use rustc_serialize::json;
use auth;
use cursor;
use user::UserID;
use error::Error::InvalidResponse;
use links;
use common::*;
use std::io::Read;
use super::error;
use super::*;

enum ListID<'a> {
    SlugName(&'a str, &'a str),
    SlugID(&'a str, u64),
    ListID(u64)
}

pub struct List<'a> {
    id: ListID<'a>,
    pub count: u64,
    pub cursor: i64,
    pub paging: bool,
    pub include_rts: bool
}

fn decode_tweets(j: json::Json) ->Result<Vec<tweet::Tweet>, error::Error> {
    if let Some(object) = j.as_object() {
        if let Some(users) = object.get("users") {
            if let Some(array) = users.as_array() {
                let mut users = Vec::with_capacity(array.len());
                for element in array.iter() {
                    if let Ok(x) = tweet::Tweet::from_json(element) {
                        users.push(x);
                    } else {
                        return Err(error::Error::InvalidResponse("Failed to parse tweet from json.", None))
                    }
                }
                Ok(users)
            } else {
                return Err(error::Error::InvalidResponse("Failed find tweet array in json response.", None))
            }
        } else {
            Err(error::Error::InvalidResponse("Expected field 'users' in json response, found none.", None))
        }
    } else {
        Err(error::Error::InvalidResponse("Expected json object.", None))
    }
}

fn decode_users(j: json::Json) -> Result<Vec<user::TwitterUser>, error::Error> {
    if let Some(object) = j.as_object() {
        if let Some(users) = object.get("users") {
            if let Some(array) = users.as_array() {
                let mut users = Vec::with_capacity(array.len());
                for element in array.iter() {
                    if let Ok(x) = user::TwitterUser::from_json(element) {
                        users.push(x);
                    } else {
                        return Err(error::Error::InvalidResponse("Failed to parse tweet from json.", None))
                    }
                }
                Ok(users)
            } else {
                return Err(error::Error::InvalidResponse("Failed find tweet array in json response.", None))
            }
        } else {
            Err(error::Error::InvalidResponse("Expected field 'users' in json response, found none.", None))
        }
    } else {
        Err(error::Error::InvalidResponse("Expected json object.", None))
    }
}

impl<'a> List<'a> {
    pub fn from_slug_name(list_name: &'a str, list_owner: &'a str) -> List<'a> {
        List { id: ListID::SlugName(list_name, list_owner), count: 20, cursor: -1, paging: false, include_rts: false }
    }
    pub fn from_id(list_id: u64) -> List<'a> {
        List { id: ListID::ListID(list_id), count: 20, cursor: -1, paging: false, include_rts: false }
    }
    pub fn from_slug_id(list_name: &'a str, owner_id: u64) -> List<'a> {
        List { id: ListID::SlugID(list_name, owner_id), count: 20, cursor: -1, paging: false, include_rts: false }
    }

    pub fn refresh(&mut self) {
        self.cursor = -1;
    }

    pub fn statuses(&mut self, token: &'a auth::Token) -> Result<Vec<tweet::Tweet>, error::Error> {
        let mut params = HashMap::new();

        match self.id {
            ListID::SlugName(slug, list_owner) => {
                add_param(&mut params, "slug", slug.to_string());
                add_param(&mut params, "owner_screen_name", list_owner.to_string());
            },
            ListID::SlugID(slug, owner_id) => {
                add_param(&mut params, "slug", slug.to_string());
                add_param(&mut params, "owner_id", owner_id.to_string());
            },
            ListID::ListID(id) => {
                add_param(&mut params, "list_id", id.to_string());
            }
        };
        add_param(&mut params, "count", self.count.to_string());
        add_param(&mut params, "cursor", self.cursor.to_string());

        if self.include_rts {
            add_param(&mut params, "include_rts", "true".to_string());
        }

        let mut resp = try!(auth::get(links::statuses::LISTS_STATUSES, &token, Some(&params)));

        let mut full_resp = String::new();
        try!(resp.read_to_string(&mut full_resp));

        let json_resp_result = json::Json::from_str(&full_resp);

        if let Ok(j) = json_resp_result {
            let ret = decode_tweets(j);
            if let Ok(_) = ret {
                if self.paging { self.cursor += 1; }
            }
            ret
        } else {
            Err(error::Error::InvalidResponse("Failed to decode json response...", None))
        }
    }

    pub fn members(&mut self, token: &'a auth::Token) -> Result<Vec<user::TwitterUser>, error::Error> {
        let mut params = HashMap::new();

        match self.id {
            ListID::SlugName(slug, list_owner) => {
                add_param(&mut params, "slug", slug.to_string());
                add_param(&mut params, "owner_screen_name", list_owner.to_string());
            },
            ListID::SlugID(slug, owner_id) => {
                add_param(&mut params, "slug", slug.to_string());
                add_param(&mut params, "owner_id", owner_id.to_string());
            },
            ListID::ListID(id) => {
                add_param(&mut params, "list_id", id.to_string());
            }
        };
        add_param(&mut params, "count", self.count.to_string());
        add_param(&mut params, "cursor", self.cursor.to_string());

        let mut resp = try!(auth::get(links::statuses::LISTS_MEMBERS, &token, Some(&params)));

        let mut full_resp = String::new();
        try!(resp.read_to_string(&mut full_resp));

        let json_resp_result = json::Json::from_str(&full_resp);

        if let Ok(j) = json_resp_result {
            let ret = decode_users(j);
            if let Ok(_) = ret {
                if self.paging { self.cursor += 1; }
            }
            ret
        } else {
            Err(error::Error::InvalidResponse("Failed to decode json response...", None))
        }
    }
}
