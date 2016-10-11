//! Methods to inquire about the Twitter service itself.

use rustc_serialize::json;

use auth;
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
