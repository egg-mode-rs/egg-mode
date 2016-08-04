extern crate egg_mode;

mod common;

use std::collections::HashSet;
use egg_mode::user;

//IMPORTANT: see common.rs for instructions on making sure this properly authenticates with
//Twitter.
fn main() {
    let config = common::Config::load();

    println!("");
    let friends =
        user::friends_ids(config.user_id, &config.con_token, &config.access_token)
              .map(|r| r.unwrap().response)
              .collect::<HashSet<i64>>();
    let followers =
        user::followers_ids(config.user_id, &config.con_token, &config.access_token)
              .map(|r| r.unwrap().response)
              .collect::<HashSet<i64>>();

    let reciprocals = friends.intersection(&followers).cloned().collect::<Vec<_>>();

    println!("{} accounts that you follow follow you back.", reciprocals.len());

    for user in user::lookup_ids(&reciprocals, &config.con_token, &config.access_token).unwrap().response {
        println!("{} (@{})", user.name, user.screen_name);
    }
}
