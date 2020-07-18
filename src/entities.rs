// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Data structures containing extracted URL, mention, tag, and media information.
//!
//! These structures are meant to be received in an API call to describe the data they accompany.
//! For example, a `UrlEntity` describes a hyperlink in a tweet or user description text, and a
//! `HashtagEntity` describes a hashtag or stock symbol extracted from a tweet.
//!
//! For more information on the data in these structures, see Twitter's documentation for
//! [Entities Object][ent] and [Extended Entities Object][ext-ent].
//!
//! [ent]: https://developer.twitter.com/en/docs/tweets/data-dictionary/overview/entities-object
//! [ext-ent]: https://developer.twitter.com/en/docs/tweets/data-dictionary/overview/extended-entities-object
//!
//! ## Entity Ranges
//!
//! Entities that refer to elements within a text have a `range` field that contains the text span
//! that is being referenced. The numbers in question are byte offsets, so if you have an entity
//! that you'd like to slice out of the source text, you can use the indices directly in slicing
//! operations:
//!
//! ```rust
//! # use egg_mode::entities::HashtagEntity;
//! # let entity = HashtagEntity { range: (0, 0), text: "".to_string() };
//! # let text = "asdf";
//! let slice = &text[entity.range.0..entity.range.1];
//! ```
//!
//! ### Shortened, Display, and Expanded URLs
//!
//! URL and Media entities contain references to a URL within their parent text. However, due to
//! the nature of how Twitter handles URLs in tweets and user bios, each entity struct has three
//! URLs within it:
//!
//! - `url`: This is the `t.co` shortened URL as returned directly from twitter. This is what
//!   contributes to character count in tweets and user bios.
//! - `expanded_url`: This is the original URL the user entered in their tweet. While it is given
//!   to API client, Twitter recommends still sending users to the shortened link, for analytics
//!   purposes. Twitter Web uses this field to supply hover-text for where the URL resolves to.
//! - `display_url`: This is a truncated version of `expanded_url`, meant to be displayed inline
//!   with the parent text. This is useful to show users where the link resolves to, without
//!   potentially filling up a lot of space with the fullly expanded URL.
use mime;
use serde::{Serialize, Deserialize, Deserializer};

use crate::common::serde_via_string;

///Represents a hashtag or symbol extracted from another piece of text.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HashtagEntity {
    ///The byte offsets where the hashtag is located. The first index is the location of the # or $
    ///character; the second is the location of the first character following the hashtag.
    #[serde(rename = "indices")]
    pub range: (usize, usize),
    ///The text of the hashtag, without the leading # or $ character.
    pub text: String,
}

///Represents a piece of media attached to a tweet.
///
///The information in this struct is subtly different depending on what media is being referenced,
///and which entity container is holding this instance. For videos and GIFs, the `media_url` and
///`media_url_https` fields each link to a thumbnail image of the media, typically of the first
///frame. The real video information can be found on the `video_info` field, including various
///encodings if available.
///
///Image links available in `media_url` and `media_url_https` can be obtained in different sizes by
///appending a colon and one of the available sizes in the `MediaSizes` struct. For example, the
///cropped thumbnail can be viewed by appending `:thumb` to the end of the URL, and the full-size
///image can be viewed by appending `:large`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MediaEntity {
    ///A shortened URL to display to clients.
    pub display_url: String,
    ///An expanded version of `display_url`; links to the media display page.
    pub expanded_url: String,
    ///A numeric ID for the media.
    pub id: u64,
    ///The byte offsets where the media URL is located. The first index is the location of the
    ///first character of the URL; the second is the location of the first character following the
    ///URL.
    #[serde(rename = "indices")]
    pub range: (usize, usize),
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
    pub source_status_id: Option<u64>,
    ///The type of media being represented.
    #[serde(rename = "type")]
    pub media_type: MediaType,
    ///The t.co link from the original text.
    pub url: String,
    ///For media entities corresponding to videos, this contains extra information about the linked
    ///video.
    pub video_info: Option<VideoInfo>,
    ///Media alt text, if present.
    pub ext_alt_text: Option<String>,
}

///Represents the types of media that can be attached to a tweet.
#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
pub enum MediaType {
    ///A static image.
    #[serde(rename = "photo")]
    Photo,
    ///A video.
    #[serde(rename = "video")]
    Video,
    ///An animated GIF, delivered as a video without audio.
    #[serde(rename = "animated_gif")]
    Gif,
}

///Represents the available sizes for a media file.
#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
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
#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
pub enum ResizeMode {
    ///The media was resized to fit one dimension, keeping its aspect ratio.
    #[serde(rename = "fit")]
    Fit,
    ///The media was cropped to fit a specific resolution.
    #[serde(rename = "crop")]
    Crop,
}

///Represents the dimensions of a media file.
#[derive(Debug, Copy, Clone, Deserialize, Serialize)]
pub struct MediaSize {
    ///The size variant's width in pixels.
    pub w: i32,
    ///The size variant's height in pixels.
    pub h: i32,
    ///The method used to obtain the given dimensions.
    pub resize: ResizeMode,
}

///Represents metadata specific to videos.
#[derive(Debug, Clone, Deserialize, Serialize)]
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
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct VideoVariant {
    ///The bitrate of the video. This value is present for GIFs, but it will be zero.
    pub bitrate: Option<i32>,
    ///The file format of the video variant.
    #[serde(with = "serde_via_string")]
    pub content_type: mime::Mime,
    ///The URL for the video variant.
    pub url: String,
}

///Represents a link extracted from another piece of text.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct UrlEntity {
    ///A truncated URL meant to be displayed inline with the text.
    #[serde(default)]
    pub display_url: String,
    ///The URL that the t.co URL resolves to.
    ///
    ///Meant to be used as hover-text when a user mouses over a link.
    #[serde(default)]
    pub expanded_url: Option<String>,
    ///The byte offsets in the companion text where the URL was extracted from.
    #[serde(rename = "indices")]
    pub range: (usize, usize),
    ///The t.co URL extracted from the companion text.
    pub url: String,
}

///Represnts a user mention extracted from another piece of text.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MentionEntity {
    ///Numeric ID of the mentioned user.
    #[serde(deserialize_with = "nullable_id")] // Very rarely this field is null
    pub id: u64,
    ///The byte offsets where the user mention is located in the original text. The first index is
    ///the location of the @ symbol; the second is the location of the first character following
    ///the user screen name.
    #[serde(rename = "indices")]
    pub range: (usize, usize),
    ///Display name of the mentioned user.
    #[serde(deserialize_with = "nullable_str")] // Very rarely, this field is null
    pub name: String,
    ///Screen name of the mentioned user, without the leading @ symbol.
    pub screen_name: String,
}

fn nullable_id<'de, D>(deserializer: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

fn nullable_str<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}
