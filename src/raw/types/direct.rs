// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Raw types used by the Direct Message API.
//!
//! When loading direct messages from the Twitter API, they don't come across as singular
//! structures that match the types that egg-mode exports. Instead, to match the Account Activity
//! API that the DM functionality is a part of, they come across as "events" with a deeply-nested
//! structure. If you would like to manually deserialize these events as part of using the raw
//! egg-mode API, these types are exported here to allow this functionality.
//!
//! All of the types in this module implement the `Deserialize` trait from serde, and can thus be
//! converted from JSON returned from Twitter. The types `SingleEvent` and `EventCursor` match the
//! data sent for the `show` and `list` endpoints, respectively, and can thus be deserialized
//! directly from those responses. These types implement the `TryFrom` trait to be converted into a
//! single `DirectMessage` or a `Vec<DirectMessage>`, respectively.
//!
//! The `RawDirectMessage` type represents a minimally-processed version of the `message_create`
//! event data sent by Twitter. It's contained within the `EventType` enum, which abstracts the
//! fact that the event data structure allows for other types of events. As egg-mode only loads
//! direct messages using these types, it only serves to ensure that the proper data is received by
//! Twitter.

pub use crate::direct::raw::*;
