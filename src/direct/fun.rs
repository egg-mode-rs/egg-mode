// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::common::*;

use std::convert::{TryFrom, TryInto};

use crate::{auth, links};
use crate::user::{self, UserID};

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

/// Marks the given message as read in the sender's interface.
///
/// This function sends a read receipt for the given message ID, marking it and all messages before
/// it as read. The Twitter Web Client and other first-party Twitter clients can display an
/// indicator to show the last message that was read.  This function can also be used to clear an
/// "unread" indicator in these clients for the message.
///
/// Note that while this function accepts any `UserID`, the underlying Twitter API call only
/// accepts a numeric ID for the sender. If you pass a string Screen Name to this function, a
/// separate user lookup will occur prior to sending the read receipt. To avoid this extra lookup,
/// pass a numeric ID (or the `UserID::ID` variant of `UserID`) to this function.
pub async fn mark_read(
    id: u64,
    sender: impl Into<UserID>,
    token: &auth::Token,
) -> Result<Response<()>, error::Error> {
    let recipient_id = match sender.into() {
        UserID::ID(id) => id,
        UserID::ScreenName(name) => {
            let user = user::show(name, token).await?;
            user.id
        }
    };
    let params = ParamList::new()
        .add_param("last_read_event_id", id.to_string())
        .add_param("recipient_id", recipient_id.to_string());
    let req = post(links::direct::MARK_READ, token, Some(&params));
    let (headers, _) = raw_request(req).await?;
    let rate_limit_status = RateLimit::try_from(&headers)?;
    Ok(Response {
        rate_limit_status,
        response: (),
    })
}

/// Displays a visual typing indicator for the recipient.
///
/// The typing indicator will display for 3 seconds or until the authenticated user sends a message
/// to the recipient, whichever comes first.
///
/// Twitter warns that sending this request for every typing event will likely quickly come across
/// rate limits (1000 requests per 15 minutes). Instead, they recommend capturing these input
/// events and limiting API requests to some slower rate based on the behavior of your users and
/// the Twitter rate limit constraints.
///
/// Note that while this function accepts any `UserID`, the underlying Twitter API call only
/// accepts a numeric ID for the sender. If you pass a string Screen Name to this function, a
/// separate user lookup will occur prior to sending the read receipt. To avoid this extra lookup,
/// pass a numeric ID (or the `UserID::ID` variant of `UserID`) to this function.
pub async fn indicate_typing(
    recipient: impl Into<UserID>,
    token: &auth::Token,
) -> Result<Response<()>, error::Error> {
    let recipient_id = match recipient.into() {
        UserID::ID(id) => id,
        UserID::ScreenName(name) => {
            let user = user::show(name, token).await?;
            user.id
        }
    };

    let params = ParamList::new().add_param("recipient_id", recipient_id.to_string());
    let req = post(links::direct::INDICATE_TYPING, token, Some(&params));
    let (headers, _) = raw_request(req).await?;
    let rate_limit_status = RateLimit::try_from(&headers)?;
    Ok(Response {
        rate_limit_status,
        response: (),
    })
}
