//! Types and methods for looking up locations.

use auth;
use links;
use common::*;

mod structs;

pub use self::structs::*;

///Load the place with the given ID.
pub fn show(id: &str, con_token: &auth::Token, access_token: &auth::Token) -> WebResponse<Place> {
    let url = format!("{}/{}.json", links::place::SHOW_STEM, id);

    let mut resp = try!(auth::get(&url, con_token, access_token, None));

    parse_response(&mut resp)
}
