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

mod regexen;

///Represents the kinds of entities that can be extracted from a given text.
#[derive(Debug)]
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
#[derive(Debug)]
pub struct Entity {
    ///The kind of entity that was extracted.
    pub kind: EntityKind,
    ///The byte offsets between which the entity text is. The first index indicates the byte at the
    ///beginning of the extracted entity, but the second one is the byte index for the first
    ///character after the extracted entity (or one past the end of the string if the entity was at
    ///the end of the string).
    pub range: (usize, usize),
}

///Parses the given string for URLs.
pub fn url_entities(text: &str) -> Vec<Entity> {
    if text.is_empty() {
        return Vec::new();
    }

    let mut results: Vec<Entity> = Vec::new();
    let mut cursor = 0;

    loop {
        if cursor >= text.len() {
            break;
        }

        //save our matching substring since we modify cursor below
        let substr = &text[cursor..];

        let caps = regexen::RE_SIMPLIFIED_VALID_URL.captures(substr);
        if caps.is_none() {
            println!("no simplified url in '{}'", substr);
            break;
        }
        let caps = caps.unwrap();

        if caps.len() < 9 {
            println!("not enough captures in simplified url: {}", caps.len());
            break;
        }

        let current_cursor = cursor;
        cursor += caps.pos(0).unwrap().1;

        let preceding_range = caps.at(2);
        let url_range = caps.pos(3);
        let protocol_range = caps.pos(4);
        let domain_range = caps.pos(5);
        let path_range = caps.pos(7);

        //if protocol is missing and domain contains non-ascii chars, extract ascii-only
        //domains.
        if protocol_range.is_none() {
            if let Some(preceding) = preceding_range {
                if regexen::RE_URL_WO_PROTOCOL_INVALID_PRECEDING_CHARS.is_match(preceding) {
                    continue;
                }
            }

            if domain_range.is_none() { continue; }
            let mut domain_range = domain_range.unwrap();

            let mut loop_inserted = false;

            while domain_range.0 < domain_range.1 {
                //include succeeding character for validation
                let extra_char = if let Some(ch) = text[(cursor+domain_range.1)..].chars().next() {
                    ch.len_utf8()
                }
                else {
                    0
                };

                let url_range: (usize, usize);

                if let Some(caps) = regexen::RE_VALID_ASCII_DOMAIN.captures(&text[(current_cursor+domain_range.0)..(current_cursor+domain_range.1+extra_char)]) {
                    if let Some(range) = caps.pos(1) {
                        url_range = range;
                    }
                    else {
                        break;
                    }
                }
                else {
                    break;
                }

                loop_inserted = true;

                if path_range.is_some() ||
                   regexen::RE_VALID_SPECIAL_SHORT_DOMAIN.is_match(&substr[url_range.0..url_range.1]) ||
                   !regexen::RE_INVALID_SHORT_DOMAIN.is_match(&substr[url_range.0..url_range.1])
                {
                    results.push(Entity {
                        kind: EntityKind::Url,
                        range: url_range,
                    });
                }

                domain_range.0 = url_range.1;
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

                cursor = last_entity.range.1;
            }
        }
        else {
            if url_range.is_none() { continue; }
            if domain_range.is_none() { continue; }
            let mut url_range = url_range.unwrap();
            let domain_range = domain_range.unwrap();

            //in case of t.co URLs, don't allow additional path characters
            if let Some((_, to)) = regexen::RE_VALID_TCO_URL.find(&substr[url_range.0..url_range.1]) {
                url_range.1 = to;
            }
            else if !regexen::RE_URL_FOR_VALIDATION.is_match(&substr[domain_range.0..domain_range.1]) {
                continue;
            }

            results.push(Entity {
                kind: EntityKind::Url,
                range: (url_range.0 + current_cursor, url_range.1 + current_cursor),
            });
        }
    }

    results
}
