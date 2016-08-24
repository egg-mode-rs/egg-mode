extern crate egg_mode;

mod common;

fn main() {
    let config = common::Config::load();

    println!("");
    println!("Load up an individual tweet:");
    let status = egg_mode::tweet::show(766678057788829697, &config.con_token, &config.access_token).unwrap().response;
    print_tweet(&status);
}

fn print_tweet(status: &egg_mode::tweet::Tweet) {
    println!("{} (@{})", status.user.name, status.user.screen_name);
    println!("posted at {}", status.created_at);
    println!("{}", status.text);
}
