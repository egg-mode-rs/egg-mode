//! Methods to inquire about the Twitter service itself.

use rustc_serialize::json;

use auth;
use entities;
use error;
use error::Error::InvalidResponse;
use links;
use common::*;

///Returns the current Twitter Terms of Service as plain text.
pub fn terms(con_token: &auth::Token, access_token: &auth::Token) -> WebResponse<String> {
    let mut resp = try!(auth::get(links::service::TERMS, con_token, access_token, None));

    let ret = try!(parse_response::<json::Json>(&mut resp));

    Ok(Response {
        response: try!(field(&ret.response, "tos")),
        rate_limit: ret.rate_limit,
        rate_limit_remaining: ret.rate_limit_remaining,
        rate_limit_reset: ret.rate_limit_reset,
    })
}

///Returns the current Twitter Privacy Policy as plain text.
pub fn privacy(con_token: &auth::Token, access_token: &auth::Token) -> WebResponse<String> {
    let mut resp = try!(auth::get(links::service::PRIVACY, con_token, access_token, None));

    let ret = try!(parse_response::<json::Json>(&mut resp));

    Ok(Response {
        response: try!(field(&ret.response, "privacy")),
        rate_limit: ret.rate_limit,
        rate_limit_remaining: ret.rate_limit_remaining,
        rate_limit_reset: ret.rate_limit_reset,
    })
}

///Return the current configuration from Twitter, including the maximum length of a t.co URL and
///maximum photo resolutions per size, among others.
///
///From Twitter: "It is recommended applications request this endpoint when they are loaded, but no
///more than once a day."
pub fn config(con_token: &auth::Token, access_token: &auth::Token) -> WebResponse<Configuration> {
    let mut resp = try!(auth::get(links::service::CONFIG, con_token, access_token, None));

    parse_response(&mut resp)
}

///Represents a service configuration from Twitter.
#[derive(Debug)]
pub struct Configuration {
    ///The character limit in direct messages.
    pub dm_text_character_limit: i32,
    ///The maximum photo sizes for received media. If an uploaded photo is above the dimensions for
    ///a given size category, it will be scaled to that size according to the `resize` property on
    ///each entry.
    pub photo_sizes: entities::MediaSizes,
    ///The maximum length for a t.co URL when given a URL with protocol `http`.
    pub short_url_length: i32,
    ///The maximum length for a t.co URL when given a URL with protocol `https`.
    pub short_url_length_https: i32,
    ///A list of URL slugs that are not valid usernames when in a URL like
    ///`https://twitter.com/[slug]`.
    pub non_username_paths: Vec<String>,
}

impl FromJson for Configuration {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse("Configuration received json that wasn't an object",
                                       Some(input.to_string())));
        }

        Ok(Configuration {
            dm_text_character_limit: try!(field(input, "dm_text_character_limit")),
            photo_sizes: try!(field(input, "photo_sizes")),
            short_url_length: try!(field(input, "short_url_length")),
            short_url_length_https: try!(field(input, "short_url_length_https")),
            non_username_paths: try!(field(input, "non_username_paths")),
        })
    }
}
