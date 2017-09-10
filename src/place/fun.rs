// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::collections::HashMap;

use futures::{Future, Async, Poll};

use auth;
use error;
use error::Error::BadUrl;
use links;
use common::*;

use super::*;
use super::PlaceQuery;

///Load the place with the given ID.
///
///## Examples
///
///```rust,no_run
///# let token = egg_mode::Token::Access {
///#     consumer: egg_mode::KeyPair::new("", ""),
///#     access: egg_mode::KeyPair::new("", ""),
///# };
///let result = egg_mode::place::show("18810aa5b43e76c7", &token).unwrap();
///
///assert!(result.full_name == "Dallas, TX");
///```
pub fn show<'a>(id: &str, token: &auth::Token, handle: &'a Handle) -> FutureResponse<'a, Place> {
    let url = format!("{}/{}.json", links::place::SHOW_STEM, id);

    let req = auth::get(&url, token, None);

    make_parsed_future(handle, req)
}

///Begins building a reverse-geocode search with the given coordinate.
///
///## Examples
///
///```rust,no_run
///# let token = egg_mode::Token::Access {
///#     consumer: egg_mode::KeyPair::new("", ""),
///#     access: egg_mode::KeyPair::new("", ""),
///# };
///use egg_mode::place::{self, PlaceType};
///let result = place::reverse_geocode(51.507222, -0.1275)
///                   .granularity(PlaceType::City)
///                   .call(&token)
///                   .unwrap();
///
///assert!(result.results.iter().any(|pl| pl.full_name == "London, England"));
///```
pub fn reverse_geocode(latitude: f64, longitude: f64) -> GeocodeBuilder
{
    GeocodeBuilder::new(latitude, longitude)
}

fn parse_url<'a>(base: &'static str, full: &'a str) -> Result<ParamList<'a>, error::Error> {
    let mut iter = full.split('?');

    if let Some(base_part) = iter.next() {
        if base_part != base {
            return Err(BadUrl);
        }
    }
    else {
        return Err(BadUrl);
    }

    if let Some(list) = iter.next() {
        let mut p = HashMap::new();

        for item in list.split('&') {
            let mut kv_iter = item.split('=');

            let k = try!(kv_iter.next().ok_or(BadUrl));
            let v = try!(kv_iter.next().ok_or(BadUrl));

            add_param(&mut p, k, v);
        }

        Ok(p)
    }
    else {
        Err(BadUrl)
    }
}

///From a URL given with the result of `reverse_geocode`, perform the same reverse-geocode search.
///
///## Errors
///
///In addition to errors that might occur generally, this function will return a `BadUrl` error if
///the given URL is not a valid `reverse_geocode` query URL.
pub fn reverse_geocode_url<'a>(url: &'a str, token: &'a auth::Token, handle: &'a Handle)
    -> CachedSearchFuture<'a>
{
    let params = parse_url(links::place::REVERSE_GEOCODE, url);
    CachedSearchFuture::new(links::place::REVERSE_GEOCODE, token, handle, params)
}

///Begins building a location search via latitude/longitude.
///
///## Example
///
///```rust,no_run
///# let token = egg_mode::Token::Access {
///#     consumer: egg_mode::KeyPair::new("", ""),
///#     access: egg_mode::KeyPair::new("", ""),
///# };
///use egg_mode::place::{self, PlaceType};
///let result = place::search_point(51.507222, -0.1275)
///                   .granularity(PlaceType::City)
///                   .call(&token)
///                   .unwrap();
///
///assert!(result.results.iter().any(|pl| pl.full_name == "London, England"));
///```
pub fn search_point(latitude: f64, longitude: f64) -> SearchBuilder<'static> {
    SearchBuilder::new(PlaceQuery::LatLon(latitude, longitude))
}

///Begins building a location search via a text query.
///
///## Example
///
///```rust,no_run
///# let token = egg_mode::Token::Access {
///#     consumer: egg_mode::KeyPair::new("", ""),
///#     access: egg_mode::KeyPair::new("", ""),
///# };
///use egg_mode::place::{self, PlaceType};
///let result = place::search_query("columbia")
///                   .granularity(PlaceType::Admin)
///                   .call(&token)
///                   .unwrap();
///
///assert!(result.results.iter().any(|pl| pl.full_name == "British Columbia, Canada"));
///```
pub fn search_query<'a>(query: &'a str) -> SearchBuilder<'a> {
    SearchBuilder::new(PlaceQuery::Query(query))
}

///Begins building a location search via an IP address.
pub fn search_ip<'a>(query: &'a str) -> SearchBuilder<'a> {
    SearchBuilder::new(PlaceQuery::IPAddress(query))
}

///From a URL given with the result of any `search_*` function, perform the same location search.
///
///## Errors
///
///In addition to errors that might occur generally, this function will return a `BadUrl` error if
///the given URL is not a valid `search` query URL.
pub fn search_url<'a>(url: &'a str, token: &'a auth::Token, handle: &'a Handle)
    -> CachedSearchFuture<'a>
{
    let params = parse_url(links::place::SEARCH, url);
    CachedSearchFuture::new(links::place::SEARCH, token, handle, params)
}

/// A `TwitterFuture` that needs to parse a provided URL before making a request.
///
/// This is a special case of [`TwitterFuture`] returned by [`reverse_geocode_url`] and
/// [`search_url`] so it can parse the cached search URL and return an error before making a web
/// request. See the docs for `TwitterFuture` for details.
///
/// [`TwitterFuture`]: ../struct.TwitterFuture.html
/// [`reverse_geocode_url`]: fn.reverse_geocode_url.html
/// [`search_url`]: fn search_url.html
pub struct CachedSearchFuture<'a> {
    stem: &'static str,
    params: Option<Result<ParamList<'a>, error::Error>>,
    token: &'a auth::Token,
    handle: &'a Handle,
    future: Option<FutureResponse<'a, SearchResult>>,
}

impl<'a> CachedSearchFuture<'a> {
    fn new(stem: &'static str,
           token: &'a auth::Token,
           handle: &'a Handle,
           params: Result<ParamList<'a>, error::Error>)
        -> CachedSearchFuture<'a>
    {
        CachedSearchFuture {
            stem: stem,
            params: Some(params),
            token: token,
            handle: handle,
            future: None,
        }
    }
}

impl<'a> Future for CachedSearchFuture<'a> {
    type Item = Response<SearchResult>;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.params.take() {
            Some(Ok(params)) => {
                let req = auth::get(self.stem, self.token, Some(&params));

                self.future = Some(make_parsed_future(self.handle, req));
            }
            Some(Err(e)) => {
                return Err(e);
            }
            None => { }
        }

        if let Some(mut fut) = self.future.take() {
            match fut.poll() {
                Ok(Async::NotReady) => {
                    self.future = Some(fut);
                    Ok(Async::NotReady)
                }
                Ok(Async::Ready(res)) => Ok(Async::Ready(res)),
                Err(e) => Err(e),
            }
        } else {
            Err(error::Error::FutureAlreadyCompleted)
        }
    }
}
