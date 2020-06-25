// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::common::*;

use std::convert::{TryFrom, TryInto};

use crate::{auth, links};

use super::*;

/// Lookup a single DM by its numeric ID.
pub async fn show(id: u64, token: &auth::Token) -> Result<Response<DirectMessage>, error::Error> {
    let params = ParamList::default().add_param("id", id.to_string());
    let req = get(links::direct::SHOW, token, Some(&params));
    let resp: Response<raw::SingleEvent> = request_with_json_response(req).await?;
    Response::try_map(resp, |ev| ev.try_into())
}

/// Load the list of direct messages sent and received by the authorized user.
///
/// This function will only return the messages sent and received in the last 30 days. For more
/// information, see the docs for [`Timeline`].
///
/// [`Timeline`]: struct.Timeline.html
pub fn list(token: &auth::Token) -> Timeline {
    Timeline::new(links::direct::LIST, token.clone())
}

/// Delete the direct message with the given ID.
///
/// The authenticated user must be the sender of this DM for this call to be successful.
///
/// This function will only delete the DM for the user - other users who have received the message
/// will still see it.
///
/// Twitter does not return anything upon a successful deletion, so this function will return an
/// empty `Response` upon success.
pub async fn delete(id: u64, token: &auth::Token) -> Result<Response<()>, error::Error> {
    let params = ParamList::new().add_param("id", id.to_string());
    let req = auth::raw::delete(links::direct::DELETE, token, Some(&params));
    let (headers, _) = raw_request(req).await?;
    let rate_limit_status = RateLimit::try_from(&headers)?;
    Ok(Response {
        rate_limit_status,
        response: (),
    })
}
