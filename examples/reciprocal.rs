extern crate twitter;

mod common;

use std::collections::HashSet;
use twitter::{user, Response};

//IMPORTANT: see common.rs for instructions on making sure this properly authenticates with
//Twitter.
fn main() {
    let config = common::Config::load();

    println!("");
    let friends = user::friends_ids(config.user_id, &config.con_token, &config.access_token).map(|r| r.unwrap()).collect::<Response<Vec<_>>>();
    let followers = user::followers_ids(config.user_id, &config.con_token, &config.access_token).map(|r| r.unwrap()).collect::<Response<Vec<_>>>();

    let friends_set = friends.response.iter().cloned().collect::<HashSet<i64>>();
    let followers_set = followers.response.iter().cloned().collect::<HashSet<i64>>();

    let reciprocals = friends_set.intersection(&followers_set).cloned().collect::<Vec<_>>();

    println!("{} accounts that you follow follow you back.", reciprocals.len());

    for user in user::lookup_ids(&reciprocals, &config.con_token, &config.access_token).unwrap().response {
        println!("{} (@{})", user.name, user.screen_name);
    }
}
