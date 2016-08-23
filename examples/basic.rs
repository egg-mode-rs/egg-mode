extern crate egg_mode;

mod common;

//IMPORTANT: see common.rs for instructions on making sure this properly authenticates with
//Twitter.
fn main() {
    let config = common::Config::load();

    println!("");
    println!("Heterogeneous multi-user lookup:");

    let mut users: Vec<egg_mode::user::UserID> = vec![];
    users.push(config.user_id.into());
    users.push("SwiftOnSecurity".into());

    for user in egg_mode::user::lookup(&users, &config.con_token, &config.access_token).unwrap().response.iter() {
        print_user(user)
    }

    println!("");
    println!("Searching based on a term: (here, it's 'rustlang')");
    for resp in egg_mode::user::search("rustlang", &config.con_token, &config.access_token).with_page_size(5).take(5) {
        print_user(&resp.unwrap().response);
    }

    println!("");
    println!("Who do you follow?");
    for resp in egg_mode::user::friends_of(config.user_id, &config.con_token, &config.access_token).with_page_size(5).take(5) {
        print_user(&resp.unwrap().response);
    }

    println!("");
    println!("Who follows you?");
    for resp in egg_mode::user::followers_of(config.user_id, &config.con_token, &config.access_token).with_page_size(5).take(5) {
        print_user(&resp.unwrap().response);
    }

    println!("");
    println!("Who have you blocked?");
    for resp in egg_mode::user::blocks(&config.con_token, &config.access_token).take(5) {
        print_user(&resp.unwrap().response);
    }
}

fn print_user(user: &egg_mode::user::TwitterUser) {
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
