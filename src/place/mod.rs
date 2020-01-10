// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Types and methods for looking up locations.
//!
//! Location search for Twitter works in one of two ways. The most direct method is to take a
//! latitude/longitude coordinate (say, from a devide's GPS system or by geolocating from wi-fi
//! networks, or simply from a known coordinate) and call `reverse_geocode`. Twitter says
//! `reverse_geocode` provides more of a "raw data access", and it can be considered to merely show
//! what locations are in that point or area.
//!
//! On the other hand, if you're intending to let a user select from a list of locations, you can
//! use the `search_*` methods instead. These have much of the same available parameters, but will
//! "potentially re-order \[results\] with regards to the user who is authenticated." In addition,
//! the results may potentially pull in "nearby" results to allow for a more broad selection or to
//! account for inaccurate location reporting.
//!
//! Since there are several optional parameters to both query methods, each one is assembled as a
//! builder. You can create the builder with the `reverse_geocode`, `search_point`, `search_query`,
//! or `search_ip` functions. From there, add any additional parameters by chaining method calls
//! onto the builder. When you're ready to peform the search call, hand your tokens to `call`, and
//! the list of results will be returned.
//!
//! Along with the list of place results, Twitter also returns the full search URL. egg-mode
//! returns this URL as part of the result struct, allowing you to perform the same search using
//! the `reverse_geocode_url` or `search_url` functions.

use std::collections::HashMap;
use std::fmt;

use serde::de::Error;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json;

use crate::common::*;
use crate::{auth, links};

mod fun;

pub use self::fun::*;

// https://developer.twitter.com/en/docs/tweets/data-dictionary/overview/geo-objects#place
///Represents a named location.
#[derive(Debug, Clone, Deserialize)]
pub struct Place {
    ///Alphanumeric ID of the location.
    pub id: String,
    ///Map of miscellaneous information about this place. See [Twitter's documentation][attrib] for
    ///details and common attribute keys.
    ///
    ///[attrib]: https://dev.twitter.com/overview/api/places#attributes
    pub attributes: HashMap<String, String>,
    ///A bounding box of latitude/longitude coordinates that encloses this place.
    #[serde(deserialize_with = "deserialize_bounding_box")]
    pub bounding_box: Vec<(f64, f64)>,
    ///Name of the country containing this place.
    pub country: String,
    ///Shortened country code representing the country containing this place.
    pub country_code: String,
    ///Full human-readable name of this place.
    pub full_name: String,
    ///Short human-readable name of this place.
    pub name: String,
    ///The type of location represented by this place.
    pub place_type: PlaceType,
    ///If present, the country or administrative region that contains this place.
    pub contained_within: Option<Vec<Place>>,
}

///Represents the type of region represented by a given place.
#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum PlaceType {
    ///A coordinate with no area.
    #[serde(rename = "poi")]
    PointOfInterest,
    ///A region within a city.
    #[serde(rename = "neighborhood")]
    Neighborhood,
    ///An entire city.
    #[serde(rename = "city")]
    City,
    ///An administrative area, e.g. state or province.
    #[serde(rename = "admin")]
    Admin,
    ///An entire country.
    #[serde(rename = "country")]
    Country,
}

///Represents the accuracy of a GPS measurement, when being given to a location search.
#[derive(Debug, Copy, Clone)]
pub enum Accuracy {
    ///Location accurate to the given number of meters.
    Meters(f64),
    ///Location accurate to the given number of feet.
    Feet(f64),
}

///Represents the result of a location search, either via `reverse_geocode` or `search`.
pub struct SearchResult {
    ///The full URL used to pull the result list. This can be fed to the `_url` version of your
    ///original call to avoid having to fill out the argument list again.
    pub url: String,
    ///The list of results from the search.
    pub results: Vec<Place>,
}

impl<'de> Deserialize<'de> for SearchResult {
    fn deserialize<D>(deser: D) -> Result<SearchResult, D::Error>
    where
        D: Deserializer<'de>,
    {
        let raw: serde_json::Value = serde_json::Value::deserialize(deser)?;
        let url = raw
            .get("query")
            .and_then(|obj| obj.get("url"))
            .ok_or_else(|| D::Error::custom("Malformed search result"))?
            .to_string();
        let results = raw
            .get("result")
            .and_then(|obj| obj.get("places"))
            .and_then(|arr| <Vec<Place>>::deserialize(arr).ok())
            .ok_or_else(|| D::Error::custom("Malformed search result"))?;
        Ok(SearchResult { url, results })
    }
}

///Represents a `reverse_geocode` query before it is sent.
///
///The available methods on this builder struct allow you to specify optional parameters to the
///search operation. Where applicable, each method lists its default value and acceptable ranges.
///
///To complete your search setup and send the query to Twitter, hand your tokens to `call`. The
///list of results from Twitter will be returned, as well as a URL to perform the same search via
///`reverse_geocode_url`.
pub struct GeocodeBuilder {
    coordinate: (f64, f64),
    accuracy: Option<Accuracy>,
    granularity: Option<PlaceType>,
    max_results: Option<u32>,
}

impl GeocodeBuilder {
    ///Begins building a reverse-geocode query with the given coordinate.
    fn new(latitude: f64, longitude: f64) -> Self {
        GeocodeBuilder {
            coordinate: (latitude, longitude),
            accuracy: None,
            granularity: None,
            max_results: None,
        }
    }

    ///Expands the area to search to the given radius. By default, this is zero.
    ///
    ///From Twitter: "If coming from a device, in practice, this value is whatever accuracy the
    ///device has measuring its location (whether it be coming from a GPS, WiFi triangulation,
    ///etc.)."
    pub fn accuracy(self, accuracy: Accuracy) -> Self {
        GeocodeBuilder {
            accuracy: Some(accuracy),
            ..self
        }
    }

    ///Sets the minimal specificity of what kind of results to return. For example, passing `City`
    ///to this will make the eventual result exclude neighborhoods and points.
    pub fn granularity(self, granularity: PlaceType) -> Self {
        GeocodeBuilder {
            granularity: Some(granularity),
            ..self
        }
    }

    ///Restricts the maximum number of results returned in this search. This is not a guarantee
    ///that the search will return this many results, but instead provides a hint as to how many
    ///"nearby" results to return.
    ///
    ///This value has a default value of 20, which is also its maximum. If zero or a number greater
    ///than 20 is passed here, it will be defaulted to 20 before sending to Twitter.
    ///
    ///From Twitter: "Ideally, only pass in the number of places you intend to display to the user
    ///here."
    pub fn max_results(self, max_results: u32) -> Self {
        GeocodeBuilder {
            max_results: Some(max_results),
            ..self
        }
    }

    ///Finalize the search parameters and return the results collection.
    pub fn call(&self, token: &auth::Token) -> FutureResponse<SearchResult> {
        let params = ParamList::new()
            .add_param("lat", self.coordinate.0.to_string())
            .add_param("long", self.coordinate.1.to_string())
            .add_opt_param("accuracy", self.accuracy.map_string())
            .add_opt_param("granularity", self.granularity.map_string())
            .add_opt_param(
                "max_results",
                self.max_results.map(|count| {
                    let count = if count == 0 || count > 20 { 20 } else { count };
                    count.to_string()
                }),
            );

        let req = auth::get(links::place::REVERSE_GEOCODE, token, Some(&params));

        make_parsed_future(req)
    }
}

enum PlaceQuery<'a> {
    LatLon(f64, f64),
    Query(&'a str),
    IPAddress(&'a str),
}

///Represents a location search query before it is sent.
///
///The available methods on this builder struct allow you to specify optional parameters to the
///search operation. Where applicable, each method lists its default value and acceptable ranges.
///
///To complete your search setup and send the query to Twitter, hand your tokens to `call`. The
///list of results from Twitter will be returned, as well as a URL to perform the same search via
///`search_url`.
pub struct SearchBuilder<'a> {
    query: PlaceQuery<'a>,
    accuracy: Option<Accuracy>,
    granularity: Option<PlaceType>,
    max_results: Option<u32>,
    contained_within: Option<&'a str>,
    attributes: Option<HashMap<&'a str, &'a str>>,
}

impl<'a> SearchBuilder<'a> {
    ///Begins building a location search with the given query.
    fn new(query: PlaceQuery<'a>) -> Self {
        SearchBuilder {
            query: query,
            accuracy: None,
            granularity: None,
            max_results: None,
            contained_within: None,
            attributes: None,
        }
    }

    ///Expands the area to search to the given radius. By default, this is zero.
    ///
    ///From Twitter: "If coming from a device, in practice, this value is whatever accuracy the
    ///device has measuring its location (whether it be coming from a GPS, WiFi triangulation,
    ///etc.)."
    pub fn accuracy(self, accuracy: Accuracy) -> Self {
        SearchBuilder {
            accuracy: Some(accuracy),
            ..self
        }
    }

    ///Sets the minimal specificity of what kind of results to return. For example, passing `City`
    ///to this will make the eventual result exclude neighborhoods and points.
    pub fn granularity(self, granularity: PlaceType) -> Self {
        SearchBuilder {
            granularity: Some(granularity),
            ..self
        }
    }

    ///Restricts the maximum number of results returned in this search. This is not a guarantee
    ///that the search will return this many results, but instead provides a hint as to how many
    ///"nearby" results to return.
    ///
    ///From experimentation, this value has a default of 20 and a maximum of 100. If fewer
    ///locations match the search parameters, fewer places will be returned.
    ///
    ///From Twitter: "Ideally, only pass in the number of places you intend to display to the user
    ///here."
    pub fn max_results(self, max_results: u32) -> Self {
        SearchBuilder {
            max_results: Some(max_results),
            ..self
        }
    }

    ///Restricts results to those contained within the given Place ID.
    pub fn contained_within(self, contained_id: &'a str) -> Self {
        SearchBuilder {
            contained_within: Some(contained_id),
            ..self
        }
    }

    ///Restricts results to those with the given attribute. A list of common attributes are
    ///available in [Twitter's documentation for Places][attrs]. Custom attributes are supported in
    ///this search, if you know them. This function may be called multiple times with different
    ///`attribute_key` values to combine attribute search parameters.
    ///
    ///[attrs]: https://dev.twitter.com/overview/api/places#attributes
    ///
    ///For example, `.attribute("street_address", "123 Main St")` searches for places with the
    ///given street address.
    pub fn attribute(self, attribute_key: &'a str, attribute_value: &'a str) -> Self {
        let mut attrs = self.attributes.unwrap_or_default();
        attrs.insert(attribute_key, attribute_value);

        SearchBuilder {
            attributes: Some(attrs),
            ..self
        }
    }

    ///Finalize the search parameters and return the results collection.
    pub fn call(&self, token: &auth::Token) -> FutureResponse<SearchResult> {
        let mut params = match self.query {
            PlaceQuery::LatLon(lat, long) => ParamList::new()
                .add_param("lat", lat.to_string())
                .add_param("long", long.to_string()),
            PlaceQuery::Query(text) => ParamList::new().add_param("query", text),
            PlaceQuery::IPAddress(text) => ParamList::new().add_param("ip", text),
        }
        .add_opt_param("accuracy", self.accuracy.map_string())
        .add_opt_param("granularity", self.granularity.map_string())
        .add_opt_param("max_results", self.max_results.map_string())
        .add_opt_param("contained_within", self.contained_within);

        if let Some(ref attrs) = self.attributes {
            for (k, v) in attrs {
                params.add_param_ref(format!("attribute:{}", k), *v);
            }
        }

        let req = auth::get(links::place::SEARCH, token, Some(&params));

        make_parsed_future(req)
    }
}

///Display impl to make `to_string()` format the enum for sending to Twitter. This is *mostly* just
///a lowercase version of the variants, but `Point` is rendered as `"poi"` instead.
impl fmt::Display for PlaceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let quoted = serde_json::to_string(self).unwrap();
        let inner = &quoted[1..quoted.len() - 1]; // ignore the quote marks
        write!(f, "{}", inner)
    }
}

///Display impl to make `to_string()` format the enum for sending to Twitter. This turns `Meters`
///into the contained number by itself, and `Feet` into the number suffixed by `"ft"`.
impl fmt::Display for Accuracy {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Accuracy::Meters(dist) => write!(f, "{}", dist),
            Accuracy::Feet(dist) => write!(f, "{}ft", dist),
        }
    }
}

fn deserialize_bounding_box<'de, D>(ser: D) -> Result<Vec<(f64, f64)>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = serde_json::Value::deserialize(ser)?;
    s.get("coordinates")
        .and_then(|arr| arr.get(0).cloned())
        .ok_or_else(|| D::Error::custom("Malformed 'bounding_box' attribute"))
        .and_then(|inner_arr| {
            serde_json::from_value::<Vec<(f64, f64)>>(inner_arr).map_err(|e| D::Error::custom(e))
        })
}
