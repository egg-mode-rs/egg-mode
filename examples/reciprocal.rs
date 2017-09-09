// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

extern crate egg_mode;

mod common;

use common::tokio_core::reactor;
use common::futures::Stream;

use std::collections::HashSet;
use egg_mode::user;

//IMPORTANT: see common.rs for instructions on making sure this properly authenticates with
//Twitter.
fn main() {
    let mut core = reactor::Core::new().unwrap();

    let config = common::Config::load(&mut core);
    let handle = core.handle();

    println!("");
    let mut friends = HashSet::new();
    core.run(user::friends_ids(config.user_id, &config.token, &handle)
                  .map(|r| r.response)
                  .for_each(|id| { friends.insert(id); Ok(()) })).unwrap();

    let mut followers = HashSet::new();
    core.run(user::followers_ids(config.user_id, &config.token, &handle)
                  .map(|r| r.response)
                  .for_each(|id| { followers.insert(id); Ok(()) })).unwrap();

    let reciprocals = friends.intersection(&followers).cloned().collect::<Vec<_>>();

    println!("{} accounts that you follow follow you back.", reciprocals.len());

    for user in core.run(user::lookup(&reciprocals, &config.token, &handle)).unwrap() {
        println!("{} (@{})", user.name, user.screen_name);
    }
}
