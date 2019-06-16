extern crate egg_mode;
extern crate futures;

mod common;
use crate::common::futures::Stream;
use crate::common::tokio::runtime::current_thread::block_on_all;

use egg_mode::stream::StreamMessage;

fn main() {
    let config = common::Config::load();
    println!("Streaming tweets containing popular programming languages (and also Rust)");
    println!("Ctrl-C to quit\n");

    let stream = egg_mode::stream::filter()
        .track(&["rustlang", "python", "java", "javascript"])
        .language(&["en"])
        .start(&config.token)
        .for_each(|m| {
            if let StreamMessage::Tweet(tweet) = m {
                common::print_tweet(&tweet);
                println!(
                    "──────────────────────────────────────"
                );
            } else {
                println!("{:?}", m);
            }
            futures::future::ok(())
        });
    if let Err(e) = block_on_all(stream) {
        println!("Stream error: {}", e);
        println!("Disconnected")
    }
}
