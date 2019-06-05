// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//since this is going to get included in examples that might not use everything, clear out warnings
//that are unnecessary
#![allow(dead_code)]

//to prevent conflicts with examples, i'll import things here and let examples use it from here if
//they need it
pub extern crate chrono;
pub extern crate futures;
pub extern crate tokio;

use egg_mode;
use std;
use std::io::{Read, Write};

use self::tokio::runtime::current_thread::block_on_all;

//This is not an example that can be built with cargo! This is some helper code for the other
//examples so they can load access keys from the same place.

pub struct Config {
    pub token: egg_mode::Token,
    pub user_id: u64,
    pub screen_name: String,
}

impl Config {
    pub fn load() -> Self {
        //IMPORTANT: make an app for yourself at apps.twitter.com and get your
        //key/secret into these files; these examples won't work without them
        let consumer_key = include_str!("consumer_key").trim();
        let consumer_secret = include_str!("consumer_secret").trim();

        let con_token = egg_mode::KeyPair::new(consumer_key, consumer_secret);

        let mut config = String::new();
        let user_id: u64;
        let username: String;
        let token: egg_mode::Token;

        //look at all this unwrapping! who told you it was my birthday?
        if let Ok(mut f) = std::fs::File::open("twitter_settings") {
            f.read_to_string(&mut config).unwrap();

            let mut iter = config.split('\n');

            username = iter.next().unwrap().to_string();
            user_id = u64::from_str_radix(&iter.next().unwrap(), 10).unwrap();
            let access_token = egg_mode::KeyPair::new(
                iter.next().unwrap().to_string(),
                iter.next().unwrap().to_string(),
            );
            token = egg_mode::Token::Access {
                consumer: con_token,
                access: access_token,
            };

            if let Err(err) = block_on_all(egg_mode::verify_tokens(&token)) {
                println!("We've hit an error using your old tokens: {:?}", err);
                println!("We'll have to reauthenticate before continuing.");
                std::fs::remove_file("twitter_settings").unwrap();
            } else {
                println!("Welcome back, {}!", username);
            }
        } else {
            let request_token = block_on_all(egg_mode::request_token(&con_token, "oob")).unwrap();

            println!("Go to the following URL, sign in, and give me the PIN that comes back:");
            println!("{}", egg_mode::authorize_url(&request_token));

            let mut pin = String::new();
            std::io::stdin().read_line(&mut pin).unwrap();
            println!("");

            let tok_result =
                block_on_all(egg_mode::access_token(con_token, &request_token, pin)).unwrap();

            token = tok_result.0;
            user_id = tok_result.1;
            username = tok_result.2;

            match token {
                egg_mode::Token::Access {
                    access: ref access_token,
                    ..
                } => {
                    config.push_str(&username);
                    config.push('\n');
                    config.push_str(&format!("{}", user_id));
                    config.push('\n');
                    config.push_str(&access_token.key);
                    config.push('\n');
                    config.push_str(&access_token.secret);
                }
                _ => unreachable!(),
            }

            let mut f = std::fs::File::create("twitter_settings").unwrap();
            f.write_all(config.as_bytes()).unwrap();

            println!("Welcome, {}, let's get this show on the road!", username);
        }

        //TODO: Is there a better way to query whether a file exists?
        if std::fs::metadata("twitter_settings").is_ok() {
            Config {
                token: token,
                user_id: user_id,
                screen_name: username,
            }
        } else {
            Self::load()
        }
    }
}

pub fn print_tweet(tweet: &egg_mode::tweet::Tweet) {
    if let Some(ref user) = tweet.user {
        println!(
            "{} (@{}) posted at {}",
            user.name,
            user.screen_name,
            tweet.created_at.with_timezone(&chrono::Local)
        );
    }

    if let Some(ref screen_name) = tweet.in_reply_to_screen_name {
        println!("--> in reply to @{}", screen_name);
    }

    if let Some(ref status) = tweet.retweeted_status {
        if let Some(ref user) = status.user {
            println!("Retweeted from {}:", user.name);
        }
        print_tweet(status);
        return;
    } else {
        println!("{}", tweet.text);
    }

    println!("--via {} ({})", tweet.source.name, tweet.source.url);

    if let Some(ref place) = tweet.place {
        println!("--from {}", place.full_name);
    }

    if let Some(ref status) = tweet.quoted_status {
        println!("--Quoting the following status:");
        print_tweet(status);
    }

    if !tweet.entities.hashtags.is_empty() {
        println!("Hashtags contained in the tweet:");
        for tag in &tweet.entities.hashtags {
            println!("{}", tag.text);
        }
    }

    if !tweet.entities.symbols.is_empty() {
        println!("Symbols contained in the tweet:");
        for tag in &tweet.entities.symbols {
            println!("{}", tag.text);
        }
    }

    if !tweet.entities.urls.is_empty() {
        println!("URLs contained in the tweet:");
        for url in &tweet.entities.urls {
            if let Some(expanded_url) = &url.expanded_url {
                println!("{}", expanded_url);
            }
        }
    }

    if !tweet.entities.user_mentions.is_empty() {
        println!("Users mentioned in the tweet:");
        for user in &tweet.entities.user_mentions {
            println!("{}", user.screen_name);
        }
    }

    if let Some(ref media) = tweet.extended_entities {
        println!("Media attached to the tweet:");
        for info in &media.media {
            println!("A {:?}", info.media_type);
        }
    }
}
