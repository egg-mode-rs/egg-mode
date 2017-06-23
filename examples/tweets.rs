// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

extern crate egg_mode;

mod common;

fn main() {
    let config = common::Config::load();
    let tweet_id = 766678057788829697;

    println!("");
    println!("Load up an individual tweet:");
    let status = egg_mode::tweet::show(tweet_id, &config.token).unwrap().response;
    common::print_tweet(&status);

    //TODO: Starting in 0.7.0 these loops will be able to drop the `.response` at the end

    println!("");
    println!("Loading retweets of an individual tweet:");
    for rt in &egg_mode::tweet::retweets_of(tweet_id, 5, &config.token).unwrap().response {
        if let Some(ref user) = rt.user {
            println!("{} (@{})", user.name, user.screen_name);
        }
    }

    println!("");
    println!("Loading the user's home timeline:");
    let mut home = egg_mode::tweet::home_timeline(&config.token).with_page_size(5);
    for status in &home.start().unwrap().response {
        common::print_tweet(&status);
        println!("");
    }

    println!("");
    println!("Loading the user's mentions timeline:");
    let mut home = egg_mode::tweet::mentions_timeline(&config.token).with_page_size(5);
    for status in &home.start().unwrap().response {
        common::print_tweet(&status);
        println!("");
    }

    println!("");
    println!("Loading the user's timeline:");
    let mut home = egg_mode::tweet::user_timeline(config.user_id, true, true,
                                                  &config.token).with_page_size(5);
    for status in &home.start().unwrap().response {
        common::print_tweet(&status);
        println!("");
    }
}
