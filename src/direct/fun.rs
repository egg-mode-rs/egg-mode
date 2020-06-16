// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::common::*;

use std::convert::TryInto;

use crate::{auth, links};

use super::*;

/// Lookup a single DM by its numeric ID.
pub async fn show(id: u64, token: &auth::Token) -> Result<Response<DirectMessage>, error::Error> {
    let params = ParamList::default().add_param("id", id.to_string());
    let req = get(links::direct::SHOW, token, Some(&params));
    let resp: Response<raw::SingleEvent> = request_with_json_response(req).await?;
    Response::try_map(resp, |ev| ev.try_into())
}

/// Load the first page of the list of direct messages sent and received by the authorized user.
pub fn list(token: &auth::Token) -> Timeline {
    Timeline::new(links::direct::LIST, token.clone())
}

/////Delete the direct message with the given ID.
/////
/////The authenticated user must be the sender of this DM for this call to be successful.
/////
/////On a successful deletion, the future returned by this function yields the freshly-deleted
/////message.
//pub async fn delete(id: u64, token: &auth::Token) -> Result<Response<DirectMessage>, error::Error> {
//    let params = ParamList::new().add_param("id", id.to_string());
//    let req = post(links::direct::DELETE, token, Some(&params));
//    request_with_json_response(req).await
//}
