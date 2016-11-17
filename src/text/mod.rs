//! Helper methods for character counting and entity extraction.
//!
//! According to the official twitter-text Objective-C implementation [(GitHub)][twitter-text], the
//! publicly-exported methods are the following:
//!
//! [twitter-text]: https://github.com/twitter/twitter-text/tree/master/objc
//!
//! * `entitiesInText`: string -> array (presumably all extracted entities)
//! * `urlsInText`: string -> array (presumably all the URLs)
//! * `hashtagsInText`: string, bool -> array (presumably all the hashtags)
//!   * boolean `checkingURLOverlap` parameter extracts URLs first and ignores tags it finds
//!     inside if set (i think?)
//! * `symbolsInText`: string, bool -> array (presumably all the cashtags)
//!   * boolean `checkingURLOverlap` parameter extracts URLs first and ignores tags it finds
//!     inside if set (i think?)
//! * `mentionedScreenNamesInText`: string -> array (presumably all the mentions)
//! * `mentionsOrListsInText`: string -> array (presumably like above, but also @user/list slugs
//!   too)
//! * `repliedScreenNameInText`: string -> single entity (presumably the first mention if in reply
//!   position)
//!
//! * `tweetLength`: string -> int (also includes version with http and https URL lengths)
//! * `remainingCharacterCount`: string -> int (like above, includes alternate version with URL
//!   lengths)

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

///Parses the given string for all entities: URLs, hashtags, financial symbols ("cashtags"), user
///mentions, and list mentions.
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
pub fn url_entities(text: &str) -> Vec<Entity> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut results: Vec<Entity> = Vec::new();

    for caps in regexen::RE_SIMPLIFIED_VALID_URL.captures_iter(text) {
        if caps.len() < 9 {
            break;
        }

        let preceding_text = caps.at(2);
        let url_range = caps.pos(3);
        let protocol_range = caps.pos(4);
        let domain_range = caps.pos(5);
        let path_range = caps.pos(7);

        //if protocol is missing and domain contains non-ascii chars, extract ascii-only
        //domains.
        if protocol_range.is_none() {
            if let Some(preceding) = preceding_text {
                if regexen::RE_URL_WO_PROTOCOL_INVALID_PRECEDING_CHARS.is_match(preceding) {
                    continue;
                }
            }

            let mut domain_range = continue_opt!(domain_range);

            let mut loop_inserted = false;

            while domain_range.0 < domain_range.1 {
                //include succeeding character for validation
                let extra_char = if let Some(ch) = text[domain_range.1..].chars().next() {
                    ch.len_utf8()
                }
                else {
                    0
                };

                let domain_test = &text[domain_range.0..(domain_range.1+extra_char)];
                let caps = break_opt!(regexen::RE_VALID_ASCII_DOMAIN.captures(domain_test));
                let url_range = break_opt!(caps.pos(1));

                if path_range.is_some() ||
                   regexen::RE_VALID_SPECIAL_SHORT_DOMAIN.is_match(&domain_test[url_range.0..url_range.1]) ||
                   !regexen::RE_INVALID_SHORT_DOMAIN.is_match(&domain_test[url_range.0..url_range.1])
                {
                    loop_inserted = true;

                    results.push(Entity {
                        kind: EntityKind::Url,
                        range: (url_range.0 + domain_range.0, url_range.1 + domain_range.0),
                    });
                }

                domain_range.0 += url_range.1;
            }

            if !loop_inserted {
                continue;
            }

            if let Some(last_entity) = results.last_mut() {
                if let Some(path_range) = path_range {
                    if last_entity.range.1 == path_range.0 {
                        last_entity.range.1 += path_range.1 - path_range.0;
                    }
                }
            }
        }
        else {
            let mut url_range = continue_opt!(url_range);
            let domain_range = continue_opt!(domain_range);

            //in case of t.co URLs, don't allow additional path characters
            if let Some((_, to)) = regexen::RE_VALID_TCO_URL.find(&text[url_range.0..url_range.1]) {
                url_range.1 = to;
            }
            else if !regexen::RE_URL_FOR_VALIDATION.is_match(&text[domain_range.0..domain_range.1]) {
                continue;
            }

            results.push(Entity {
                kind: EntityKind::Url,
                range: url_range,
            });
        }
    }

    results
}

///Parses the given string for user and list mentions.
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
            cursor += 1;
        }
    }

    results
}

///Parses the given string for user mentions.
pub fn mention_entities(text: &str) -> Vec<Entity> {
    let mut results = mention_list_entities(text);

    results.retain(|e| e.kind == EntityKind::ScreenName);

    results
}

///Parses the given string for a user mention at the beginning of the text, if present.
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

///Returns how many characters the given text would be, after accounting for URL shortening.
pub fn character_count(text: &str, http_url_len: i32, https_url_len: i32) -> usize {
    //twitter uses code point counts after NFC normalization
    let mut text = text.nfc().collect::<String>();

    if text.is_empty() {
        return 0;
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

    text.len() + url_offset
}

///Returns how many characters would remain in a traditional 140-character tweet with the given
///text.
pub fn characters_remaining(text: &str, http_url_len: i32, https_url_len: i32) -> usize {
    140 - character_count(text, http_url_len, https_url_len)
}

#[cfg(test)]
mod test {
    extern crate yaml_rust;
    use super::*;

    use std::collections::HashSet;

    //file copied from https://github.com/twitter/twitter-text/tree/master/conformance
    //as of 2016-11-14
    const EXTRACT: &'static str = include_str!("extract.yml");

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
            let actual = symbol_entities(text, true).into_iter().map(|e| text[e.range.0..e.range.1].trim_matches('$')).collect::<HashSet<_>>();

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
                (text[input.range.0..input.range.1].trim_matches('$'), [input.range.0, input.range.1])
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
                                                     .map(|e| text[e.range.0..e.range.1].trim_matches(is_hash))
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
                                                     .map(|e| text[e.range.0..e.range.1].trim_matches(is_hash))
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
                (text[input.range.0..input.range.1].trim_matches(is_hash),
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
    }
}
