use serde::Deserialize;

use super::PlaceType;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawTrendLocation {
    pub country: String,
    pub country_code: String,
    pub name: String,
    pub parentid: u32,
    pub place_type: PlaceType,
    pub url: String,
    pub woeid: u32,
}
