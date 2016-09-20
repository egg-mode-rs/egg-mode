extern crate egg_mode;

mod common;

use egg_mode::place::PlaceType;

fn main() {
    let config = common::Config::load();

    let result = egg_mode::place::search_query("columbia")
                                 .granularity(PlaceType::Admin)
                                 .max_results(10)
                                 .call(&config.con_token, &config.access_token).unwrap();

    println!("{} results for \"columbia\", administrative regions or larger:", result.response.results.len());

    for place in &result.response.results {
        println!("{}", place.full_name);
    }
}
