extern crate egg_mode;
extern crate futures;

mod common;
use common::futures::Stream;
use common::tokio::runtime::current_thread::block_on_all;

use egg_mode::stream::StreamMessage;

fn main() {
    let config = common::Config::load();
    println!("Printing messages from the 'sample' stream\n");
    let stream = egg_mode::stream::sample(&config.token).for_each(|m| {
        use StreamMessage::*;
        match m {
            Tweet(tweet) => {
                common::print_tweet(&tweet);
                println!();
            },
            other => println!("{:?}", other)
        }
        futures::future::ok(())
    });
    if let Err(e) = block_on_all(stream) {
        println!("Stream error: {}", e);
        println!("Disconnected")
    }
}
