// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

mod common;

use egg_mode::place::PlaceType;

#[tokio::main]
async fn main() {
    let config = common::Config::load().await;

    let result = egg_mode::place::search_query("columbia")
        .granularity(PlaceType::Admin)
        .max_results(10)
        .call(&config.token)
        .await
        .unwrap();

    println!(
        "{} results for \"columbia\", administrative regions or larger:",
        result.results.len()
    );

    for place in &result.results {
        println!("{}", place.full_name);
    }
    println!("");

    let result = egg_mode::place::reverse_geocode(51.507222, -0.1275)
        .granularity(PlaceType::City)
        .call(&config.token)
        .await
        .unwrap();

    println!(
        "{} results for reverse-geocoding {}, {}:",
        result.results.len(),
        51.507222,
        -0.1275
    );

    for place in &result.results {
        println!("{}", place.full_name);
    }
}
