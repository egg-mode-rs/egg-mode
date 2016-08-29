extern crate egg_mode;

mod common;

fn main() {
    let config = common::Config::load();
    let tweet_id = 766678057788829697;

    println!("");
    println!("Load up an individual tweet:");
    let status = egg_mode::tweet::show(tweet_id, &config.con_token, &config.access_token).unwrap().response;
    common::print_tweet(&status);

    println!("");
    println!("Loading retweets of an individual tweet:");
    for rt in &egg_mode::tweet::retweets_of(tweet_id, 5, &config.con_token, &config.access_token).unwrap().response {
        println!("{} (@{})", rt.user.name, rt.user.screen_name);
    }

    println!("");
    println!("Loading the user's home timeline:");
    let mut home = egg_mode::tweet::home_timeline(&config.con_token, &config.access_token).with_page_size(5);
    for status in &home.start().unwrap().response {
        common::print_tweet(&status);
        println!("");
    }

    println!("");
    println!("Loading the user's mentions timeline:");
    let mut home = egg_mode::tweet::mentions_timeline(&config.con_token, &config.access_token).with_page_size(5);
    for status in &home.start().unwrap().response {
        common::print_tweet(&status);
        println!("");
    }

    println!("");
    println!("Loading the user's timeline:");
    let mut home = egg_mode::tweet::user_timeline(config.user_id, true, true,
                                                  &config.con_token, &config.access_token).with_page_size(5);
    for status in &home.start().unwrap().response {
        common::print_tweet(&status);
        println!("");
    }
}
