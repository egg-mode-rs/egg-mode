use crate::common::*;
use crate::error::Result;
use crate::trend::TrendLocation;
use crate::{auth, links};

///Returns the locations that Twitter has trending topic information for, closest to a
///specified location.
pub async fn closest(
    lat: f32,
    long: f32,
    token: &auth::Token,
) -> Result<Response<Vec<TrendLocation>>> {
    let params = ParamList::new()
        .add_param("lat", lat.to_string())
        .add_param("long", long.to_string());

    let req = get(links::trend::CLOSEST, token, Some(&params));

    request_with_json_response(req).await
}

///Returns the locations that Twitter has trending topic information for.
pub async fn available(token: &auth::Token) -> Result<Response<Vec<TrendLocation>>> {
    let req = get(links::trend::AVAILABLE, token, None);
    request_with_json_response(req).await
}
