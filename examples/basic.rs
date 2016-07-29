extern crate twitter;

use std::io::{Write, Read};

fn main() {
    //IMPORTANT: make an app for yourself at apps.twitter.com and get your
    //key/secret into these files; this example won't work without them
    let consumer_key = include_str!("consumer_key").trim();
    let consumer_secret = include_str!("consumer_secret").trim();

    let token = twitter::Token::new(consumer_key, consumer_secret);

    let mut config = String::new();
    let user_id: i64;
    let username: String;
    let access_token: twitter::Token;

    //look at all this unwrapping! who told you it was my birthday?
    if let Ok(mut f) = std::fs::File::open("twitter_settings") {
        f.read_to_string(&mut config).unwrap();

        let mut iter = config.split('\n');

        username = iter.next().unwrap().to_string();
        user_id = i64::from_str_radix(&iter.next().unwrap(), 10).unwrap();
        access_token = twitter::Token::new(iter.next().unwrap(),
                                                 iter.next().unwrap());

        println!("Welcome back, {}!", username);
    }
    else {
        let request_token = match twitter::request_token(&token, "oob") {
            Ok(token) => token,
            Err(e) => {
                println!("Error: {}", e);
                return;
            },
        };

        println!("Go to the following URL, sign in, and give me the PIN that comes back:");
        println!("{}", twitter::authorize_url(&request_token));

        let mut pin = String::new();
        std::io::stdin().read_line(&mut pin).unwrap();
        println!("");

        let tok_result = twitter::access_token(&token, &request_token, pin).unwrap();

        access_token = tok_result.0;
        user_id = tok_result.1;
        username = tok_result.2;

        config.push_str(&username);
        config.push('\n');
        config.push_str(&format!("{}", user_id));
        config.push('\n');
        config.push_str(&access_token.key);
        config.push('\n');
        config.push_str(&access_token.secret);

        let mut f = std::fs::File::create("twitter_settings").unwrap();
        f.write_all(config.as_bytes()).unwrap();

        println!("Welcome, {}, let's get this show on the road!", username);
    }

    let resp = twitter::user::TwitterUser::lookup_ids(&[user_id], &token, &access_token);
    let ref user = resp.unwrap().response[0];

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
