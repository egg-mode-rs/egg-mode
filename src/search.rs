// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Structs and methods for searching for tweets.
//!
//! Since there are several optional parameters for searches, egg-mode handles it with a builder
//! pattern. To begin, call `search` with your requested search term. Additional parameters can be
//! added onto the `SearchBuilder` struct that is returned. When you're ready to load the first
//! page of results, hand your tokens to `call`.
//!
//! ```rust,no_run
//! # use egg_mode::Token;
//! use tokio::runtime::current_thread::block_on_all;
//! # fn main() {
//! # let token: Token = unimplemented!();
//! use egg_mode::search::{self, ResultType};
//!
//! let search = block_on_all(search::search("rustlang")
//!                                  .result_type(ResultType::Recent)
//!                                  .call(&token))
//!                  .unwrap();
//!
//! for tweet in &search.statuses {
//!     println!("(@{}) {}", tweet.user.as_ref().unwrap().screen_name, tweet.text);
//! }
//! # }
//! ```
//!
//! Once you have your `SearchResult`, you can navigate the search results by calling `older` and
//! `newer` to get the next and previous pages, respsectively. In addition, you can see your
//! original query in the search result struct as well, so you can categorize multiple searches by
//! their query. While this is given as a regular field, note that modifying `query` will not
//! change what is searched for when you call `older` or `newer`; the `SearchResult` keeps its
//! search arguments in a separate private field.
//!
//! The search parameter given in the initial call to `search` has several options itself. A full
//! reference is available in [Twitter's Search API documentation][search-doc]. This listing by
//! itself does not include the search by Place ID, as mentioned on [a separate Tweets by Place
//! page][search-place]. A future version of egg-mode might break these options into further
//! methods on `SearchBuilder`.
//!
//! The lifetime parameter on `SearchBuilder`, `SearchFuture`, and `SearchResult` correspond to the
//! text given as the search query and (if applicable) to the `lang` method of `SearchBuilder`. As
//! these types use `Cow<'a, str>` internally, you can hand these types owned Strings to give them
//! a `'static` lifetime, if necessary.
//!
//! [search-doc]: https://dev.twitter.com/rest/public/search
//! [search-place]: https://dev.twitter.com/rest/public/search-by-place

use std::borrow::Cow;
use std::collections::HashMap;
use std::fmt;

use futures::{Async, Future, Poll};
use serde::{Deserialize, Deserializer};

use crate::auth;
use crate::common::*;
use crate::error;
use crate::links;
use crate::tweet::Tweet;

///Begin setting up a tweet search with the given query.
pub fn search<'a, S: Into<Cow<'a, str>>>(query: S) -> SearchBuilder<'a> {
    SearchBuilder {
        query: query.into(),
        lang: None,
        result_type: None,
        count: None,
        until: None,
        geocode: None,
        since_id: None,
        max_id: None,
    }
}

///Represents what kind of tweets should be included in search results.
#[derive(Debug, Copy, Clone)]
pub enum ResultType {
    ///Return only the most recent tweets in the response.
    Recent,
    ///Return only the most popular tweets in the response.
    Popular,
    ///Include both popular and real-time results in the response.
    Mixed,
}

///Display impl that turns the variants into strings that can be used as search parameters.
impl fmt::Display for ResultType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ResultType::Recent => write!(f, "recent"),
            ResultType::Popular => write!(f, "popular"),
            ResultType::Mixed => write!(f, "mixed"),
        }
    }
}

///Represents a radius around a given location to return search results for.
pub enum Distance {
    ///A radius given in miles.
    Miles(f32),
    ///A radius given in kilometers.
    Kilometers(f32),
}

///Represents a tweet search query before being sent.
#[must_use = "SearchBuilder is lazy and won't do anything unless `call`ed"]
pub struct SearchBuilder<'a> {
    ///The text to search for.
    query: Cow<'a, str>,
    lang: Option<Cow<'a, str>>,
    result_type: Option<ResultType>,
    count: Option<u32>,
    until: Option<(u32, u32, u32)>,
    geocode: Option<(f32, f32, Distance)>,
    since_id: Option<u64>,
    max_id: Option<u64>,
}

impl<'a> SearchBuilder<'a> {
    ///Restrict search results to those that have been machine-parsed as the given two-letter
    ///language code.
    pub fn lang<S: Into<Cow<'a, str>>>(self, lang: S) -> Self {
        SearchBuilder {
            lang: Some(lang.into()),
            ..self
        }
    }

    ///Specify the type of search results to include. The default is `Recent`.
    pub fn result_type(self, result_type: ResultType) -> Self {
        SearchBuilder {
            result_type: Some(result_type),
            ..self
        }
    }

    ///Set the number of tweets to return per-page, up to a maximum of 100. The default is 15.
    pub fn count(self, count: u32) -> Self {
        SearchBuilder {
            count: Some(count),
            ..self
        }
    }

    ///Returns tweets created before the given date. Keep in mind that search is limited to the
    ///last 7 days of results, so giving a date here that's older than a week will return no
    ///results.
    pub fn until(self, year: u32, month: u32, day: u32) -> Self {
        SearchBuilder {
            until: Some((year, month, day)),
            ..self
        }
    }

    ///Restricts results to users located within the given radius of the given coordinate. This is
    ///preferably populated from location-tagged tweets, but can be filled in from the user's
    ///profile as a fallback.
    pub fn geocode(self, latitude: f32, longitude: f32, radius: Distance) -> Self {
        SearchBuilder {
            geocode: Some((latitude, longitude, radius)),
            ..self
        }
    }

    ///Restricts results to those with higher IDs than (i.e. that were posted after) the given
    ///tweet ID.
    pub fn since_tweet(self, since_id: u64) -> Self {
        SearchBuilder {
            since_id: Some(since_id),
            ..self
        }
    }

    ///Restricts results to those with IDs no higher than (i.e. were posted earlier than) the given
    ///tweet ID. Will include the given tweet in search results.
    pub fn max_tweet(self, max_id: u64) -> Self {
        SearchBuilder {
            max_id: Some(max_id),
            ..self
        }
    }

    ///Finalize the search terms and return the first page of responses.
    pub fn call(self, token: &auth::Token) -> SearchFuture<'a> {
        let mut params = HashMap::new();

        add_param(&mut params, "q", self.query);

        if let Some(lang) = self.lang {
            add_param(&mut params, "lang", lang);
        }

        if let Some(result_type) = self.result_type {
            add_param(&mut params, "result_type", result_type.to_string());
        }

        if let Some(count) = self.count {
            add_param(&mut params, "count", count.to_string());
        }

        if let Some((year, month, day)) = self.until {
            add_param(&mut params, "until", format!("{}-{}-{}", year, month, day));
        }

        if let Some((lat, lon, radius)) = self.geocode {
            match radius {
                Distance::Miles(r) => add_param(
                    &mut params,
                    "geocode",
                    format!("{:.6},{:.6},{}mi", lat, lon, r),
                ),
                Distance::Kilometers(r) => add_param(
                    &mut params,
                    "geocode",
                    format!("{:.6},{:.6},{}km", lat, lon, r),
                ),
            };
        }

        if let Some(since_id) = self.since_id {
            add_param(&mut params, "since_id", since_id.to_string());
        }

        if let Some(max_id) = self.max_id {
            add_param(&mut params, "max_id", max_id.to_string());
        }

        let req = auth::get(links::statuses::SEARCH, token, Some(&params));

        SearchFuture {
            loader: make_parsed_future(req),
            params: Some(params),
        }
    }
}

/// `Future` that represents a search query in progress.
///
/// When this future resolves, it will either contain a `SearchResult` or the error that was
/// encountered while loading or parsing the response.
#[must_use = "futures do nothing unless polled"]
pub struct SearchFuture<'a> {
    loader: FutureResponse<SearchResult<'a>>,
    params: Option<ParamList<'a>>,
}

impl<'a> Future for SearchFuture<'a> {
    type Item = Response<SearchResult<'a>>;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let mut resp = match self.loader.poll() {
            Ok(Async::Ready(resp)) => resp,
            Ok(Async::NotReady) => return Ok(Async::NotReady),
            Err(e) => return Err(e),
        };

        resp.params = self.params.take();
        Ok(Async::Ready(resp))
    }
}

#[derive(Debug, Deserialize)]
struct RawSearch<'a> {
    #[serde(borrow)]
    search_metadata: RawSearchMetaData<'a>,
    statuses: Vec<Tweet>,
}

#[derive(Debug, Deserialize)]
struct RawSearchMetaData<'a> {
    completed_in: f64,
    max_id: u64,
    /// absent if no more results to retrieve
    next_results: Option<&'a str>,
    query: &'a str,
    /// absent if no results
    refresh_url: Option<&'a str>,
    count: u64,
    since_id: u64,
}

impl<'de> Deserialize<'de> for SearchResult<'static> {
    fn deserialize<D>(deser: D) -> Result<SearchResult<'static>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawSearch::deserialize(deser)?;
        Ok(SearchResult {
            statuses: raw.statuses,
            query: raw.search_metadata.query.into(),
            max_id: raw.search_metadata.max_id,
            since_id: raw.search_metadata.since_id,
            params: None,
        })
    }
}

///Represents a page of search results, along with metadata to request the next or previous page.
#[derive(Debug)]
pub struct SearchResult<'a> {
    ///The list of statuses in this page of results.
    pub statuses: Vec<Tweet>,
    ///The query used to generate this page of results. Note that changing this will not affect the
    ///`next_page` method.
    pub query: String,
    ///Last tweet id in this page of results. This id can be used in `SearchBuilder::since_tweet`
    pub max_id: u64,
    ///First tweet id in this page of results. This id can be used in `SearchBuilder::since_tweet`
    pub since_id: u64,
    params: Option<ParamList<'a>>,
}

impl<'a> SearchResult<'a> {
    ///Load the next page of search results for the same query.
    pub fn older(&self, token: &auth::Token) -> SearchFuture<'a> {
        let mut params = self.params.as_ref().cloned().unwrap_or_default();
        params.remove("since_id");

        if let Some(min_id) = self.statuses.iter().map(|t| t.id).min() {
            add_param(&mut params, "max_id", (min_id - 1).to_string());
        } else {
            params.remove("max_id");
        }

        let req = auth::get(links::statuses::SEARCH, token, Some(&params));

        SearchFuture {
            loader: make_parsed_future(req),
            params: Some(params),
        }
    }

    ///Load the previous page of search results for the same query.
    pub fn newer(&self, token: &auth::Token) -> SearchFuture<'a> {
        let mut params = self.params.as_ref().cloned().unwrap_or_default();
        params.remove("max_id");

        if let Some(max_id) = self.statuses.iter().map(|t| t.id).max() {
            add_param(&mut params, "since_id", max_id.to_string());
        } else {
            params.remove("since_id");
        }

        let req = auth::get(links::statuses::SEARCH, token, Some(&params));

        SearchFuture {
            loader: make_parsed_future(req),
            params: Some(params),
        }
    }
}
