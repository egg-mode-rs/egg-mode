// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use std::collections::HashMap;

mod common;

#[tokio::main]
async fn main() {
    let c = common::Config::load().await;

    let convos = egg_mode::direct::list(&c.token).into_conversations().await.unwrap();
    let mut users = HashMap::new();

    for (id, convo) in &convos {
        let user = egg_mode::user::show(*id, &c.token).await.unwrap();
        println!("-----");
        println!("Conversation with @{}:", user.screen_name);
        for msg in convo {
            if !users.contains_key(&msg.sender_id) {
                let sender = egg_mode::user::show(msg.sender_id, &c.token).await.unwrap();
                users.insert(msg.sender_id, sender);
            }
            let sender = &users[&msg.sender_id];
            println!(
                "--@{} sent at {}:",
                sender.screen_name,
                msg.created_at.with_timezone(&chrono::Local)
            );
            println!("    {}", msg.text);
        }
        println!("");
    }
}
