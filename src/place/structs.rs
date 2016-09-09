use std::collections::HashMap;

use rustc_serialize::json;

use common::*;
use error;
use error::Error::{InvalidResponse, MissingValue};

///Represents a named location.
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
