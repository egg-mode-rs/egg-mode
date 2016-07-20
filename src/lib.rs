extern crate hyper;
extern crate url;
extern crate time;
extern crate rand;
extern crate crypto;
extern crate rustc_serialize;

pub mod auth;
pub mod error;
mod links;

use std::borrow::Cow;
use std::collections::HashMap;
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

///Encodes the given string slice for transmission to Twitter.
pub fn percent_encode(src: &str) -> String {
    utf8_percent_encode(src, TwitterEncodeSet).collect::<String>()
}

type ParamList<'a> = HashMap<Cow<'a, str>, Cow<'a, str>>;

fn add_param<'a, K, V>(list: &mut ParamList<'a>, key: K, value: V) -> Option<Cow<'a, str>>
    where K: Into<Cow<'a, str>>,
          V: Into<Cow<'a, str>>
{
    list.insert(key.into(), value.into())
}
