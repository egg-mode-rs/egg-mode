// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

mod common;

#[tokio::main]
async fn main() {
    let c = common::Config::load().await;

    let convos = egg_mode::direct::conversations(&c.token);
    let convos = convos.newest().await.unwrap();

    for (id, convo) in &convos.conversations {
        let user = egg_mode::user::show(*id, &c.token).await.unwrap();
        println!("-----");
        println!("Conversation with @{}:", user.screen_name);
        for msg in convo {
            println!(
                "--@{} sent at {}:",
                msg.sender_screen_name,
                msg.created_at.with_timezone(&chrono::Local)
            );
            println!("    {}", msg.text);
        }
        println!("");
    }
}
