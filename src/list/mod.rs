use std::collections::HashMap;
use rustc_serialize::json;
use auth;
use cursor;
use cursor::CursorIter;
use user::UserID;
use user::UserSearch;
use error::Error::InvalidResponse;
use links;
use common::*;
use std::io::Read;
use tweet;
use tweet::Tweet;
use super::error;
use super::*;

pub mod fun;

pub enum ListID<'a> {
    SlugName(&'a str, &'a str),
    SlugID(&'a str, u64),
    ListID(u64)
}

impl<'a> ListID<'a> {
    pub fn from_slug_name(list_name: &'a str, list_owner: &'a str) -> ListID<'a> {
        ListID::SlugName(list_name, list_owner)
    }
    pub fn from_id(list_id: u64) -> ListID<'a> {
        ListID::ListID(list_id)
    }
    pub fn from_slug_id(list_name: &'a str, owner_id: u64) -> ListID<'a> {
        ListID::SlugID(list_name, owner_id)
    }
    pub fn show(self, token: &'a auth::Token) -> WebResponse<ListInfo> {
        let mut params = HashMap::new();
        match &self {
            &ListID::SlugName(slug, list_owner) => {
                add_param(&mut params, "slug", slug.to_string());
                add_param(&mut params, "owner_screen_name", list_owner.to_string());
            },
            &ListID::SlugID(slug, owner_id) => {
                add_param(&mut params, "slug", slug.to_string());
                add_param(&mut params, "owner_id", owner_id.to_string());
            },
            &ListID::ListID(id) => {
                add_param(&mut params, "list_id", id.to_string());
            }
        };
        let mut resp = try!(auth::get(links::lists::LISTS_SHOW, token, Some(&params)));

        parse_response(&mut resp)
    }
}

pub struct ListInfo {
    pub name: String,
    pub slug: String,
    pub id: u64,
    pub subscriber_count: u64,
    pub member_count: u64,
    pub full_name: String,
    pub description: String,
    pub uri: String,
    pub created_at: chrono::DateTime<chrono::UTC>,
}

impl FromJson for ListInfo {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse("List received json that wasn't an object", Some(input.to_string())));
        }

        field_present!(input, contributors_enabled);
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

impl ListInfo {
    pub fn into_list<'a>(self, params_base: Option<ParamList<'a>>, token: &'a auth::Token<'a>) -> List<'a> {
        List {
            list_id: ListID::ListID(self.id),
            token: token,
            params_base: params_base,
            count: 20,
            max_id: None,
            min_id: None,
            user_count: 20,
            include_rts: false,
            info: Some(self)
        }
    }
}

pub struct List<'a> {
    ///The list to use in requests
    list_id: ListID<'a>,
    ///The token to authorize requests with.
    token: &'a auth::Token<'a>,
    ///Optional set of params to include prior to adding lifetime navigation parameters.
    params_base: Option<ParamList<'a>>,
    ///The maximum number of tweets to return in a single call. Twitter doesn't guarantee returning
    ///exactly this number, as suspended or deleted content is removed after retrieving the initial
    ///collection of tweets.
    pub count: i32,
    ///The largest/most recent tweet ID returned in the last call to `start`, `older`, or `newer`.
    pub max_id: Option<u64>,
    ///The smallest/oldest tweet ID returned in the last call to `start`, `older`, or `newer`.
    pub min_id: Option<u64>,
    /// The maximum number of users to return in a single request.
    pub user_count: i32,
    /// Whether to get retweets from the list in addition to normal tweets
    pub include_rts: bool,

    pub info: Option<ListInfo>
}


impl<'a> List<'a> {
    ///Clear the saved IDs on this timeline.
    pub fn reset(&mut self) {
        self.max_id = None;
        self.min_id = None;
    }

    ///Return the set of tweets newer than the last set pulled, optionall placing a maximum tweet
    ///ID to bound with.
    pub fn newer_statuses(&mut self, max_id: Option<u64>) -> WebResponse<Vec<Tweet>> {
        let resp = try!(self.statuses(self.max_id, max_id));

        self.map_ids(&resp.response);

        Ok(resp)
    }

    ///Return the set of tweets between the IDs given.
    ///
    ///Note that the range is not fully inclusive; the tweet ID given by `since_id` will not be
    ///returned, but the tweet ID in `max_id` will be returned.
    ///
    ///If the range of tweets given by the IDs would return more than `self.count`, the newest set
    ///of tweets will be returned.
    pub fn statuses(&self, since_id: Option<u64>, max_id: Option<u64>) -> WebResponse<Vec<Tweet>> {
        let mut params = self.params_base.as_ref().cloned().unwrap_or_default();
        self.add_list_params(&mut params);
        if self.include_rts         { add_param(&mut params, "include_rts", "true".to_string()); }
        if let Some(id) = since_id  { add_param(&mut params, "since_id", id.to_string()); }
        if let Some(id) = max_id    { add_param(&mut params, "max_id", id.to_string()); }

        let mut resp = try!(auth::get(links::lists::LISTS_STATUSES, self.token, Some(&params)));

        parse_response(&mut resp)
    }

    pub fn is_member(&self, user: &user::UserID) -> bool {
        let mut params = self.params_base.as_ref().cloned().unwrap_or_default();
        self.add_list_params(&mut params);

        match user {
            &user::UserID::ID(id) => {
                add_param(&mut params, "user_id", id.to_string());
            },
            &user::UserID::ScreenName(screen_name) => {
                add_param(&mut params, "screen_name", screen_name.to_string());
            }
        };

        let mut resp = auth::get(links::lists::LISTS_MEMBERS_SHOW, self.token, Some(&params)).unwrap();
        let mut full_resp = String::new();
        resp.read_to_string(&mut full_resp);

        let json_resp_result = json::Json::from_str(&full_resp);

        if let Ok(j) = json_resp_result {
            if let Ok(_) = user::TwitterUser::from_json(&j) {
                true
            } else {
                false
            }
        } else {
            false
        }
    }

    pub fn members(&self) -> CursorIter<'a, cursor::UserCursor> {
       if let Some(ref p) = self.params_base {
           let mut params = p.clone();
           self.add_list_params(&mut params);
           CursorIter::new(links::lists::LISTS_MEMBERS, self.token, Some(params), Some(self.user_count))
       } else {
           let mut params = HashMap::new();
           self.add_list_params(&mut params);
           CursorIter::new(links::lists::LISTS_MEMBERS, self.token, Some(params), Some(self.user_count))
        }
    }

    fn add_list_params(&self, params: &mut ParamList<'a>) {
        match self.list_id {
            ListID::SlugName(slug, list_owner) => {
                add_param(params, "slug", slug.to_string());
                add_param(params, "owner_screen_name", list_owner.to_string());
            },
            ListID::SlugID(slug, owner_id) => {
                add_param(params, "slug", slug.to_string());
                add_param(params, "owner_id", owner_id.to_string());
            },
            ListID::ListID(id) => {
                add_param(params, "list_id", id.to_string());
            }
        };
    }

    ///Helper builder function to set the page size.
    pub fn with_page_size(self, page_size: i32) -> Self {
        List {
            count: page_size,
            ..self
        }
    }

    ///With the returned slice of Tweets, set the min_id and max_id on self.
    fn map_ids(&mut self, resp: &[Tweet]) {
        self.max_id = resp.first().map(|status| status.id);
        self.min_id = resp.last().map(|status| status.id);
    }

    ///Create an instance of `Timeline` with the given link and tokens.
    fn new(list_id: ListID<'a>, params_base: Option<ParamList<'a>>, token: &'a auth::Token) -> Self {
        List {
            list_id: list_id,
            token: token,
            params_base: params_base,
            count: 20,
            user_count: 20,
            max_id: None,
            min_id: None,
            include_rts: false,
            info: None
        }
    }
}

pub enum ListIterType {
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
    pub fn call_ownerships(&self) -> WebResponse<Vec<ListInfo>> {
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
    pub fn call_subscriptions(&self) -> WebResponse<Vec<ListInfo>> {
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
    pub fn call_memberships(&self) -> WebResponse<Vec<ListInfo>> {
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

    pub fn call_lists(&self) -> WebResponse<Vec<ListInfo>> {
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

        let mut x = match self.iter_type {
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
