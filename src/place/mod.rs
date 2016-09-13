//! Types and methods for looking up locations.

use std::collections::HashMap;
use std::fmt;

use rustc_serialize::json;

use common::*;
use error;
use error::Error::{InvalidResponse, MissingValue};

mod fun;

pub use self::fun::*;

///Represents a named location.
#[derive(Debug)]
pub struct Place {
    ///Alphanumeric ID of the location.
    pub id: String,
    ///Map of miscellaneous information about this place. See [Twitter's documentation][attrib] for
    ///details and common attribute keys.
    ///
    ///[attrib]: https://dev.twitter.com/overview/api/places#attributes
    pub attributes: HashMap<String, String>,
    ///A bounding box of latitude/longitude coordinates that encloses this place.
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
    pub contained_within: Option<Box<Place>>,
}

///Represents the type of region represented by a given place.
#[derive(Debug)]
pub enum PlaceType {
    ///A coordinate with no area.
    Point,
    ///A region within a city.
    Neighborhood,
    ///An entire city.
    City,
    ///An administrative area, e.g. state or province.
    Admin,
    ///An entire country.
    Country,
}

///Represents the accuracy of a GPS measurement, when being given to a location search.
#[derive(Debug)]
pub enum Accuracy {
    ///Location accurate to the given number of meters.
    Meters(u32),
    ///Location accurate to the given number of feet.
    Feet(u32),
}

///Represents the result of a location search, either via `reverse_geocode` or `search`.
pub struct SearchResult {
    ///The full URL used to pull the result list. This can be fed to the `_url` version of your
    ///original call to avoid having to fill out the argument list again.
    pub url: String,
    ///The list of results from the search.
    pub results: Vec<Place>,
}

///Display impl to make `to_string()` format the enum for sending to Twitter. This is *mostly* just
///a lowercase version of the variants, but `Point` is rendered as `"poi"` instead.
impl fmt::Display for PlaceType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            PlaceType::Point => write!(f, "poi"),
            PlaceType::Neighborhood => write!(f, "neighborhood"),
            PlaceType::City => write!(f, "city"),
            PlaceType::Admin => write!(f, "admin"),
            PlaceType::Country => write!(f, "country"),
        }
    }
}

impl FromJson for PlaceType {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if let Some(s) = input.as_string() {
            if s == "poi" {
                Ok(PlaceType::Point)
            }
            else if s == "neighborhood" {
                Ok(PlaceType::Neighborhood)
            }
            else if s == "city" {
                Ok(PlaceType::City)
            }
            else if s == "admin" {
                Ok(PlaceType::Admin)
            }
            else if s == "country" {
                Ok(PlaceType::Country)
            }
            else {
                Err(InvalidResponse("unexpected string for PlaceType", Some(input.to_string())))
            }
        }
        else {
            Err(InvalidResponse("PlaceType received json that wasn't a string", Some(input.to_string())))
        }
    }
}

impl FromJson for Place {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse("Place received json that wasn't an object", Some(input.to_string())));
        }

        let attributes = if let Some(json) = input.find("attributes") {
            if let Some(attr) = json.as_object() {
                let mut attributes = HashMap::new();

                for (k, v) in attr.iter() {
                    attributes.insert(k.clone(), try!(String::from_json(v)));
                }

                attributes
            }
            else {
                return Err(InvalidResponse("Place.attributes received json that wasn't an object",
                                           Some(json.to_string())));
            }
        }
        else {
            return Err(MissingValue("attributes"));
        };

        let bounding_box = if let Some(vec) = input.find_path(&["bounding_box", "coordinates"]) {
            //"Array of Array of Array of Float" https://dev.twitter.com/overview/api/places#obj-boundingbox
            let parsed = try!(<Vec<Vec<(f64, f64)>>>::from_json(vec));
            try!(parsed.into_iter().next().ok_or(InvalidResponse("Place.bounding_box received an empty array",
                                                                 Some(vec.to_string()))))
        }
        else {
            return Err(MissingValue("bounding_box"));
        };

        Ok(Place {
            id: try!(field(input, "id")),
            attributes: attributes,
            bounding_box: bounding_box,
            country: try!(field(input, "country")),
            country_code: try!(field(input, "country_code")),
            full_name: try!(field(input, "full_name")),
            name: try!(field(input, "name")),
            place_type: try!(field(input, "place_type")),
            contained_within: field(input, "contained_within").ok(),
        })
    }
}

impl FromJson for SearchResult {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse("place::SearchResult received json that wasn't an object",
                                       Some(input.to_string())));
        }

        let query = try!(input.find("query").ok_or(MissingValue("query")));
        let result = try!(input.find("result").ok_or(MissingValue("result")));

        Ok(SearchResult {
            url: try!(field(query, "url")),
            results: try!(field(result, "places")),
        })
    }
}
