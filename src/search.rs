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
//! # #[tokio::main]
//! # async fn main() {
//! # let token: Token = unimplemented!();
//! use egg_mode::search::{self, ResultType};
//!
//! let search = search::search("rustlang")
//!     .result_type(ResultType::Recent)
//!     .call(&token)
//!     .await
//!     .unwrap();
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
//! [search-doc]: https://developer.twitter.com/en/docs/tweets/search/api-reference/get-search-tweets
//! [search-place]: https://developer.twitter.com/en/docs/tweets/search/guides/tweets-by-place

use std::fmt;

use serde::{Deserialize, Deserializer};

use crate::common::*;
use crate::tweet::Tweet;
use crate::{auth, error, links};

///Begin setting up a tweet search with the given query.
pub fn search<S: Into<CowStr>>(query: S) -> SearchBuilder {
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
pub struct SearchBuilder {
    ///The text to search for.
    query: CowStr,
    lang: Option<CowStr>,
    result_type: Option<ResultType>,
    count: Option<u32>,
    until: Option<(u32, u32, u32)>,
    geocode: Option<(f32, f32, Distance)>,
    since_id: Option<u64>,
    max_id: Option<u64>,
}

impl SearchBuilder {
    ///Restrict search results to those that have been machine-parsed as the given two-letter
    ///language code.
    pub fn lang<S: Into<CowStr>>(self, lang: S) -> Self {
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
    pub async fn call(self, token: &auth::Token) -> Result<Response<SearchResult>, error::Error> {
        let params = ParamList::new()
            .extended_tweets()
            .add_param("q", self.query)
            .add_opt_param("lang", self.lang)
            .add_opt_param("result_type", self.result_type.map_string())
            .add_opt_param("count", self.count.map_string())
            .add_opt_param("since_id", self.since_id.map_string())
            .add_opt_param("max_id", self.max_id.map_string())
            .add_opt_param(
                "until",
                self.until
                    .map(|(year, month, day)| format!("{}-{}-{}", year, month, day)),
            )
            .add_opt_param(
                "geocode",
                self.geocode.map(|(lat, lon, radius)| match radius {
                    Distance::Miles(r) => format!("{:.6},{:.6},{}mi", lat, lon, r),
                    Distance::Kilometers(r) => format!("{:.6},{:.6},{}km", lat, lon, r),
                }),
            );

        let req = get(links::statuses::SEARCH, token, Some(&params));
        let mut resp = request_with_json_response::<SearchResult>(req).await?;

        resp.response.params = Some(params);
        Ok(resp)
    }
}

#[derive(Debug, Deserialize)]
struct RawSearch {
    search_metadata: RawSearchMetaData,
    statuses: Vec<Tweet>,
}

#[derive(Debug, Deserialize)]
struct RawSearchMetaData {
    completed_in: f64,
    max_id: u64,
    /// absent if no more results to retrieve
    next_results: Option<String>,
    query: String,
    /// absent if no results
    refresh_url: Option<String>,
    count: u64,
    since_id: u64,
}

impl<'de> Deserialize<'de> for SearchResult {
    fn deserialize<D>(deser: D) -> Result<SearchResult, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw = RawSearch::deserialize(deser)?;
        Ok(SearchResult {
            statuses: raw.statuses,
            query: raw.search_metadata.query,
            max_id: raw.search_metadata.max_id,
            since_id: raw.search_metadata.since_id,
            params: None,
        })
    }
}

///Represents a page of search results, along with metadata to request the next or previous page.
#[derive(Debug)]
pub struct SearchResult {
    ///The list of statuses in this page of results.
    pub statuses: Vec<Tweet>,
    ///The query used to generate this page of results. Note that changing this will not affect the
    ///`next_page` method.
    pub query: String,
    ///Last tweet id in this page of results. This id can be used in `SearchBuilder::since_tweet`
    pub max_id: u64,
    ///First tweet id in this page of results. This id can be used in `SearchBuilder::since_tweet`
    pub since_id: u64,
    params: Option<ParamList>,
}

impl SearchResult {
    ///Load the next page of search results for the same query.
    pub async fn older(&self, token: &auth::Token) -> Result<Response<SearchResult>, error::Error> {
        let mut params = self
            .params
            .as_ref()
            .cloned()
            .unwrap_or_default()
            .extended_tweets();

        params.remove("since_id");

        if let Some(min_id) = self.statuses.iter().map(|t| t.id).min() {
            params.add_param_ref("max_id", (min_id - 1).to_string());
        } else {
            params.remove("max_id");
        }

        let req = get(links::statuses::SEARCH, token, Some(&params));
        let mut resp = request_with_json_response::<SearchResult>(req).await?;

        resp.response.params = Some(params);
        Ok(resp)
    }

    ///Load the previous page of search results for the same query.
    pub async fn newer(&self, token: &auth::Token) -> Result<Response<SearchResult>, error::Error> {
        let mut params = self
            .params
            .as_ref()
            .cloned()
            .unwrap_or_default()
            .extended_tweets();

        params.remove("max_id");
        if let Some(max_id) = self.statuses.iter().map(|t| t.id).max() {
            params.add_param_ref("since_id", max_id.to_string());
        } else {
            params.remove("since_id");
        }

        let req = get(links::statuses::SEARCH, token, Some(&params));
        let mut resp = request_with_json_response::<SearchResult>(req).await?;

        resp.response.params = Some(params);
        Ok(resp)
    }
}
