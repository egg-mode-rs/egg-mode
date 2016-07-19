extern crate hyper;
extern crate url;

pub mod auth;

use url::percent_encoding::{EncodeSet, utf8_percent_encode};

//the encode sets in the url crate don't quite match what twitter wants,
//so i'll make up my own
#[derive(Copy, Clone)]
struct TwitterEncodeSet;

impl EncodeSet for TwitterEncodeSet {
    fn contains(&self, byte: u8) -> bool {
        match byte {
            b'a' ... b'z' => false,
            b'A' ... b'Z' => false,
            b'0' ... b'9' => false,
            b'-' | b'.' | b'_' | b'~' => false,
            _ => true
        }
    }
}

pub fn percent_encode(src: &str) -> String {
    utf8_percent_encode(src, TwitterEncodeSet).collect::<String>()
}
