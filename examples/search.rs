extern crate egg_mode;

mod common;

use egg_mode::search::{self, ResultType};

fn main() {
    let config = common::Config::load();

    //rust tweets around dallas
    let search = search::search("rustlang")
                        .result_type(ResultType::Recent)
                        .count(10)
                        .call(&config.token)
                        .unwrap();

    for tweet in &search.statuses {
        common::print_tweet(tweet);
    }
}
