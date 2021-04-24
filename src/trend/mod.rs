//! Sturcts and functions for working with trending topic in Twitter.
//!
//! In this module, you are able to get locations with trending topics.
//!
//! ## Types
//! - `TrendLocation`: the element of trending information returned by trend API
//! - `PlaceType`: a member in `TrendLocation`, which includes the code and related name
//!   to specify the kind of place
use serde::{Deserialize, Serialize};

mod fun;
mod raw;

pub use self::fun::*;

round_trip! { raw::RawTrendLocation,
    ///Reprsent the locations that Twitter has trending topic information
    #[derive(Debug, Clone)]
    pub struct TrendLocation {
        ///The country of the location that Twitter has trending topic information for.
        pub country: String,
        ///short alphabetic or numeric geographical codes developed to represent countries
        ///and dependent areas.
        pub country_code: Option<String>,
        ///The location with trending topic information.
        pub name: String,
        ///The woeid of the parent place.
        pub parentid: u32,
        ///The code and related name to specify the kind of location.
        pub place_type: PlaceType,
        ///The related url of woeid of the location. Note that the url returned in the response,
        ///is no longer valid.
        pub url: String,
        ///The "where on earth identifier"
        pub woeid: u32
    }
}

impl From<raw::RawTrendLocation> for TrendLocation {
    fn from(raw: raw::RawTrendLocation) -> TrendLocation {
        TrendLocation {
            country: raw.country,
            country_code: raw.country_code,
            name: raw.name,
            parentid: raw.parentid,
            place_type: raw.place_type,
            url: raw.url,
            woeid: raw.woeid,
        }
    }
}

///The code and related name to specify the kind of location.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PlaceType {
    ///The code of the location type
    pub code: u32,
    ///The name of the location type
    pub name: String,
}
