// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

mod common;
use futures::TryStreamExt;

use egg_mode::stream::StreamMessage;

#[tokio::main]
async fn main() {
    let config = common::Config::load().await;
    println!("Streaming tweets containing popular programming languages (and also Rust)");
    println!("Ctrl-C to quit\n");

    let stream = egg_mode::stream::filter()
        .track(&["rustlang", "python", "java", "javascript"])
        .language(&["en"])
        .start(&config.token)
        .try_for_each(|m| {
            if let StreamMessage::Tweet(tweet) = m {
                common::print_tweet(&tweet);
                println!("──────────────────────────────────────");
            } else {
                println!("{:?}", m);
            }
            futures::future::ok(())
        });
    if let Err(e) = stream.await {
        println!("Stream error: {}", e);
        println!("Disconnected")
    }
}
