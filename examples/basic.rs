extern crate twitter;

fn main() {
    let consumer_key = include_str!("consumer_key").trim();
    let consumer_secret = include_str!("consumer_secret").trim();

    let token = twitter::auth::Token::new(consumer_key, consumer_secret);

    let request_token = twitter::auth::request_token(&token, "oob").unwrap();

    println!("Go to the following URL, sign in, and give me the PIN that comes back:");
    println!("{}", twitter::auth::authorize_url(&request_token));

    let mut pin = String::new();
    std::io::stdin().read_line(&mut pin).unwrap();
    println!("");

    let (access_token, user_id, username) = twitter::auth::access_token(&token, &request_token, pin).unwrap();

    println!("Welcome, {}, let's get this show on the road!", username);
}
