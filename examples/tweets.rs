// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

extern crate egg_mode;

mod common;

use common::tokio::runtime::current_thread::block_on_all;

fn main() {
    let config = common::Config::load();
    let tweet_id = 766678057788829697;

    println!("");
    println!("Load up an individual tweet:");
    let status = block_on_all(egg_mode::tweet::show(tweet_id, &config.token)).unwrap();
    common::print_tweet(&status);

    println!("");
    println!("Loading retweets of an individual tweet:");
    for rt in &block_on_all(egg_mode::tweet::retweets_of(tweet_id, 5, &config.token)).unwrap() {
        if let Some(ref user) = rt.user {
            println!("{} (@{})", user.name, user.screen_name);
        }
    }

    println!("");
    println!("Loading the user's home timeline:");
    let home = egg_mode::tweet::home_timeline(&config.token).with_page_size(5);
    let (_home, feed) = block_on_all(home.start()).unwrap();
    for status in feed {
        common::print_tweet(&status);
        println!("");
    }

    println!("");
    println!("Loading the user's mentions timeline:");
    let mentions = egg_mode::tweet::mentions_timeline(&config.token).with_page_size(5);
    let (_mentions, feed) = block_on_all(mentions.start()).unwrap();
    for status in feed {
        common::print_tweet(&status);
        println!("");
    }

    println!("");
    println!("Loading the user's timeline:");
    let user = egg_mode::tweet::user_timeline(config.user_id, true, true,
                                              &config.token).with_page_size(5);
    let (_user, feed) = block_on_all(user.start()).unwrap();
    for status in feed {
        common::print_tweet(&status);
        println!("");
    }
}
