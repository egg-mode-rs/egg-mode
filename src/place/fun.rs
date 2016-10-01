use std::collections::HashMap;

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
///# let con_token = egg_mode::Token::new("", "");
///# let access_token = egg_mode::Token::new("", "");
///let result = egg_mode::place::show("18810aa5b43e76c7",
///                                   &con_token, &access_token)
///                             .unwrap();
///
///assert!(result.response.full_name == "Dallas, TX");
///```
pub fn show(id: &str, con_token: &auth::Token, access_token: &auth::Token) -> WebResponse<Place> {
    let url = format!("{}/{}.json", links::place::SHOW_STEM, id);

    let mut resp = try!(auth::get(&url, con_token, access_token, None));

    parse_response(&mut resp)
}

///Begins building a reverse-geocode search with the given coordinate.
///
///## Examples
///
///```rust,no_run
///# let con_token = egg_mode::Token::new("", "");
///# let access_token = egg_mode::Token::new("", "");
///use egg_mode::place::{self, PlaceType};
///let result = place::reverse_geocode(51.507222, -0.1275)
///                   .granularity(PlaceType::City)
///                   .call(&con_token, &access_token)
///                   .unwrap();
///
///assert!(result.response.results.iter().any(|pl| pl.full_name == "London, England"));
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
pub fn reverse_geocode_url(url: &str, con_token: &auth::Token, access_token: &auth::Token)
    -> WebResponse<SearchResult>
{
    let params = try!(parse_url(links::place::REVERSE_GEOCODE, url));

    let mut resp = try!(auth::get(links::place::REVERSE_GEOCODE, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}

///Begins building a location search via latitude/longitude.
///
///## Example
///
///```rust,no_run
///# let con_token = egg_mode::Token::new("", "");
///# let access_token = egg_mode::Token::new("", "");
///use egg_mode::place::{self, PlaceType};
///let result = place::search_point(51.507222, -0.1275)
///                   .granularity(PlaceType::City)
///                   .call(&con_token, &access_token)
///                   .unwrap();
///
///assert!(result.response.results.iter().any(|pl| pl.full_name == "London, England"));
///```
pub fn search_point(latitude: f64, longitude: f64) -> SearchBuilder<'static> {
    SearchBuilder::new(PlaceQuery::LatLon(latitude, longitude))
}

///Begins building a location search via a text query.
///
///## Example
///
///```rust,no_run
///# let con_token = egg_mode::Token::new("", "");
///# let access_token = egg_mode::Token::new("", "");
///use egg_mode::place::{self, PlaceType};
///let result = place::search_query("columbia")
///                   .granularity(PlaceType::Admin)
///                   .call(&con_token, &access_token)
///                   .unwrap();
///
///assert!(result.response.results.iter().any(|pl| pl.full_name == "British Columbia, Canada"));
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
pub fn search_url(url: &str, con_token: &auth::Token, access_token: &auth::Token)
    -> WebResponse<SearchResult>
{
    let params = try!(parse_url(links::place::SEARCH, url));

    let mut resp = try!(auth::get(links::place::SEARCH, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}
