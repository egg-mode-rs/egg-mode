// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

extern crate egg_mode;

mod common;

use common::tokio_core::reactor;
use common::futures::Stream;

use egg_mode::user;

//IMPORTANT: see common.rs for instructions on making sure this properly authenticates with
//Twitter.
fn main() {
    let mut core = reactor::Core::new().unwrap();

    let config = common::Config::load(&mut core);
    let handle = core.handle();

    println!("");
    println!("Heterogeneous multi-user lookup:");

    let mut users: Vec<egg_mode::user::UserID> = vec![];
    users.push(config.user_id.into());
    users.push("SwiftOnSecurity".into());

    for user in core.run(user::lookup(&users, &config.token, &handle)).unwrap().response.iter() {
        print_user(user)
    }

    println!("");
    println!("Searching based on a term: (here, it's 'rustlang')");
    core.run(user::search("rustlang", &config.token, &handle).with_page_size(5).take(5).for_each(|resp| {
        print_user(&resp);
        Ok(())
    })).unwrap();

    println!("");
    println!("Who do you follow?");
    core.run(user::friends_of(config.user_id, &config.token, &handle).with_page_size(5).take(5).for_each(|resp| {
        print_user(&resp);
        Ok(())
    })).unwrap();

    println!("");
    println!("Who follows you?");
    core.run(user::followers_of(config.user_id, &config.token, &handle).with_page_size(5).take(5).for_each(|resp| {
        print_user(&resp);
        Ok(())
    })).unwrap();
}

fn print_user(user: &user::TwitterUser) {
    println!("");
    println!("{} (@{})", user.name, user.screen_name);
    println!("Created at {}", user.created_at);
    println!("Follows {}, followed by {}", user.friends_count, user.followers_count);
    if let Some(ref desc) = user.description {
        println!("{}", desc);
    }
    else {
        println!("(no description provided)");
    }
    match (&user.location, &user.url) {
        (&Some(ref loc), &Some(ref link)) => println!("{} | {}", loc, link),
        (&None, &Some(ref link)) => println!("{}", link),
        (&Some(ref loc), &None) => println!("{}", loc),
        (&None, &None) => (),
    }
}
