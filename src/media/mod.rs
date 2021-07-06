//! Functionality to upload images, GIFs, and videos that can be attached to tweets.
//!
//! Tweet media is uploaded separately from the act of posting the tweet itself.
//! In order to attach an image to a new tweet, you need to upload it first,
//! then take the Media ID that Twitter generates and reference that when posting the tweet.
//! The media id is returned as part of the result of a call to [`upload_media`].
//!
//! Here's a basic example of uploading an image and attaching to a tweet:
//!
//! ```rust,no_run
//! # use egg_mode::Token;
//! # #[tokio::main]
//! # async fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # let token: Token = unimplemented!();
//! use egg_mode::media::{upload_media, media_types};
//! use egg_mode::tweet::DraftTweet;
//!
//! let image = b"some image bytes"; //pretend we loaded an image file into this
//! let handle = upload_media(image, &media_types::image_png(), &token).await?;
//! let draft = DraftTweet::new("Hey, check out this cute cat!");
//! draft.add_media(handle.id);
//! let tweet = draft.send(&token).await?;
//! # }
//! ```

use std::time::{Duration, Instant};

use base64;
use serde::de::Error;
use serde::{Deserialize, Deserializer};

use crate::common::*;
use crate::{auth, error, links};

use mime;

/// A collection of convenience functions that return media types accepted by Twitter.
///
/// These are convenience types that can be handed to [`upload_media`] to set the right
/// media type of a piece of media. The functions in the module correspond to media types
/// that Twitter is known to accept.
///
/// Note that using `image_gif` and `video_mp4` will automatically set the upload's
/// `media_category` to `tweet_gif` and `tweet_video` respectively, allowing
/// larger file sizes and extra processing time.
pub mod media_types {
    use mime::{self, Mime};

    /// PNG images.
    pub fn image_png() -> Mime {
        mime::IMAGE_PNG
    }

    /// JPG images.
    pub fn image_jpg() -> Mime {
        mime::IMAGE_JPEG
    }

    /// WEBP images.
    pub fn image_webp() -> Mime {
        "image/webp".parse().unwrap()
    }

    /// Animated GIF images.
    pub fn image_gif() -> Mime {
        mime::IMAGE_GIF
    }

    /// MP4 videos.
    pub fn video_mp4() -> Mime {
        "video/mp4".parse().unwrap()
    }
}

/// Upload progress info.
#[derive(Debug, Clone, PartialEq)]
pub enum ProgressInfo {
    /// Video is pending for processing. Contains number of seconds after which to check.
    Pending(u64),
    /// Video is beeing processed. Contains number of seconds after which to check.
    InProgress(u64),
    /// Video's processing failed. Contains reason.
    Failed(error::MediaError),
    /// Video's processing is finished. RawMedia can be used in other API calls.
    Success,
}

#[derive(Debug, Deserialize)]
enum RawProgressInfoTag {
    #[serde(rename = "pending")]
    Pending,
    #[serde(rename = "in_progress")]
    InProgress,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "succeeded")]
    Success,
}

#[derive(Debug, Deserialize)]
struct RawProgressInfo {
    state: RawProgressInfoTag,
    progress_percent: Option<f64>,
    check_after_secs: Option<u64>,
    error: Option<error::MediaError>,
}

impl<'de> Deserialize<'de> for ProgressInfo {
    fn deserialize<D>(deser: D) -> Result<ProgressInfo, D::Error>
    where
        D: Deserializer<'de>,
    {
        use self::RawProgressInfoTag::*;
        let raw = RawProgressInfo::deserialize(deser)?;
        let check_after = raw
            .check_after_secs
            .ok_or_else(|| D::Error::custom("Missing field: check_after_secs"));
        Ok(match raw.state {
            Pending => ProgressInfo::Pending(check_after?),
            InProgress => ProgressInfo::InProgress(check_after?),
            Success => ProgressInfo::Success,
            Failed => {
                let err = raw
                    .error
                    .ok_or_else(|| D::Error::custom("Missing field: error"))?;
                ProgressInfo::Failed(err)
            }
        })
    }
}

///Represents media file that is uploaded on twitter.
#[derive(Debug, Deserialize)]
struct RawMedia {
    /// ID that can be used in API calls (e.g. attach to tweet).
    #[serde(rename = "media_id_string")]
    id: String,
    /// Number of second the media can be used in other API calls.
    //We can miss this field on failed upload in which case 0 is pretty reasonable value.
    #[serde(default)]
    #[serde(rename = "expires_after_secs")]
    expires_after: u64,
    #[serde(rename = "processing_info")]
    progress: Option<ProgressInfo>,
}

#[derive(Debug, Clone, derive_more::From)]
/// An opaque type representing a media id.
pub struct MediaId(pub(crate) String);

/// A handle representing uploaded media.
#[derive(Debug, Clone)]
pub struct MediaHandle {
    /// ID that can be used in API calls (e.g. to attach media to tweet).
    pub id: MediaId,
    /// Number of second the media can be used in other API calls.
    pub expires_at: Instant,
    /// Progress information. If present determines whether RawMedia can be used.
    pub progress: Option<ProgressInfo>,
}

impl From<RawMedia> for MediaHandle {
    fn from(raw: RawMedia) -> Self {
        Self {
            id: raw.id.into(),
            // this conversion only makes sense if we create it immediately
            // after receiving from the server!
            expires_at: Instant::now() + Duration::from_secs(raw.expires_after),
            progress: raw.progress,
        }
    }
}

impl MediaHandle {
    /// Media uploads expire after a certain amount of time
    /// This method returns true if the upload is still valid
    /// and can therefore e.g. be attached to a tweet
    pub fn is_valid(&self) -> bool {
        Instant::now() < self.expires_at
    }
}

/// Represents the kind of media that Twitter will accept.
/// `.to_string()` will return a string suitable for use in API calls
#[derive(Debug, Copy, Clone, PartialEq, Eq, derive_more::Display)]
enum MediaCategory {
    /// Static image. Four can be attached to a single tweet.
    #[display(fmt = "tweet_image")]
    Image,
    /// Animated GIF.
    #[display(fmt = "tweet_gif")]
    Gif,
    /// Video.
    #[display(fmt = "tweet_video")]
    Video,
}

impl From<&mime::Mime> for MediaCategory {
    fn from(mime: &mime::Mime) -> Self {
        if mime == &media_types::image_gif() {
            MediaCategory::Gif
        } else if mime == &media_types::video_mp4() {
            MediaCategory::Video
        } else {
            // fallthrough
            MediaCategory::Image
        }
    }
}

impl MediaCategory {
    fn dm_category(&self) -> &'static str {
        match self {
            MediaCategory::Image => "dm_image",
            MediaCategory::Gif => "dm_gif",
            MediaCategory::Video => "dm_video",
        }
    }
}

/// Upload media to the server.
///
/// The upload proceeds in 1MB chunks until completed. After completion,
/// be sure to check the status of the uploaded media with [`get_status`].
/// Twitter often needs time to post-process media before it can be attached
/// to a tweet.
pub async fn upload_media(
    data: &[u8],
    media_type: &mime::Mime,
    token: &auth::Token,
) -> error::Result<MediaHandle> {
    let media_category = MediaCategory::from(media_type);
    let params = ParamList::new()
        .add_param("command", "INIT")
        .add_param("total_bytes", data.len().to_string())
        .add_param("media_type", media_type.to_string())
        .add_param("media_category", media_category.to_string());
    let req = post(links::media::UPLOAD, &token, Some(&params));

    let media = request_with_json_response::<RawMedia>(req).await?.response;

    finish_upload(media, data, token).await
}

/// Upload media to the server, for use in a Direct Message.
///
/// This function works the same as [`upload_media`], but uses a separate set of `media_category`
/// values to allow the resulting media to be attached to a Direct Message.
///
/// Because of the private nature of DMs, a separate flag is used to allow for media to be attached
/// to multiple messages. If the `shared` argument is set to `true`, then the resulting `MediaId`
/// can be used in multiple messages, but the resulting URL for the upload can be accessed by
/// anyone with the URL, rather then being private to the message. Twitter states that you must
/// provide the user with clear notice that the media can be viewed by anyone with the URL, and get
/// their express permission to set `shared`. Also note that even if you set `shared` to `true`,
/// the resulting media can only be attached to messages from the same user. The default (and
/// recommended) value for `shared` is `false`.
///
/// The upload proceeds in 1MB chunks until completed. After completion, be sure to check the
/// status of the uploaded media with [`get_status`]. Twitter often needs time to post-process
/// media before it can be attached to a message.
pub async fn upload_media_for_dm(
    data: &[u8],
    media_type: &mime::Mime,
    shared: bool,
    token: &auth::Token,
) -> error::Result<MediaHandle> {
    let media_category = MediaCategory::from(media_type);
    let params = ParamList::new()
        .add_param("command", "INIT")
        .add_param("total_bytes", data.len().to_string())
        .add_param("media_type", media_type.to_string())
        .add_param("media_category", media_category.dm_category())
        .add_param("shared", shared.to_string());
    let req = post(links::media::UPLOAD, &token, Some(&params));

    let media = request_with_json_response::<RawMedia>(req).await?.response;

    finish_upload(media, data, token).await
}

async fn finish_upload(
    media: RawMedia,
    data: &[u8],
    token: &auth::Token,
) -> error::Result<MediaHandle> {
    // divide into 1MB chunks
    for (ix, chunk) in data.chunks(1024 * 1024).enumerate() {
        let params = ParamList::new()
            .add_param("command", "APPEND")
            .add_param("media_id", media.id.clone())
            .add_param("media_data", base64::encode(chunk))
            .add_param("segment_index", ix.to_string());
        let req = post(links::media::UPLOAD, token, Some(&params));
        // This request has no response (upon success)
        raw_request(req).await?;
    }

    let params = ParamList::new()
        .add_param("command", "FINALIZE")
        .add_param("media_id", media.id.clone());
    let req = post(links::media::UPLOAD, token, Some(&params));
    Ok(request_with_json_response::<RawMedia>(req)
        .await?
        .response
        .into())
}

/// Check the status of uploaded media
pub async fn get_status(media_id: MediaId, token: &auth::Token) -> error::Result<MediaHandle> {
    let params = ParamList::new()
        .add_param("command", "STATUS")
        .add_param("media_id", media_id.0);
    let req = get(links::media::UPLOAD, token, Some(&params));
    Ok(request_with_json_response::<RawMedia>(req)
        .await?
        .response
        .into())
}

/// Set metadata for a media upload. At the moment the only attribute that may
/// be set is `alt_text`.
pub async fn set_metadata(
    media_id: &MediaId,
    alt_text: &str,
    token: &auth::Token,
) -> error::Result<()> {
    let payload = serde_json::json!({
        "media_id": media_id.0,
        "alt_text": {
            "text": alt_text
        }
    });
    let req = post_json(links::media::METADATA, &token, payload);
    raw_request(req).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::RawMedia;
    use crate::common::tests::load_file;

    fn load_media(path: &str) -> RawMedia {
        let content = load_file(path);
        ::serde_json::from_str::<RawMedia>(&content).unwrap()
    }

    #[test]
    fn parse_media() {
        let media = load_media("sample_payloads/media.json");

        assert_eq!(media.id, "710511363345354753");
        assert_eq!(media.expires_after, 86400);
    }

    #[test]
    fn parse_media_pending() {
        let media = load_media("sample_payloads/media_pending.json");

        assert_eq!(media.id, "13");
        assert_eq!(media.expires_after, 86400);
        assert!(media.progress.is_some());

        match media.progress {
            Some(super::ProgressInfo::Pending(5)) => (),
            other => assert!(false, format!("Unexpected value of progress={:?}", other)),
        }
    }

    #[test]
    fn parse_media_in_progress() {
        let media = load_media("sample_payloads/media_in_progress.json");

        assert_eq!(media.id, "13");
        assert_eq!(media.expires_after, 3595);
        assert!(media.progress.is_some());

        match media.progress {
            Some(super::ProgressInfo::InProgress(10)) => (),
            other => assert!(false, format!("Unexpected value of progress={:?}", other)),
        }
    }

    #[test]
    fn parse_media_fail() {
        let media = load_media("sample_payloads/media_fail.json");

        assert_eq!(media.id, "710511363345354753");
        assert_eq!(media.expires_after, 0);
        assert!(media.progress.is_some());

        match media.progress {
            Some(super::ProgressInfo::Failed(error)) => assert_eq!(
                error,
                crate::error::MediaError {
                    code: 1,
                    name: "InvalidMedia".to_string(),
                    message: "Unsupported video format".to_string(),
                }
            ),
            other => assert!(false, format!("Unexpected value of progress={:?}", other)),
        }
    }
}
