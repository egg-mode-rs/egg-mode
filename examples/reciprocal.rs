// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

mod common;

use futures::TryStreamExt;

use egg_mode::error::Result;
use egg_mode::user;
use std::collections::HashSet;

// IMPORTANT: see common.rs for instructions on making
// sure this properly authenticates with Twitter.
#[tokio::main]
async fn main() -> Result<()> {
    let config = common::Config::load().await;

    println!("");
    let friends: HashSet<u64> = user::friends_ids(config.user_id, &config.token)
        .map_ok(|r| r.response)
        .try_collect()
        .await?;

    let followers: HashSet<u64> = user::followers_ids(config.user_id, &config.token)
        .map_ok(|r| r.response)
        .try_collect()
        .await?;

    let reciprocals = friends
        .intersection(&followers)
        .cloned()
        .collect::<Vec<_>>();
    let reciprocals_ct = reciprocals.len();
    println!(
        "{} accounts that you follow follow you back.",
        reciprocals_ct
    );

    if reciprocals_ct > 0 {
        for user in user::lookup(reciprocals, &config.token).await?.iter() {
            println!("{} (@{})", user.name, user.screen_name);
        }
    }
    Ok(())
}
