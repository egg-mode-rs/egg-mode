// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use crate::common::*;
use crate::user;

use chrono;

use super::DMEntities;

#[derive(Debug, Deserialize)]
pub struct RawDirectMessage {
    ///Numeric ID for this DM.
    pub id: u64,
    ///UTC timestamp from when this DM was created.
    #[serde(deserialize_with = "deserialize_datetime")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    ///The text of the DM.
    pub text: String,
    ///Link, hashtag, and user mention information parsed out of the DM.
    pub entities: DMEntities,
    ///The screen name of the user who sent the DM.
    pub sender_screen_name: String,
    ///The ID of the user who sent the DM.
    pub sender_id: u64,
    ///Full information of the user who sent the DM.
    pub sender: Box<user::TwitterUser>,
    ///The screen name of the user who received the DM.
    pub recipient_screen_name: String,
    ///The ID of the user who received the DM.
    pub recipient_id: u64,
    ///Full information for the user who received the DM.
    pub recipient: Box<user::TwitterUser>,
}
