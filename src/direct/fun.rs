// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::collections::HashMap;

use auth;
use links;
use common::*;
use user::UserID;

use super::*;

///Lookup a single DM by its numeric ID.
pub fn show(id: u64, token: &auth::Token, handle: &Handle)
    -> FutureResponse<DirectMessage>
{
    let mut params = HashMap::new();
    add_param(&mut params, "id", id.to_string());

    let req = auth::get(links::direct::SHOW, token, Some(&params));

    make_parsed_future(handle, req)
}

///Create a `Timeline` struct to navigate the direct messages received by the authenticated user.
pub fn received(token: &auth::Token, handle: &Handle) -> Timeline {
    Timeline::new(links::direct::RECEIVED, None, token, handle)
}

///Create a `Timeline` struct to navigate the direct messages sent by the authenticated user.
pub fn sent(token: &auth::Token, handle: &Handle) -> Timeline {
    Timeline::new(links::direct::SENT, None, token, handle)
}

///Send a new direct message to the given user.
///
///The recipient must allow DMs from the authenticated user for this to be successful. In practice,
///this means that the recipient must either follow the authenticated user, or they must have the
///"allow DMs from anyone" setting enabled. As the latter setting has no visibility on the API,
///there may be situations where you can't verify the recipient's ability to receive the requested
///DM beforehand.
///
///Upon successfully sending the DM, the message will be returned.
pub fn send<'id, T: Into<UserID<'id>>>(to: T, text: &str, token: &auth::Token, handle: &Handle)
    -> FutureResponse<DirectMessage>
{
    let mut params = HashMap::new();
    add_name_param(&mut params, &to.into());

    add_param(&mut params, "text", text);

    let req = auth::post(links::direct::SEND, token, Some(&params));

    make_parsed_future(handle, req)
}

///Delete the direct message with the given ID.
///
///The authenticated user must be the sender of this DM for this call to be successful.
///
///On a successful deletion, the future returned by this function yields the freshly-deleted
///message.
pub fn delete(id: u64, token: &auth::Token, handle: &Handle)
    -> FutureResponse<DirectMessage>
{
    let mut params = HashMap::new();
    add_param(&mut params, "id", id.to_string());

    let req = auth::post(links::direct::DELETE, token, Some(&params));

    make_parsed_future(handle, req)
}

///Create a `ConversationTimeline` loader that can load direct messages as a collection of
///pre-sorted conversations.
///
///Note that this does not load any messages; you need to call `newest` or `next` for that. See
///[`ConversationTimeline`] for details.
///
///[`ConversationTimeline`]: struct.ConversationTimeline.html
pub fn conversations(token: &auth::Token, handle: &Handle) -> ConversationTimeline {
    ConversationTimeline::new(token, handle)
}
