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
//! single `DirectMessage` or a `Vec<DirectMessage>`, respectively. The other types in this module
//! represent sub-structures of these event types, and are either contained within those types, or
//! (in the case of `RawDirectMessage`) is converted from one of these other types during
//! deserialization.

pub use crate::direct::raw::*;
