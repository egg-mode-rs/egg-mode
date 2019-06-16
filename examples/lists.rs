// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

extern crate egg_mode;

mod common;

use crate::common::tokio::runtime::current_thread::block_on_all;

use egg_mode::list::{self, ListID};

fn main() {
    let config = common::Config::load();

    println!("Lists curated by user @Scobleizer:");
    let lists = block_on_all(list::list("Scobleizer", true, &config.token)).unwrap();
    for list in lists {
        if list.user.screen_name == "Scobleizer" {
            println!("    {} ({})", list.name, list.slug);
        }
    }

    println!("\nMembers of @Scobleizer/lists/tech-news:");
    let members = block_on_all(
        list::members(ListID::from_slug("Scobleizer", "tech-news"), &config.token).call(),
    )
    .unwrap();
    for m in members.response.users {
        println!("    {}", m.screen_name)
    }
}
