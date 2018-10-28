// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

extern crate egg_mode;

mod common;

use common::tokio::runtime::current_thread::block_on_all;
use common::futures::Stream;

use std::collections::HashSet;
use egg_mode::user;

//IMPORTANT: see common.rs for instructions on making sure this properly authenticates with
//Twitter.
fn main() {
    let config = common::Config::load();

    println!("");
    let mut friends = HashSet::new();
    block_on_all(user::friends_ids(config.user_id, &config.token)
                  .map(|r| r.response)
                  .for_each(|id| { friends.insert(id); Ok(()) })).unwrap();

    let mut followers = HashSet::new();
    block_on_all(user::followers_ids(config.user_id, &config.token)
                  .map(|r| r.response)
                  .for_each(|id| { followers.insert(id); Ok(()) })).unwrap();

    let reciprocals = friends.intersection(&followers).cloned().collect::<Vec<_>>();
    let reciprocals_ct = reciprocals.len();
    println!("{} accounts that you follow follow you back.", reciprocals_ct);

    if reciprocals_ct > 0 {
        for user in block_on_all(user::lookup(&reciprocals, &config.token)).unwrap() {
            println!("{} (@{})", user.name, user.screen_name);
        }
    }
}
