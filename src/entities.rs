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
use mime;

///Represents a hashtag or symbol extracted from another piece of text.
#[derive(Debug)]
pub struct HashtagEntity {
    ///The character indices where the hashtag is located. The first index is the location of the #
    ///or $ character; the second is the location of the first character following the hashtag.
    pub indices: (i32, i32),
    ///The text of the hashtag, without the leading # or $ character.
    pub text: String,
}

///Represents a piece of media attached to a tweet.
#[derive(Debug)]
pub struct MediaEntity {
    ///A shortened URL to display to clients.
    pub display_url: String,
    ///An expanded version of `display_url`; links to the media display page.
    pub expanded_url: String,
    ///A numeric ID for the media.
    pub id: i64,
    ///Character indices where the media URL is located. The first index is the location of the
    ///first character of the URL; the second is the location of the first character following the
    ///URL.
    pub indices: (i32, i32),
    ///A URL pointing directly to the media file. Uses HTTP as the protocol.
    ///
    ///For videos and GIFs, this link will be to a thumbnail of the media, and the real video link
    ///will be contained in `video_info`.
    pub media_url: String,
    ///A URL pointing directly to the media file. Uses HTTPS as the protocol.
    ///
    ///For videos and GIFs, this link will be to a thumbnail of the media, and the real video link
    ///will be contained in `video_info`.
    pub media_url_https: String,
    ///Various sizes available for the media file.
    pub sizes: MediaSizes,
    ///For tweets containing media that was originally associated with a different tweet, this
    ///contains the ID of the original tweet.
    pub source_status_id: Option<i64>,
    ///The type of media being represented.
    pub media_type: MediaType,
    ///The t.co link from the original text.
    pub url: String,
    ///For media entities corresponding to videos, this contains extra information about the linked
    ///video.
    pub video_info: Option<VideoInfo>,
}

///Represents the types of media that can be attached to a tweet.
#[derive(Debug)]
pub enum MediaType {
    ///A static image.
    Photo,
    ///A video.
    Video,
    ///An animated GIF, delivered as a video without audio.
    Gif,
}

///Represents the available sizes for a media file.
#[derive(Debug)]
pub struct MediaSizes {
    ///Information for a thumbnail-sized version of the media.
    pub thumb: MediaSize,
    ///Information for a small-sized version of the media.
    pub small: MediaSize,
    ///Information for a medium-sized version of the media.
    pub medium: MediaSize,
    ///Information for a large-sized version of the media.
    pub large: MediaSize,
}

///Represents how an image has been resized for a given size variant.
#[derive(Debug)]
pub enum ResizeMode {
    ///The media was resized to fit one dimension, keeping its aspect ratio.
    Fit,
    ///The media was cropped to fit a specific resolution.
    Crop,
}

///Represents the dimensions of a media file.
#[derive(Debug)]
pub struct MediaSize {
    ///The size variant's width in pixels.
    pub w: i32,
    ///The size variant's height in pixels.
    pub h: i32,
    ///The method used to obtain the given dimensions.
    pub resize: ResizeMode,
}

///Represents metadata specific to videos.
#[derive(Debug)]
pub struct VideoInfo {
    ///The aspect ratio of the video.
    pub aspect_ratio: (i32, i32),
    ///The duration of the video, in milliseconds.
    ///
    ///This field is not given for animated GIFs.
    pub duration_millis: Option<i32>,
    ///Information about various encodings available for the video.
    pub variants: Vec<VideoVariant>,
}

///Represents information about a specific encoding of a video.
#[derive(Debug)]
pub struct VideoVariant {
    ///The bitrate of the video. This value is present for GIFs, but it will be zero.
    pub bitrate: Option<i32>,
    ///The file format of the video variant.
    pub content_type: mime::Mime,
    ///The URL for the video variant.
    pub url: String,
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

///Represnts a user mention extracted from another piece of text.
#[derive(Debug)]
pub struct MentionEntity {
    ///Numeric ID of the mentioned user.
    pub id: i64,
    ///Character indices where the user mention is located in the original text. The first index is
    ///the location of the @ symbol; the second is the location of the first character following
    ///the user screen name.
    pub indices: (i32, i32),
    ///Display name of the mentioned user.
    pub name: String,
    ///Screen name of the mentioned user, without the leading @ symbol.
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
            source_status_id: field(input, "source_status_id").ok(),
            media_type: try!(field(input, "type")),
            url: try!(field(input, "url")),
            video_info: None,
        })
    }
}

impl FromJson for MediaType {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if let Some(s) = input.as_string() {
            if s == "photo" {
                Ok(MediaType::Photo)
            }
            else if s == "video" {
                Ok(MediaType::Video)
            }
            else if s == "animated_gif" {
                Ok(MediaType::Gif)
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

impl FromJson for VideoInfo {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse);
        }

        Ok(VideoInfo {
            aspect_ratio: try!(field(input, "aspect_ratio")),
            duration_millis: field(input, "duration_millis").ok(),
            variants: try!(field(input, "variants")),
        })
    }
}

impl FromJson for VideoVariant {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse);
        }

        Ok(VideoVariant {
            bitrate: field(input, "bitrate").ok(),
            content_type: try!(field(input, "content_type")),
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
