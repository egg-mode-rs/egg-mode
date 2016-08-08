//! Data structures containing extracted URL, mention, tag, and media information.
//!
//! These structures are meant to be received in an API call to describe the data they accompany.
//! For example, a `UrlEntity` describes a hyperlink in a tweet or user description text, and a
//! `HashtagEntity` describes a hashtag or stock symbol extracted from a tweet.
//!
//! For more information on the data in these structures, see Twitter's documentation for
//! [Entities][] and [Entities in Objects][obj].
//!
//! [Entities]: https://dev.twitter.com/overview/api/entities
//! [obj]: https://dev.twitter.com/overview/api/entities-in-twitter-objects

use common::*;
use error;
use error::Error::InvalidResponse;
use rustc_serialize::json;

#[allow(missing_docs)]
#[derive(Debug)]
pub struct HashtagEntity {
    pub indices: (i32, i32),
    pub text: String,
}

#[allow(missing_docs)]
#[derive(Debug)]
pub struct MediaEntity {
    pub display_url: String,
    pub expanded_url: String,
    pub id: i64,
    pub indices: (i32, i32),
    pub media_url: String,
    pub media_url_https: String,
    pub sizes: MediaSizes,
    pub source_status_id: i64,
    pub media_type: String,
    pub url: String,
}

#[allow(missing_docs)]
#[derive(Debug)]
pub struct MediaSizes {
    pub thumb: MediaSize,
    pub small: MediaSize,
    pub medium: MediaSize,
    pub large: MediaSize,
}

#[allow(missing_docs)]
#[derive(Debug)]
pub enum ResizeMode {
    Fit,
    Crop,
}

#[allow(missing_docs)]
#[derive(Debug)]
pub struct MediaSize {
    pub w: i32,
    pub h: i32,
    pub resize: ResizeMode,
}

///Represents a link extracted from another piece of text.
#[derive(Debug)]
pub struct UrlEntity {
    ///A truncated URL meant to be displayed inline with the text.
    pub display_url: String,
    ///The URL that the t.co URL resolves to.
    ///
    ///Meant to be used as hover-text when a user mouses over a link.
    pub expanded_url: String,
    ///The character positions in the companion text the URL was extracted from.
    pub indices: (i32, i32),
    ///The t.co URL extracted from the companion text.
    pub url: String,
}

#[allow(missing_docs)]
#[derive(Debug)]
pub struct MentionEntity {
    pub id: i64,
    pub indices: (i32, i32),
    pub name: String,
    pub screen_name: String,
}

impl FromJson for HashtagEntity {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse);
        }

        Ok(HashtagEntity {
            indices: try!(field(input, "indices")),
            text: try!(field(input, "text")),
        })
    }
}

impl FromJson for MediaEntity {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse);
        }

        Ok(MediaEntity {
            display_url: try!(field(input, "display_url")),
            expanded_url: try!(field(input, "expanded_url")),
            id: try!(field(input, "id")),
            indices: try!(field(input, "indices")),
            media_url: try!(field(input, "media_url")),
            media_url_https: try!(field(input, "media_url_https")),
            sizes: try!(field(input, "sizes")),
            source_status_id: try!(field(input, "source_status_id")),
            media_type: try!(field(input, "type")),
            url: try!(field(input, "url")),
        })
    }
}

impl FromJson for ResizeMode {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if let Some(s) = input.as_string() {
            if s == "fit" {
                Ok(ResizeMode::Fit)
            }
            else if s == "crop" {
                Ok(ResizeMode::Crop)
            }
            else {
                Err(InvalidResponse)
            }
        }
        else {
            Err(InvalidResponse)
        }
    }
}

impl FromJson for MediaSize {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse);
        }

        Ok(MediaSize {
            w: try!(field(input, "w")),
            h: try!(field(input, "h")),
            resize: try!(field(input, "resize")),
        })
    }
}

impl FromJson for MediaSizes {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse);
        }

        Ok(MediaSizes {
            thumb: try!(field(input, "thumb")),
            small: try!(field(input, "small")),
            medium: try!(field(input, "medium")),
            large: try!(field(input, "large")),
        })
    }
}

impl FromJson for UrlEntity {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse);
        }

        Ok(UrlEntity {
            display_url: try!(field(input, "display_url")),
            expanded_url: try!(field(input, "expanded_url")),
            indices: try!(field(input, "indices")),
            url: try!(field(input, "url")),
        })
    }
}

impl FromJson for MentionEntity {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse);
        }

        Ok(MentionEntity {
            id: try!(field(input, "id")),
            indices: try!(field(input, "indices")),
            name: try!(field(input, "name")),
            screen_name: try!(field(input, "screen_name")),
        })
    }
}
