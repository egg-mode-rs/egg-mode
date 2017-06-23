// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Helper methods for character counting and entity extraction.
//!
//! This is an implementation of the [twitter-text][] library that Twitter makes available as
//! reference code to demonstrate how they count characters in tweets and parse links, hashtags,
//! and user mentions.
//!
//! [twitter-text]: https://github.com/twitter/twitter-text
//!
//! The most likely entry point into this module is `character_count` or its close sibling,
//! `characters_remaining`. These functions parse the given text for URLs and returns a character
//! count according to [the rules set up by Twitter][character-counting], with the parsed URLs only
//! accounting for the given short-URL lengths. The remaining `*_entities` functions allow you to
//! parse a given text to see what entities of a given kind Twitter would extract from it, or for
//! all entities with the `entities` function.  These can be used, for example, to provide
//! auto-completion for a screen name or hashtag when composing a tweet.
//!
//! [character-counting]: https://dev.twitter.com/basics/counting-characters
//!
//! As the entities parsed by this module are simplified compared to the entities returned via the
//! Twitter API, they have been combined into one simplified `Entity` struct, with a companion
//! `EntityKind` enum to differentiate between them. See the struct documentation for `Entity` for
//! examples of how to use one.

///A convenience macro to break loops if the given value is `None`.
macro_rules! break_opt {
    ($input:expr) => {{
        if let Some(val) = $input {
            val
        }
        else { break; }
    }};
}

///A convenience macro to continue loops if the given value is `None`.
macro_rules! continue_opt {
    ($input:expr) => {{
        if let Some(val) = $input {
            val
        }
        else { continue; }
    }};
}

///A convenience macro to unwrap a given Option or return None from the containining function.
macro_rules! try_opt {
    ($input:expr) => {{
        if let Some(val) = $input {
            val
        }
        else { return None; }
    }};
}

use unicode_normalization::UnicodeNormalization;

mod regexen;

///Represents the kinds of entities that can be extracted from a given text.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Hash)]
pub enum EntityKind {
    ///A URL.
    Url,
    ///A user mention.
    ScreenName,
    ///A list mention, in the form "@user/list-name".
    ListName,
    ///A hashtag.
    Hashtag,
    ///A financial symbol ("cashtag").
    Symbol,
}

///Represents an entity extracted from a given text.
///
///This struct is meant to be returned from the entity parsing functions and linked to the source
///string that was parsed from the function in question. This is because the Entity struct itself
///only contains byte offsets for the string in question.
///
///# Examples
///
///To load the string in question, you can use the byte offsets directly, or use the `substr`
///method on the Entity itself:
///
///```rust
/// use egg_mode::text::hashtag_entities;
///
/// let text = "this is a #hashtag";
/// let results = hashtag_entities(text, true);
/// let entity = results.first().unwrap();
///
/// assert_eq!(&text[entity.range.0..entity.range.1], "#hashtag");
/// assert_eq!(entity.substr(text), "#hashtag");
///```
///
///Just having the byte offsets may seem like a roundabout way to store the extracted string, but
///with the byte offsets, you can also substitute in text decoration, like HTML links:
///
///```rust
/// use egg_mode::text::hashtag_entities;
///
/// let text = "this is a #hashtag";
/// let results = hashtag_entities(text, true);
/// let mut output = String::new();
/// let mut last_pos = 0;
///
/// for entity in results {
///     output.push_str(&text[last_pos..entity.range.0]);
///     //NOTE: this doesn't URL-encode the hashtag for the link
///     let tag = entity.substr(text);
///     let link = format!("<a href='https://twitter.com/#!/search?q={0}'>{0}</a>", tag);
///     output.push_str(&link);
///     last_pos = entity.range.1;
/// }
/// output.push_str(&text[last_pos..]);
///
/// assert_eq!(output, "this is a <a href='https://twitter.com/#!/search?q=#hashtag'>#hashtag</a>");
///```
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Copy, Clone, Hash)]
pub struct Entity {
    ///The kind of entity that was extracted.
    pub kind: EntityKind,
    ///The byte offsets between which the entity text is. The first index indicates the byte at the
    ///beginning of the extracted entity, but the second one is the byte index for the first
    ///character after the extracted entity (or one past the end of the string if the entity was at
    ///the end of the string). For hashtags and symbols, the range includes the # or $ character.
    pub range: (usize, usize),
}

impl Entity {
    ///Returns the substring matching this entity's byte offsets from the given text.
    ///
    ///# Panics
    ///
    ///This function will panic if the byte offsets in this entity do not match codepoint
    ///boundaries in the given text. This can happen if the text is not the original string that
    ///this entity was parsed from.
    pub fn substr<'a>(&self, text: &'a str) -> &'a str {
        &text[self.range.0..self.range.1]
    }
}

///Parses the given string for all entities: URLs, hashtags, financial symbols ("cashtags"), user
///mentions, and list mentions.
///
///This function is a shorthand for calling `url_entities`, `mention_list_entities`,
///`hashtag_entities`, and `symbol_entities` before merging the results together into a single Vec.
///The output is sorted so that entities are in that order (and individual kinds are ordered
///according to their appearance within the string) before exiting.
///
///# Example
///
///```rust
/// use egg_mode::text::{EntityKind, entities};
///
/// let text = "sample #text with a link to twitter.com";
/// let mut results = entities(text).into_iter();
///
/// let entity = results.next().unwrap();
/// assert_eq!(entity.kind, EntityKind::Url);
/// assert_eq!(entity.substr(text), "twitter.com");
///
/// let entity = results.next().unwrap();
/// assert_eq!(entity.kind, EntityKind::Hashtag);
/// assert_eq!(entity.substr(text), "#text");
///
/// assert_eq!(results.next(), None);
///```
pub fn entities(text: &str) -> Vec<Entity> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut results = url_entities(text);

    let urls = results.clone();

    results.extend(extract_hashtags(text, &urls));
    results.extend(extract_symbols(text, &urls));

    for mention in mention_list_entities(text) {
        let mut found = false;

        for existing in &results {
            if mention.range.0 <= existing.range.1 && existing.range.0 <= mention.range.1 {
                found = true;
                break;
            }
        }

        if !found {
            results.push(mention);
        }
    }

    results.sort();
    results
}

///Parses the given string for URLs.
///
///The entities returned from this function can be used to determine whether a url will be
///automatically shortened with a t.co link (in fact, this function is called from
///`character_count`), or to automatically add hyperlinks to URLs in a text if it hasn't been sent
///to Twitter yet.
///
///# Example
///
///```rust
/// use egg_mode::text::url_entities;
///
/// let text = "sample text with a link to twitter.com and one to rust-lang.org as well";
/// let mut results = url_entities(text).into_iter();
///
/// let entity = results.next().unwrap();
/// assert_eq!(entity.substr(text), "twitter.com");
///
/// let entity = results.next().unwrap();
/// assert_eq!(entity.substr(text), "rust-lang.org");
///
/// assert_eq!(results.next(), None);
///```
pub fn url_entities(text: &str) -> Vec<Entity> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut results: Vec<Entity> = Vec::new();
    let mut cursor = 0;

    while cursor < text.len() {
        let substr = &text[cursor..];
        let current_cursor = cursor;

        let caps = break_opt!(regexen::RE_SIMPLIFIED_VALID_URL.captures(substr));
        if caps.len() < 9 {
            break;
        }

        cursor += caps.pos(0).unwrap().1;

        let preceding_text = caps.at(2);
        let url_range = caps.pos(3);
        let protocol_range = caps.pos(4);
        let domain_range = caps.pos(5);
        let path_range = caps.pos(7);

        //if protocol is missing and domain contains non-ascii chars, extract ascii-only
        //domains.
        if protocol_range.is_none() {
            if let Some(preceding) = preceding_text {
                if !preceding.is_empty() && regexen::RE_URL_WO_PROTOCOL_INVALID_PRECEDING_CHARS.is_match(preceding) {
                    continue;
                }
            }

            let mut domain_range = continue_opt!(domain_range);

            let mut loop_inserted = false;

            while domain_range.0 < domain_range.1 {
                //include succeeding character for validation
                let extra_char = if let Some(ch) = substr[domain_range.1..].chars().next() {
                    ch.len_utf8()
                }
                else {
                    0
                };

                let domain_test = &substr[domain_range.0..(domain_range.1+extra_char)];
                let caps = break_opt!(regexen::RE_VALID_ASCII_DOMAIN.captures(domain_test));
                let url_range = break_opt!(caps.pos(1));
                let ascii_url = &domain_test[url_range.0..url_range.1];

                if path_range.is_some() ||
                   regexen::RE_VALID_SPECIAL_SHORT_DOMAIN.is_match(ascii_url) ||
                   !regexen::RE_INVALID_SHORT_DOMAIN.is_match(ascii_url)
                {
                    loop_inserted = true;

                    results.push(Entity {
                        kind: EntityKind::Url,
                        range: (current_cursor + domain_range.0 + url_range.0,
                                current_cursor + domain_range.0 + url_range.1),
                    });
                }

                domain_range.0 += url_range.1;
            }

            if !loop_inserted {
                continue;
            }

            if let Some(last_entity) = results.last_mut() {
                if let Some(path_range) = path_range {
                    if last_entity.range.1 == (current_cursor + path_range.0) {
                        last_entity.range.1 += path_range.1 - path_range.0;
                    }
                }

                cursor = last_entity.range.1;
            }
        }
        else {
            let mut url_range = continue_opt!(url_range);
            let domain_range = continue_opt!(domain_range);

            //in case of t.co URLs, don't allow additional path characters
            if let Some((_, to)) = regexen::RE_VALID_TCO_URL.find(&substr[url_range.0..url_range.1]) {
                url_range.1 = url_range.0 + to;
            }
            else if !regexen::RE_URL_FOR_VALIDATION.is_match(&substr[domain_range.0..domain_range.1]) {
                continue;
            }

            results.push(Entity {
                kind: EntityKind::Url,
                range: (current_cursor + url_range.0,
                        current_cursor + url_range.1),
            });
        }
    }

    results
}

///Parses the given string for user and list mentions.
///
///As the parsing rules for user mentions and list mentions, this function is able to extract both
///kinds at once. To differentiate between the two, check the entity's `kind` field.
///
///The entities returned by this function can be used to find mentions for hyperlinking, as well as
///to provide an autocompletion facility, if the byte-offset position of the cursor is known with
///relation to the full text.
///
///# Example
///
///```rust
/// use egg_mode::text::{EntityKind, mention_list_entities};
///
/// let text = "sample text with a mention for @twitter and a link to @rustlang/fakelist";
/// let mut results = mention_list_entities(text).into_iter();
///
/// let entity = results.next().unwrap();
/// assert_eq!(entity.kind, EntityKind::ScreenName);
/// assert_eq!(entity.substr(text), "@twitter");
///
/// let entity = results.next().unwrap();
/// assert_eq!(entity.kind, EntityKind::ListName);
/// assert_eq!(entity.substr(text), "@rustlang/fakelist");
///
/// assert_eq!(results.next(), None);
///```
pub fn mention_list_entities(text: &str) -> Vec<Entity> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut results = Vec::new();
    let mut cursor = 0usize;

    loop {
        if cursor >= text.len() {
            break;
        }

        //save our matching substring since we modify cursor below
        let substr = &text[cursor..];

        let caps = break_opt!(regexen::RE_VALID_MENTION_OR_LIST.captures(substr));

        if caps.len() < 5 {
            break;
        }

        let current_cursor = cursor;
        cursor += caps.pos(0).unwrap().1;

        if !regexen::RE_END_MENTION.is_match(&text[cursor..]) {
            let at_sign_range = continue_opt!(caps.pos(2));
            let screen_name_range = caps.pos(3);
            let list_name_range = caps.pos(4);

            if let Some((_, end)) = list_name_range {
                results.push(Entity {
                    kind: EntityKind::ListName,
                    range: (current_cursor + at_sign_range.0, current_cursor + end),
                });
            }
            else if let Some((_, end)) = screen_name_range {
                results.push(Entity {
                    kind: EntityKind::ScreenName,
                    range: (current_cursor + at_sign_range.0, current_cursor + end),
                });
            }
        }
        else {
            //Avoid matching the second username in @username@username
            cursor += if let Some(ch) = text[cursor..].chars().next() {
                ch.len_utf8()
            }
            else {
                1
            };
        }
    }

    results
}

///Parses the given string for user mentions.
///
///This is given as a convenience function for uses where mentions are needed but list mentions are
///not. This function effectively returns the same set as `mention_list_entities` but with list
///mentions removed.
///
///# Example
///
///```rust
/// use egg_mode::text::{EntityKind, mention_entities};
///
/// let text = "sample text with a mention for @twitter and a link to @rustlang/fakelist";
/// let mut results = mention_entities(text).into_iter();
///
/// let entity = results.next().unwrap();
/// assert_eq!(entity.kind, EntityKind::ScreenName);
/// assert_eq!(entity.substr(text), "@twitter");
///
/// assert_eq!(results.next(), None);
///```
pub fn mention_entities(text: &str) -> Vec<Entity> {
    let mut results = mention_list_entities(text);

    results.retain(|e| e.kind == EntityKind::ScreenName);

    results
}

///Parses the given string for a user mention at the beginning of the text, if present.
///
///This function is provided as a convenience method to see whether the given text counts as a
///tweet reply. If this function returns `Some` for a given draft tweet, then the final tweet is
///counted as a direct reply.
///
///Note that the entity returned by this function does not include the @-sign at the beginning of
///the mention.
///
///# Examples
///
///```rust
/// use egg_mode::text::reply_mention_entity;
///
/// let text = "@rustlang this is a reply";
/// let reply = reply_mention_entity(text).unwrap();
/// assert_eq!(reply.substr(text), "rustlang");
///
/// let text = ".@rustlang this is not a reply";
/// assert_eq!(reply_mention_entity(text), None);
///```
pub fn reply_mention_entity(text: &str) -> Option<Entity> {
    if text.is_empty() {
        return None;
    }

    let caps = try_opt!(regexen::RE_VALID_REPLY.captures(text));
    if caps.len() < 2 {
        return None;
    }

    let reply_range = try_opt!(caps.pos(1));

    if regexen::RE_END_MENTION.is_match(&text[reply_range.1..]) {
        return None;
    }

    Some(Entity {
        kind: EntityKind::ScreenName,
        range: reply_range,
    })
}

///Parses the given string for hashtags, optionally leaving out those that are part of URLs.
///
///The entities returned by this function can be used to find hashtags for hyperlinking, as well as
///to provide an autocompletion facility, if the byte-offset position of the cursor is known with
///relation to the full text.
///
///# Example
///
///With the `check_url_overlap` parameter, you can make sure you don't include text anchors from
///URLs:
///
///```rust
/// use egg_mode::text::hashtag_entities;
///
/// let text = "some #hashtag with a link to twitter.com/#anchor";
/// let mut results = hashtag_entities(text, true).into_iter();
///
/// let tag = results.next().unwrap();
/// assert_eq!(tag.substr(text), "#hashtag");
///
/// assert_eq!(results.next(), None);
///```
///
///If you pass `false` for that parameter, it won't parse for URLs to check for overlap:
///
///```rust
/// use egg_mode::text::hashtag_entities;
///
/// let text = "some #hashtag with a link to twitter.com/#anchor";
/// let mut results = hashtag_entities(text, false).into_iter();
///
/// let tag = results.next().unwrap();
/// assert_eq!(tag.substr(text), "#hashtag");
///
/// let tag = results.next().unwrap();
/// assert_eq!(tag.substr(text), "#anchor");
///
/// assert_eq!(results.next(), None);
///```
pub fn hashtag_entities(text: &str, check_url_overlap: bool) -> Vec<Entity> {
    if text.is_empty() {
        return Vec::new();
    }

    let url_entities = if check_url_overlap {
        url_entities(text)
    }
    else {
        Vec::new()
    };

    extract_hashtags(text, &url_entities)
}

fn extract_hashtags(text: &str, url_entities: &[Entity]) -> Vec<Entity> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut results = Vec::new();
    let mut cursor = 0usize;

    loop {
        if cursor >= text.len() {
            break;
        }

        let substr = &text[cursor..];

        let caps = break_opt!(regexen::RE_VALID_HASHTAG.captures(substr));

        if caps.len() < 3 {
            break;
        }

        let current_cursor = cursor;
        cursor += caps.pos(0).unwrap().1;

        let hashtag_range = break_opt!(caps.pos(1));
        let text_range = break_opt!(caps.pos(2));

        //note: check character after the # to make sure it's not \u{fe0f} or \u{20e3}
        //this is because the regex crate doesn't have lookahead assertions, which the objc impl
        //used to check for this
        if regexen::RE_HASHTAG_INVALID_INITIAL_CHARS.is_match(&substr[text_range.0..text_range.1]) {
            break;
        }

        let mut match_ok = true;

        for url in url_entities {
            if (hashtag_range.0 + current_cursor) <= url.range.1 &&
                url.range.0 <= (hashtag_range.1 + current_cursor)
            {
                //this hashtag is part of a url in the same text, skip it
                match_ok = false;
                break;
            }
        }

        if match_ok {
            if regexen::RE_END_HASHTAG.is_match(&substr[hashtag_range.1..]) {
                match_ok = false;
            }
        }

        if match_ok {
            results.push(Entity {
                kind: EntityKind::Hashtag,
                range: (hashtag_range.0 + current_cursor, hashtag_range.1 + current_cursor),
            });
        }
    }

    results
}

///Parses the given string for financial symbols ("cashtags"), optionally leaving out those that
///are part of URLs.
///
///The entities returned by this function can be used to find symbols for hyperlinking, as well as
///to provide an autocompletion facility, if the byte-offset position of the cursor is known with
///relation to the full text.
///
///The `check_url_overlap` parameter behaves the same way as in `hashtag_entities`; when `true`, it
///will parse URLs from the text first and check symbols to make sure they don't overlap with any
///extracted URLs.
///
///# Example
///
///```rust
/// use egg_mode::text::symbol_entities;
///
/// let text = "some $stock symbol";
/// let mut results = symbol_entities(text, true).into_iter();
///
/// let tag = results.next().unwrap();
/// assert_eq!(tag.substr(text), "$stock");
///
/// assert_eq!(results.next(), None);
///```
pub fn symbol_entities(text: &str, check_url_overlap: bool) -> Vec<Entity> {
    if text.is_empty() {
        return Vec::new();
    }

    let url_entities = if check_url_overlap {
        url_entities(text)
    }
    else {
        Vec::new()
    };

    extract_symbols(text, &url_entities)
}

fn extract_symbols(text: &str, url_entities: &[Entity]) -> Vec<Entity> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut results = Vec::new();

    for caps in regexen::RE_VALID_SYMBOL.captures_iter(text) {
        if caps.len() < 2 { break; }

        let text_range = break_opt!(caps.pos(0));
        let symbol_range = break_opt!(caps.pos(1));
        let mut match_ok = true;

        //check the text after the match to see if it's valid; this is because i can't use
        //lookahead assertions in the regex crate and this is how it's implemented in the obj-c
        //version
        if !regexen::RE_END_SYMBOL.is_match(&text[text_range.1..]) {
            match_ok = false;
        }

        for url in url_entities {
            if symbol_range.0 <= url.range.1 && url.range.0 <= symbol_range.1 {
                //this symbol is part of a url in the same text, skip it
                match_ok = false;
                break;
            }
        }

        if match_ok {
            results.push(Entity {
                kind: EntityKind::Symbol,
                range: symbol_range,
            });
        }
    }

    results
}

///Returns how many characters the given text would be, after accounting for URL shortening. Also
///returns an indicator of whether the given text is a valid length for a tweet.
///
///For the `http_url_len` and `https_url_len` parameters, call [`service::config`][] and use the
///`short_url_len` and `short_url_len_https` fields on the struct that's returned. If you want to
///perform these checks offline, twitter-text's sample code and tests assume 23 characters for both
///sizes. At the time of this writing (2016-11-28), those numbers were also being returned from the
///service itself.
///
///[`service::config`]: ../service/fn.config.html
///
///# Examples
///
///```rust
/// use egg_mode::text::character_count;
///
/// let (count, _) = character_count("This is a test.", 23, 23);
/// assert_eq!(count, 15);
///
/// // URLs get replaced by a t.co URL of the given length
/// let (count, _) = character_count("test.com", 23, 23);
/// assert_eq!(count, 23);
///
/// // Multiple URLs get shortened individually
/// let (count, _) =
///     character_count("Test https://test.com test https://test.com test.com test", 23, 23);
/// assert_eq!(count, 86);
///```
pub fn character_count(text: &str, http_url_len: i32, https_url_len: i32) -> (usize, bool) {
    //twitter uses code point counts after NFC normalization
    let mut text = text.nfc().collect::<String>();

    if text.is_empty() {
        return (0, false);
    }

    let mut url_offset = 0usize;
    let entities = url_entities(&text);

    for url in &entities {
        let substr = &text[url.range.0..url.range.1];
        if substr.contains("https") {
            url_offset += https_url_len as usize;
        }
        else {
            url_offset += http_url_len as usize;
        }
    }

    //put character removal in a second pass so we don't mess up the byte offsets
    for url in entities.iter().rev() {
        text.drain(url.range.0..url.range.1);
    }

    //make sure to count codepoints, not bytes
    let len = text.chars().count() + url_offset;

    (len, len > 0 && len <= 140)
}

///Returns how many characters would remain in a traditional 140-character tweet with the given
///text. Also returns an indicator of whether the given text is a valid length for a tweet.
///
///This function exists as a sort of convenience method to allow clients to call one uniform method
///to show a remaining character count on a tweet compose box, and to conditionally enable a
///"submit" button.
///
///For the `http_url_len` and `https_url_len` parameters, call [`service::config`][] and use the
///`short_url_len` and `short_url_len_https` fields on the struct that's returned. If you want to
///perform these checks offline, twitter-text's sample code and tests assume 23 characters for both
///sizes. At the time of this writing (2016-11-28), those numbers were also being returned from the
///service itself.
///
///If you're writing text for a direct message and want to know how many characters are available
///in that context, see [`service::config`][] and the `dm_text_character_limit` on the struct
///returned by that function, then call [`character_count`][] and subtract the result from the
///configuration value.
///
///[`service::config`]: ../service/fn.config.html
///[`character_count`]: fn.character_count.html
///
///# Examples
///
///```rust
/// use egg_mode::text::characters_remaining;
///
/// let (count, _) = characters_remaining("This is a test.", 23, 23);
/// assert_eq!(count, 140 - 15);
///
/// // URLs get replaced by a t.co URL of the given length
/// let (count, _) = characters_remaining("test.com", 23, 23);
/// assert_eq!(count, 140 - 23);
///
/// // Multiple URLs get shortened individually
/// let (count, _) =
///     characters_remaining("Test https://test.com test https://test.com test.com test", 23, 23);
/// assert_eq!(count, 140 - 86);
///```
pub fn characters_remaining(text: &str, http_url_len: i32, https_url_len: i32) -> (usize, bool) {
    let (len, is_valid) = character_count(text, http_url_len, https_url_len);

    (140 - len, is_valid)
}

#[cfg(test)]
mod test {
    extern crate yaml_rust;
    use super::*;

    use std::collections::HashSet;

    //files copied from https://github.com/twitter/twitter-text/tree/master/conformance
    //as of 2016-11-14
    const EXTRACT: &'static str = include_str!("extract.yml");
    const VALIDATE: &'static str = include_str!("validate.yml");
    const TLDS: &'static str = include_str!("tlds.yml");

    fn byte_to_char(text: &str, byte_offset: usize) -> usize {
        if byte_offset == text.len() {
            text.chars().count()
        }
        else {
            text.char_indices()
                .enumerate()
                .find(|&(_ch_idx, (by_idx, _))| by_idx == byte_offset)
                .unwrap().0
        }
    }

    #[test]
    fn extract() {
        let tests = yaml_rust::YamlLoader::load_from_str(EXTRACT).unwrap();
        let tests = tests.first().unwrap();
        let ref tests = tests["tests"];

        assert!(tests.as_hash().is_some(), "could not load tests document");

        for test in tests["cashtags"].as_vec().expect("tests 'cashtags' could not be loaded") {
            let description = test["description"].as_str().expect("test was missing 'description");
            let text = test["text"].as_str().expect("test was missing 'text'");
            let expected = test["expected"].as_vec().expect("test was missing 'expected'");
            let expected = expected.iter()
                                   .map(|s| s.as_str().expect("non-string found in 'expected'"))
                                   .collect::<HashSet<_>>();
            let actual = symbol_entities(text, true).into_iter().map(|e| e.substr(text).trim_matches('$')).collect::<HashSet<_>>();

            for extra in actual.difference(&expected) {
                panic!("test \"{}\" failed on text \"{}\": extracted erroneous symbol \"{}\"",
                       description, text, extra);
            }

            for missed in expected.difference(&actual) {
                panic!("test \"{}\" failed on text \"{}\": did not extract symbol \"{}\"",
                       description, text, missed);
            }
        }

        for test in tests["cashtags_with_indices"].as_vec().expect("tests 'cashtags_with_indices' could not be loaded") {
            fn cashtag_pair(input: &yaml_rust::Yaml) -> (&str, [usize; 2]) {
                let tag = input["cashtag"].as_str().expect("test was missing 'expected.cashtag'");
                let indices = input["indices"].as_vec().expect("test was missing 'expected.indices'");
                let indices = indices.iter()
                                     .map(|it| it.as_i64().expect("'expected.indices' was not an int") as usize)
                                     .collect::<Vec<_>>();

                (tag, [indices[0], indices[1]])
            }

            fn cashtag_entity<'a>(input: Entity, text: &'a str) -> (&'a str, [usize; 2]) {
                (input.substr(text).trim_matches('$'), [input.range.0, input.range.1])
            }

            let description = test["description"].as_str().expect("test was missing 'description");
            let text = test["text"].as_str().expect("test was missing 'text'");
            let expected = test["expected"].as_vec().expect("test was missing 'expected'");
            let expected = expected.iter().map(cashtag_pair).collect::<HashSet<_>>();
            let actual = symbol_entities(text, true).into_iter()
                                                    .map(|s| cashtag_entity(s, text))
                                                    .collect::<HashSet<_>>();

            for extra in actual.difference(&expected) {
                panic!("test \"{}\" failed on text \"{}\": extracted erroneous symbol \"{:?}\"",
                       description, text, extra);
            }

            for missed in expected.difference(&actual) {
                panic!("test \"{}\" failed on text \"{}\": did not extract symbol \"{:?}\"",
                       description, text, missed);
            }
        }

        for test in tests["hashtags"].as_vec().expect("tests 'hashtags' could not be loaded") {
            fn is_hash(input: char) -> bool {
                match input {
                    '#' | '＃' => true,
                    _ => false,
                }
            }

            let description = test["description"].as_str().expect("test was missing 'description");
            let text = test["text"].as_str().expect("test was missing 'text'");
            let expected = test["expected"].as_vec().expect("test was missing 'expected'");
            let expected = expected.iter()
                                   .map(|s| s.as_str().expect("non-string found in 'expected'"))
                                   .collect::<HashSet<_>>();
            let actual = hashtag_entities(text, true).into_iter()
                                                     .map(|e| e.substr(text).trim_matches(is_hash))
                                                     .collect::<HashSet<_>>();

            for extra in actual.difference(&expected) {
                panic!("test \"{}\" failed on text \"{}\": extracted erroneous hashtag \"{}\"",
                       description, text, extra);
            }

            for missed in expected.difference(&actual) {
                panic!("test \"{}\" failed on text \"{}\": did not extract hashtag \"{}\"",
                       description, text, missed);
            }
        }

        for test in tests["hashtags_from_astral"].as_vec().expect("tests 'hashtags_from_astral' could not be loaded") {
            fn is_hash(input: char) -> bool {
                match input {
                    '#' | '＃' => true,
                    _ => false,
                }
            }

            let description = test["description"].as_str().expect("test was missing 'description");
            let text = test["text"].as_str().expect("test was missing 'text'");
            let expected = test["expected"].as_vec().expect("test was missing 'expected'");
            let expected = expected.iter()
                                   .map(|s| s.as_str().expect("non-string found in 'expected'"))
                                   .collect::<HashSet<_>>();
            let actual = hashtag_entities(text, true).into_iter()
                                                     .map(|e| e.substr(text).trim_matches(is_hash))
                                                     .collect::<HashSet<_>>();

            for extra in actual.difference(&expected) {
                panic!("test \"{}\" failed on text \"{}\": extracted erroneous hashtag \"{}\"",
                       description, text, extra);
            }

            for missed in expected.difference(&actual) {
                panic!("test \"{}\" failed on text \"{}\": did not extract hashtag \"{}\"",
                       description, text, missed);
            }
        }

        for test in tests["hashtags_with_indices"].as_vec().expect("tests 'hashtags_with_indices' could not be loaded") {
            fn is_hash(input: char) -> bool {
                match input {
                    '#' | '＃' => true,
                    _ => false,
                }
            }

            fn hashtag_pair(input: &yaml_rust::Yaml) -> (&str, [usize; 2]) {
                let tag = input["hashtag"].as_str().expect("test was missing 'expected.hashtag'");
                let indices = input["indices"].as_vec().expect("test was missing 'expected.indices'");
                let indices = indices.iter()
                                     .map(|it| it.as_i64().expect("'expected.indices' was not an int") as usize)
                                     .collect::<Vec<_>>();

                (tag, [indices[0], indices[1]])
            }

            fn hashtag_entity<'a>(input: Entity, text: &'a str) -> (&'a str, [usize; 2]) {
                (input.substr(text).trim_matches(is_hash),
                 [byte_to_char(text, input.range.0), byte_to_char(text, input.range.1)])
            }

            let description = test["description"].as_str().expect("test was missing 'description");
            let text = test["text"].as_str().expect("test was missing 'text'");
            let expected = test["expected"].as_vec().expect("test was missing 'expected'");
            let expected = expected.iter().map(hashtag_pair).collect::<HashSet<_>>();
            let actual = hashtag_entities(text, true).into_iter()
                                                     .map(|e| hashtag_entity(e, text))
                                                     .collect::<HashSet<_>>();

            for extra in actual.difference(&expected) {
                panic!("test \"{}\" failed on text \"{}\": extracted erroneous hashtag \"{:?}\"",
                       description, text, extra);
            }

            for missed in expected.difference(&actual) {
                panic!("test \"{}\" failed on text \"{}\": did not extract hashtag \"{:?}\"",
                       description, text, missed);
            }
        }

        for test in tests["mentions"].as_vec().expect("tests 'mentions' could not be loaded") {
            fn is_at(input: char) -> bool {
                match input {
                    '@' | '＠' => true,
                    _ => false,
                }
            }

            let description = test["description"].as_str().expect("test was missing 'description");
            let text = test["text"].as_str().expect("test was missing 'text'");
            let expected = test["expected"].as_vec().expect("test was missing 'expected'");
            let expected = expected.iter()
                                   .map(|s| s.as_str().expect("non-string found in 'expected'"))
                                   .collect::<HashSet<_>>();
            let actual = mention_entities(text).into_iter()
                                               .map(|e| e.substr(text).trim_matches(is_at))
                                               .collect::<HashSet<_>>();

            for extra in actual.difference(&expected) {
                panic!("test \"{}\" failed on text \"{}\": extracted erroneous mention \"{}\"",
                       description, text, extra);
            }

            for missed in expected.difference(&actual) {
                panic!("test \"{}\" failed on text \"{}\": did not extract mention \"{}\"",
                       description, text, missed);
            }
        }

        for test in tests["mentions_with_indices"].as_vec().expect("tests 'mentions_with_indices' could not be loaded") {
            fn is_at(input: char) -> bool {
                match input {
                    '@' | '＠' => true,
                    _ => false,
                }
            }

            fn mention_pair(input: &yaml_rust::Yaml) -> (&str, [usize; 2]) {
                let name = input["screen_name"].as_str().expect("test was missing 'expected.screen_name'");
                let indices = input["indices"].as_vec().expect("test was missing 'expected.indices'");
                let indices = indices.iter()
                                     .map(|it| it.as_i64().expect("'expected.indices' was not an int") as usize)
                                     .collect::<Vec<_>>();

                (name, [indices[0], indices[1]])
            }

            fn mention_entity<'a>(input: Entity, text: &'a str) -> (&'a str, [usize; 2]) {
                (input.substr(text).trim_matches(is_at),
                 [byte_to_char(text, input.range.0), byte_to_char(text, input.range.1)])
            }

            let description = test["description"].as_str().expect("test was missing 'description");
            let text = test["text"].as_str().expect("test was missing 'text'");
            let expected = test["expected"].as_vec().expect("test was missing 'expected'");
            let expected = expected.iter().map(mention_pair).collect::<HashSet<_>>();
            let actual = mention_entities(text).into_iter()
                                               .map(|e| mention_entity(e, text))
                                               .collect::<HashSet<_>>();

            for extra in actual.difference(&expected) {
                panic!("test \"{}\" failed on text \"{}\": extracted erroneous mention \"{:?}\"",
                       description, text, extra);
            }

            for missed in expected.difference(&actual) {
                panic!("test \"{}\" failed on text \"{}\": did not extract mention \"{:?}\"",
                       description, text, missed);
            }
        }

        for test in tests["mentions_or_lists_with_indices"].as_vec().expect("tests 'mentions_or_lists_with_indices' could not be loaded") {
            fn is_at(input: char) -> bool {
                match input {
                    '@' | '＠' => true,
                    _ => false,
                }
            }

            fn mention_pair(input: &yaml_rust::Yaml) -> (String, [usize; 2]) {
                let name = input["screen_name"].as_str().expect("test was missing 'expected.screen_name'");
                let list = input["list_slug"].as_str().expect("test was missing 'expected.list_slug'");
                let name = name.to_owned() + list;
                let indices = input["indices"].as_vec().expect("test was missing 'expected.indices'");
                let indices = indices.iter()
                                     .map(|it| it.as_i64().expect("'expected.indices' was not an int") as usize)
                                     .collect::<Vec<_>>();

                (name, [indices[0], indices[1]])
            }

            fn mention_entity(input: Entity, text: &str) -> (String, [usize; 2]) {
                (input.substr(text).trim_matches(is_at).to_owned(),
                 [byte_to_char(text, input.range.0), byte_to_char(text, input.range.1)])
            }

            let description = test["description"].as_str().expect("test was missing 'description");
            let text = test["text"].as_str().expect("test was missing 'text'");
            let expected = test["expected"].as_vec().expect("test was missing 'expected'");
            let expected = expected.iter().map(mention_pair).collect::<HashSet<_>>();
            let actual = mention_list_entities(text).into_iter()
                                                    .map(|e| mention_entity(e, text))
                                                    .collect::<HashSet<_>>();

            for extra in actual.difference(&expected) {
                panic!("test \"{}\" failed on text \"{}\": extracted erroneous mention \"{:?}\"",
                       description, text, extra);
            }

            for missed in expected.difference(&actual) {
                panic!("test \"{}\" failed on text \"{}\": did not extract mention \"{:?}\"",
                       description, text, missed);
            }
        }

        for test in tests["replies"].as_vec().expect("tests 'replies' could not be loaded") {
            use self::yaml_rust::Yaml;

            fn is_at(input: char) -> bool {
                match input {
                    '@' | '＠' => true,
                    _ => false,
                }
            }

            let description = test["description"].as_str().expect("test was missing 'description");
            let text = test["text"].as_str().expect("test was missing 'text'");
            let expected = match test["expected"] {
                Yaml::String(ref val) => Some(&val[..]),
                Yaml::Null | Yaml::BadValue => None,
                _ => panic!("unexpected value for 'expected'"),
            };
            let actual = reply_mention_entity(text).map(|s| s.substr(text).trim_matches(is_at));

            if expected != actual {
                panic!("test \"{}\" failed on text \"{}\": expected '{:?}', exracted '{:?}'",
                       description, text, expected, actual);
            }
        }

        for test in tests["urls"].as_vec().expect("tests 'urls' could not be loaded") {
            let description = test["description"].as_str().expect("test was missing 'description");
            let text = test["text"].as_str().expect("test was missing 'text'");
            let expected = test["expected"].as_vec().expect("test was missing 'expected'");
            let expected = expected.iter()
                                   .map(|s| s.as_str().expect("non-string found in 'expected'"))
                                   .collect::<HashSet<_>>();
            let actual = url_entities(text).into_iter()
                                               .map(|e| e.substr(text))
                                               .collect::<HashSet<_>>();

            for extra in actual.difference(&expected) {
                panic!("test \"{}\" failed on text \"{}\": extracted erroneous url \"{}\"",
                       description, text, extra);
            }

            for missed in expected.difference(&actual) {
                panic!("test \"{}\" failed on text \"{}\": did not extract url \"{}\"",
                       description, text, missed);
            }
        }

        for test in tests["urls_with_indices"].as_vec().expect("tests 'urls_with_indices' could not be loaded") {
            fn url_pair(input: &yaml_rust::Yaml) -> (&str, [usize; 2]) {
                let name = input["url"].as_str().expect("test was missing 'expected.url'");
                let indices = input["indices"].as_vec().expect("test was missing 'expected.indices'");
                let indices = indices.iter()
                                     .map(|it| it.as_i64().expect("'expected.indices' was not an int") as usize)
                                     .collect::<Vec<_>>();

                (name, [indices[0], indices[1]])
            }

            fn url_entity<'a>(input: Entity, text: &'a str) -> (&'a str, [usize; 2]) {
                (input.substr(text),
                 [byte_to_char(text, input.range.0), byte_to_char(text, input.range.1)])
            }

            let description = test["description"].as_str().expect("test was missing 'description");
            let text = test["text"].as_str().expect("test was missing 'text'");
            let expected = test["expected"].as_vec().expect("test was missing 'expected'");
            let expected = expected.iter().map(url_pair).collect::<HashSet<_>>();
            let actual = url_entities(text).into_iter()
                                           .map(|e| url_entity(e, text))
                                           .collect::<HashSet<_>>();

            for extra in actual.difference(&expected) {
                panic!("test \"{}\" failed on text \"{}\": extracted erroneous url \"{:?}\"",
                       description, text, extra);
            }

            for missed in expected.difference(&actual) {
                panic!("test \"{}\" failed on text \"{}\": did not extract url \"{:?}\"",
                       description, text, missed);
            }
        }
    }

    #[test]
    fn validate() {
        let tests = yaml_rust::YamlLoader::load_from_str(VALIDATE).unwrap();
        let tests = tests.first().unwrap();
        let ref tests = tests["tests"];

        assert!(tests.as_hash().is_some(), "could not load tests document");

        for test in tests["tweets"].as_vec().expect("tests 'tweets' could not be loaded") {
            let description = test["description"].as_str().expect("test was missing 'description");
            let text = test["text"].as_str().expect("test was missing 'text'");
            let expected = test["expected"].as_bool().expect("test was missing 'expected'");

            //23 is the default character count in the obj-c implementation, tho at time of writing
            //(2016-11-21) i think these lengths have bumped up to 24
            let (count, is_valid) = character_count(text, 23, 23);

            assert_eq!(expected, is_valid, "test '{}' failed with text '{}', counted {} characters",
                       description, text, count);
        }

        for test in tests["lengths"].as_vec().expect("tests 'lengths' could not be loaded") {
            let description = test["description"].as_str().expect("test was missing 'description");
            let text = test["text"].as_str().expect("test was missing 'text'");
            let expected = test["expected"].as_i64().expect("test was missing 'expected'");

            //23 is the default character count in the obj-c implementation, tho at time of writing
            //(2016-11-21) i think these lengths have bumped up to 24
            let (count, _) = character_count(text, 23, 23);

            assert_eq!(expected as usize, count, "test '{}' failed with text '{}'", description, text);
        }

        for test in tests["usernames"].as_vec().expect("tests 'usernames' could not be loaded") {
            let description = test["description"].as_str().expect("test was missing 'description");
            let text = test["text"].as_str().expect("test was missing 'text'");
            let expected = test["expected"].as_bool().expect("test was missing 'expected'");

            let actual = mention_entities(text);

            match actual.first() {
                Some(entity) => {
                    let name = entity.substr(text);
                    if (name == text) != expected {
                        panic!("test '{}' failed: extracted username '{}' from '{}' failed to match expectation {}",
                               description, name, text, expected);
                    }
                },
                None => if expected {
                    panic!("test '{}' failed: failed to extract valid username from '{}'",
                           description, text);
                },
            }
        }

        for test in tests["lists"].as_vec().expect("tests 'lists' could not be loaded") {
            let description = test["description"].as_str().expect("test was missing 'description");
            let text = test["text"].as_str().expect("test was missing 'text'");
            let expected = test["expected"].as_bool().expect("test was missing 'expected'");

            let actual = mention_list_entities(text);

            match actual.first() {
                Some(entity) if entity.kind == EntityKind::ListName => {
                    let name = entity.substr(text);
                    if (name == text) != expected {
                        panic!("test '{}' failed: extracted list name '{}' from '{}' failed to match expectation {}",
                               description, name, text, expected);
                    }
                },
                _ => if expected {
                    panic!("test '{}' failed: failed to extract valid list name from '{}'",
                           description, text);
                },
            }
        }

        for test in tests["hashtags"].as_vec().expect("tests 'hashtags' could not be loaded") {
            let description = test["description"].as_str().expect("test was missing 'description");
            let text = test["text"].as_str().expect("test was missing 'text'");
            let expected = test["expected"].as_bool().expect("test was missing 'expected'");

            let actual = hashtag_entities(text, false);

            match actual.first() {
                Some(entity) => {
                    let name = entity.substr(text);
                    if (name == text) != expected {
                        panic!("test '{}' failed: extracted hashtag '{}' from '{}' failed to match expectation {}",
                               description, name, text, expected);
                    }
                },
                None => if expected {
                    panic!("test '{}' failed: failed to extract valid hashtag from '{}'",
                           description, text);
                },
            }
        }
    }

    #[test]
    fn tlds() {
        let tests = yaml_rust::YamlLoader::load_from_str(TLDS).unwrap();
        let tests = tests.first().unwrap();
        let ref tests = tests["tests"];

        assert!(tests.as_hash().is_some(), "could not load tests document");

        for test in tests["country"].as_vec().expect("tests 'country' could not be loaded") {
            let description = test["description"].as_str().expect("test was missing 'description");
            let text = test["text"].as_str().expect("test was missing 'text'");
            let expected = test["expected"].as_vec().expect("test was missing 'expected'");
            let expected = expected.iter()
                                   .map(|s| s.as_str().expect("non-string found in 'expected'"))
                                   .collect::<HashSet<_>>();
            let actual = url_entities(text).into_iter().map(|e| e.substr(text)).collect::<HashSet<_>>();

            for extra in actual.difference(&expected) {
                panic!("test \"{}\" failed on text \"{}\": extracted erroneous symbol \"{}\"",
                       description, text, extra);
            }

            for missed in expected.difference(&actual) {
                panic!("test \"{}\" failed on text \"{}\": did not extract symbol \"{}\"",
                       description, text, missed);
            }
        }

        for test in tests["generic"].as_vec().expect("tests 'generic' could not be loaded") {
            let description = test["description"].as_str().expect("test was missing 'description");
            let text = test["text"].as_str().expect("test was missing 'text'");
            let expected = test["expected"].as_vec().expect("test was missing 'expected'");
            let expected = expected.iter()
                                   .map(|s| s.as_str().expect("non-string found in 'expected'"))
                                   .collect::<HashSet<_>>();
            let actual = url_entities(text).into_iter().map(|e| e.substr(text)).collect::<HashSet<_>>();

            for extra in actual.difference(&expected) {
                panic!("test \"{}\" failed on text \"{}\": extracted erroneous symbol \"{}\"",
                       description, text, extra);
            }

            for missed in expected.difference(&actual) {
                panic!("test \"{}\" failed on text \"{}\": did not extract symbol \"{}\"",
                       description, text, missed);
            }
        }
    }
}
