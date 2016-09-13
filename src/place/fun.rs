use std::collections::HashMap;

use auth;
use error::Error::BadUrl;
use links;
use common::*;

use super::*;

///Load the place with the given ID.
pub fn show(id: &str, con_token: &auth::Token, access_token: &auth::Token) -> WebResponse<Place> {
    let url = format!("{}/{}.json", links::place::SHOW_STEM, id);

    let mut resp = try!(auth::get(&url, con_token, access_token, None));

    parse_response(&mut resp)
}

///From the given latitude/longitude and accuracy measurement, return up to 20 Places that can be
///attached to a new tweet.
pub fn reverse_geocode(latitude: f64, longitude: f64, within: Accuracy, granularity: Option<PlaceType>,
                       max_results: Option<u32>, con_token: &auth::Token, access_token: &auth::Token)
    -> WebResponse<SearchResult>
{
    let mut params = HashMap::new();

    add_param(&mut params, "lat", latitude.to_string());
    add_param(&mut params, "long", longitude.to_string());

    match within {
        Accuracy::Meters(dist) => add_param(&mut params, "accuracy", dist.to_string()),
        Accuracy::Feet(dist) => add_param(&mut params, "accuracy", format!("{}ft", dist)),
    };

    if let Some(param) = granularity {
        add_param(&mut params, "granularity", param.to_string());
    }

    if let Some(count) = max_results {
        let count = if count == 0 || count > 20 { 20 } else { count };
        add_param(&mut params, "max_results", count.to_string());
    }

    let mut resp = try!(auth::get(links::place::REVERSE_GEOCODE, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}

///From a URL given with the result of `reverse_geocode`, return the same set of Places.
pub fn reverse_geocode_url(url: &str, con_token: &auth::Token, access_token: &auth::Token)
    -> WebResponse<SearchResult>
{
    let mut iter = url.split('?');

    if let Some(base) = iter.next() {
        if base != links::place::REVERSE_GEOCODE {
            return Err(BadUrl);
        }
    }
    else {
        return Err(BadUrl);
    }

    let params = if let Some(list) = iter.next() {
        let mut p = HashMap::new();

        for item in list.split('&') {
            let mut kv_iter = item.split('=');

            let k = try!(kv_iter.next().ok_or(BadUrl));
            let v = try!(kv_iter.next().ok_or(BadUrl));

            add_param(&mut p, k, v);
        }

        p
    }
    else {
        return Err(BadUrl);
    };

    let mut resp = try!(auth::get(links::place::REVERSE_GEOCODE, con_token, access_token, Some(&params)));

    parse_response(&mut resp)
}
