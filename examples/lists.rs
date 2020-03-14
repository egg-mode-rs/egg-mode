// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

mod common;

use egg_mode::list::{self, ListID};

#[tokio::main]
async fn main() {
    let config = common::Config::load().await;

    println!("Lists curated by user @Scobleizer:");
    let lists = list::list("Scobleizer", true, &config.token).await.unwrap();
    for list in lists.iter() {
        if list.user.screen_name == "Scobleizer" {
            println!("    {} ({})", list.name, list.slug);
        }
    }

    println!("\nMembers of @Scobleizer/lists/tech-news:");
    let members = list::members(ListID::from_slug("Scobleizer", "tech-news"), &config.token)
        .call()
        .await
        .unwrap();
    for m in members.response.users {
        println!("    {}", m.screen_name)
    }
}
