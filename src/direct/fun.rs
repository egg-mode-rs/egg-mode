// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::common::*;

use std::convert::TryInto;

use crate::user::UserID;
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
pub async fn list(token: &auth::Token) -> Result<Response<Vec<DirectMessage>>, error::Error> {
    let req = get(links::direct::LIST, token, None);
    let resp: Response<raw::EventCursor> = request_with_json_response(req).await?;
    Response::try_map(resp, |evs| evs.try_into())
}

/////Create a `Timeline` struct to navigate the direct messages received by the authenticated user.
//pub fn received(token: &auth::Token) -> Timeline {
//    Timeline::new(links::direct::RECEIVED, None, token)
//}

/////Create a `Timeline` struct to navigate the direct messages sent by the authenticated user.
//pub fn sent(token: &auth::Token) -> Timeline {
//    Timeline::new(links::direct::SENT, None, token)
//}

/////Send a new direct message to the given user.
/////
/////The recipient must allow DMs from the authenticated user for this to be successful. In practice,
/////this means that the recipient must either follow the authenticated user, or they must have the
/////"allow DMs from anyone" setting enabled. As the latter setting has no visibility on the API,
/////there may be situations where you can't verify the recipient's ability to receive the requested
/////DM beforehand.
/////
/////Upon successfully sending the DM, the message will be returned.
//pub async fn send<T: Into<UserID>>(
//    to: T,
//    text: CowStr,
//    token: &auth::Token,
//) -> Result<Response<DirectMessage>, error::Error> {
//    let params = ParamList::new()
//        .add_user_param(to.into())
//        .add_param("text", text);

//    let req = post(links::direct::SEND, token, Some(&params));

//    request_with_json_response(req).await
//}

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

/////Create a `ConversationTimeline` loader that can load direct messages as a collection of
/////pre-sorted conversations.
/////
/////Note that this does not load any messages; you need to call `newest` or `next` for that. See
/////[`ConversationTimeline`] for details.
/////
/////[`ConversationTimeline`]: struct.ConversationTimeline.html
//pub fn conversations(token: &auth::Token) -> ConversationTimeline {
//    ConversationTimeline::new(token)
//}
