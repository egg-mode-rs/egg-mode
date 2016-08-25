extern crate egg_mode;

mod common;

fn main() {
    let config = common::Config::load();
    let tweet_id = 766678057788829697;

    println!("");
    println!("Load up an individual tweet:");
    let status = egg_mode::tweet::show(tweet_id, &config.con_token, &config.access_token).unwrap().response;
    print_tweet(&status);

    println!("");
    println!("Loading retweets of an individual tweet:");
    for rt in &egg_mode::tweet::retweets_of(tweet_id, 5, &config.con_token, &config.access_token).unwrap().response {
        println!("{} (@{})", rt.user.name, rt.user.screen_name);
    }
}

fn print_tweet(status: &egg_mode::tweet::Tweet) {
    println!("{} (@{})", status.user.name, status.user.screen_name);
    println!("posted at {}", status.created_at);
    println!("{}", status.text);
}
