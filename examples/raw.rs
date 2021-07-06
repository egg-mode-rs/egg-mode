// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

mod common;

// By using the `raw` module, you can add in extra parameters that egg-mode doesn't add, or parse
// the response exactly the way you want.
//
// This example shows how to use the `raw` API to load a tweet, like with `tweet::show`.

use egg_mode::raw;
use egg_mode::Response;

#[tokio::main]
async fn main() {
    let config = common::Config::load().await;

    let url = "https://api.twitter.com/1.1/statuses/show.json";
    let tweet_id: u64 = 1261253754969640960;

    let params = raw::ParamList::new()
        .extended_tweets()
        .add_param("id", tweet_id.to_string());

    let req = raw::request_get(url, &config.token, Some(&params));
    let output: Response<serde_json::Value> = raw::response_json(req).await.unwrap();
    let json = output.response;

    // now that we have the response as plain JSON, we can directly access the fields we want
    let user_name = json["user"]["name"].as_str().unwrap();
    let screen_name = json["user"]["screen_name"].as_str().unwrap();

    // in tweet JSON, the text of the tweet can be in three places. In plain tweets without setting
    // `extended_tweets` above, the "text" field has the text, but it may be truncated if the tweet
    // is over 140 characters. In this case, the full text will be available in the
    // "extended_tweet" field, which contains the complete text and all the extended entities. If
    // you do set `extended_tweets`, then the tweet that gets returned is the same as what would
    // have been in "extended_tweet", and the "full_text" field needs to be checked.
    //
    // (This is checked as part of the Deserialize impl for Tweets.)
    let text = if let Some(t) = json["extended_tweet"]["full_text"].as_str() {
        t
    } else if let Some(t) = json["full_text"].as_str() {
        t
    } else if let Some(t) = json["text"].as_str() {
        t
    } else {
        panic!("couldn't load text from tweet?");
    };

    println!("{} (@{}) tweeted:", user_name, screen_name);
    println!("---");
    println!("{}", text);
}
