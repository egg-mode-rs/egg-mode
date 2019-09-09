// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

mod common;

use egg_mode::search::{self, ResultType};

use std::io::{stdin, BufRead};

#[tokio::main]
async fn main() {
    let config = common::Config::load().await;

    println!("Search term:");
    let line = stdin().lock().lines().next().unwrap().unwrap();

    let search = search::search(line)
        .result_type(ResultType::Recent)
        .count(10)
        .call(&config.token)
        .await
        .unwrap();

    for tweet in &search.statuses {
        common::print_tweet(tweet);
        println!()
    }
}
