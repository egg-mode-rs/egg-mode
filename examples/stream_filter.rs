extern crate egg_mode;
extern crate futures;

mod common;
use common::futures::Stream;
use common::tokio::runtime::current_thread::block_on_all;

use egg_mode::stream::StreamMessage;

fn main() {
    let config = common::Config::load();
    println!("Streaming tweets containing popular programming languages (also Rust)");
    println!("Ctrl-C to quit\n");

    let stream = egg_mode::stream::filter()
        .track(&["rustlang", "python", "java", "javascript"])
        .start(&config.token)
        .for_each(|m| {
            match m {
                StreamMessage::Tweet(tweet) => {
                    common::print_tweet(&tweet);
                    println!();
                }
                other => println!("{:?}", other),
            }
            futures::future::ok(())
        });
    if let Err(e) = block_on_all(stream) {
        println!("Stream error: {}", e);
        println!("Disconnected")
    }
}
