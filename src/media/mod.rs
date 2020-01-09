// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Functionality to upload images, GIFs, and videos that can be attached to tweets.
//!
//! Tweet media is uploaded separately from the act of posting the tweet itself. In order to attach
//! an image to a new tweet, you need to upload it first, then take the Media ID that Twitter
//! generates and reference that when posting the tweet. The way this works in egg-mode is to
//! create an [`UploadBuilder`] and turn that into an [`UploadFuture`], which manages the upload
//! process.
//!
//! [`UploadBuilder`]: struct.UploadBuilder.html
//! [`UploadFuture`]: struct.UploadFuture.html
//!
//! For example, here's a basic use of `UploadFuture` to upload an image, then attach it to a
//! tweet:
//!
//! ```rust,no_run
//! # use egg_mode::Token;
//! # #[tokio::main]
//! # async fn main() {
//! # let token: Token = unimplemented!();
//! use egg_mode::media::{UploadBuilder, media_types};
//! use egg_mode::tweet::DraftTweet;
//!
//! let image = vec![]; //pretend we loaded an image file into this
//! let builder = UploadBuilder::new(image, media_types::image_png());
//! let media_handle = builder.call(&token).await.unwrap();
//!
//! let draft = DraftTweet::new("Hey, check out this cute cat!")
//!                        .media_ids(&[media_handle.id]);
//! let tweet = draft.send(&token).await.unwrap();
//! # }
//! ```
//!
//! For more information, see the [`UploadBuilder`] documentation.

use std::borrow::Cow;
use std::collections::HashMap;
use std::error::Error as StdError;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::{Duration, Instant};

use base64;
use serde::de::Error;
use serde::{Deserialize, Deserializer};
use tokio::time::{self, Delay};

use crate::common::*;
use crate::error::Error::InvalidResponse;
use crate::{auth, error, links};

use mime;

/// A collection of convenience functions that return media types accepted by Twitter.
///
/// These are convenience types that can be handed to [`UploadBuilder::new`] to set the right media
/// type of a piece of media. The functions in the module correspond to media types that Twitter is
/// known to accept.
///
/// Note that using `image_gif` and `video_mp4` will automatically set the upload's
/// `media_category` to `tweet_gif` and `tweet_video` respectively, allowing larger file sizes and
/// extra processing time.
///
/// [`UploadBuilder::new`]: ../struct.UploadBuilder.html#method.new
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

///RawMedia's upload progressing info.
#[derive(Debug, PartialEq)]
enum ProgressInfo {
    ///Video is pending for processing. Contains number of seconds after which to check.
    Pending(u64),
    ///Video is beeing processed. Contains number of seconds after which to check.
    InProgress(u64),
    ///Video's processing failed. Contains reason.
    Failed(error::MediaError),
    ///Video's processing is finished. RawMedia can be used in other API calls.
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

/// A media handle returned by twitter upon successful upload.
///
/// To get one of these, start with [`UploadBuilder`]. To use the `id` inside, see
/// [`DraftTweet::media_ids`].
///
/// [`UploadBuilder`]: struct.UploadBuilder.html
/// [`DraftTweet::media_ids`]: ../tweet/struct.DraftTweet.html#method.media_ids
#[derive(Copy, Clone, Debug)]
pub struct MediaHandle {
    /// The ID that can be used to reference the media.
    pub id: u64,
    /// The time after which the media will be rendered unusable in the twitter API.
    pub valid_until: Instant,
}

impl MediaHandle {
    #[inline]
    /// Returns whether media is still valid to be used in API calls.
    ///
    /// Under hood it is `Instant::now() < handle.valid_until`.
    pub fn is_valid(&self) -> bool {
        Instant::now() < self.valid_until
    }
}

///Represents media file that is uploaded on twitter.
#[derive(Deserialize)]
struct RawMedia {
    ///ID that can be used in API calls (e.g. attach to tweet).
    #[serde(rename = "media_id")]
    pub id: u64,
    ///Number of second the media can be used in other API calls.
    //We can miss this field on failed upload in which case 0 is pretty reasonable value.
    #[serde(default)]
    #[serde(rename = "expires_after_secs")]
    pub expires_after: u64,
    ///Progress information. If present determines whether RawMedia can be used.
    #[serde(rename = "processing_info")]
    pub progress: Option<ProgressInfo>,
}

impl RawMedia {
    fn into_handle(self) -> MediaHandle {
        MediaHandle {
            id: self.id,
            valid_until: Instant::now() + Duration::from_secs(self.expires_after),
        }
    }
}

/// Represents the kinda of media that Twitter will accept.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum MediaCategory {
    /// Static image. Four can be attached to a single tweet.
    Image,
    /// Animated GIF.
    Gif,
    /// Video.
    Video,
}

/// `Display` impl for `MediaCategory` so that `.to_string()` will return a string suitable for use
/// in an API call. This will turn the enum into `"tweet_image"`, `"tweet_gif"`, and
/// `"tweet_video"`.
impl ::std::fmt::Display for MediaCategory {
    fn fmt(&self, fmt: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        match *self {
            MediaCategory::Image => write!(fmt, "tweet_image"),
            MediaCategory::Gif => write!(fmt, "tweet_gif"),
            MediaCategory::Video => write!(fmt, "tweet_video"),
        }
    }
}

/// Represents a media upload before it is sent.
///
/// `UploadBuilder` is the entry point to uploading media to Twitter.
///  It allows you to configure an upload and set the proper metadata.
///
/// [`UploadFuture`]: struct.UploadFuture.html
///
/// To begin setting up an upload, call `new` with your data and its media type. (Convenience
/// functions to create `Mime` instances for types Twitter is known to accept are available in the
/// [`media_types`] module.) With that, you can configure the upload, and finally start the
/// process using the `call` method. See a basic example of using `UploadBuilder` to upload an image and attach it
/// to a Tweet in [the module documentation].
///
/// [`media_types`]: media_types/index.html
/// [the module documentation]: index.html
///
/// To see more precise specifications for what media formats Twitter supports (resolution, file
/// size, etc), see [their API documentation][media-best-practices]. Note that `UploadBuilder`
/// automatically sets the underlying `media_category` to `tweet_gif` or `tweet_video` for
/// `media_type`s of `"image/gif"` and `"video/mp4"` respectively. (Note that these are returned by
/// [`media_types::image_gif`] and [`media_types::video_mp4`] as a convenience.)
///
/// [media-best-practices]: https://developer.twitter.com/en/docs/media/upload-media/uploading-media/media-best-practices
/// [`media_types::image_gif`]: media_types/fn.image_gif.html
/// [`media_types::video_mp4`]: media_types/fn.video_mp4.html
///
/// The lifetime parameter on `UploadBuilder` and [`UploadFuture`] is based on the data you hand to
/// `new` and `alt_text`. Because they use `std::borrow::Cow` internally, if you hand them owned
/// data (`Vec` or `String`), the resulting [`UploadFuture`] will have lifetime `'static`.
pub struct UploadBuilder<'a> {
    data: Cow<'a, [u8]>,
    media_type: mime::Mime,
    chunk_size: Option<usize>,
    category: MediaCategory,
    alt_text: Option<Cow<'a, str>>,
}

impl<'a> UploadBuilder<'a> {
    /// Creates a new instance of `UploadBuilder` with the given data and media type.
    ///
    /// For convenience functions to get known `media_type`s that Twitter will accept, see the
    /// [`media_types`] module.
    ///
    /// [`media_types`]: media_types/index.html
    pub fn new<V: Into<Cow<'a, [u8]>>>(data: V, media_type: mime::Mime) -> UploadBuilder<'a> {
        let category = if media_type == media_types::image_gif() {
            MediaCategory::Gif
        } else if media_type == media_types::video_mp4() {
            MediaCategory::Video
        } else {
            MediaCategory::Image
        };
        UploadBuilder {
            data: data.into(),
            media_type,
            chunk_size: None,
            category,
            alt_text: None,
        }
    }

    /// Sets how many bytes to upload in one network call. By default this is set to 512 KiB.
    ///
    /// `UploadFuture` uses Twitter's chunked media upload under-the-hood, and this allows you to
    /// set the size of each chunk.
    /// With a smaller chunk size, Twitter can "save" the data more often.
    /// However, there's also network overhead, since each chunk needs a separate HTTP request.
    /// Larger chunk sizes are better for stable network conditions, where you can reasonably expect a large upload to succeed.
    /// Note that once the `UploadFuture` is created, the chunk size cannot be changed.
    pub fn chunk_size(self, chunk_size: usize) -> Self {
        UploadBuilder {
            chunk_size: Some(chunk_size),
            ..self
        }
    }

    /// Applies the given alt text to the media when the upload is finished.
    pub fn alt_text<S: Into<Cow<'a, str>>>(self, alt_text: S) -> Self {
        UploadBuilder {
            alt_text: Some(alt_text.into()),
            ..self
        }
    }

    /// Starts the upload process and returns a `Future` that represents it.
    pub fn call(self, token: &auth::Token) -> UploadFuture<'a> {
        UploadFuture {
            data: self.data,
            media_type: self.media_type,
            media_category: self.category,
            timeout: Instant::now(),
            token: token.clone(),
            chunk_size: self.chunk_size.unwrap_or(1024 * 512), // 512 KiB default
            alt_text: self.alt_text,
            status: UploadInner::PreInit,
        }
    }
}

/// A `Future` that represents an in-progress media upload.
///
/// This struct is obtained from an [`UploadBuilder`]. See those docs for specifics on creating
/// one, and [the module docs] for more information on how to upload media in general.
///
/// [`UploadBuilder`]: struct.UploadBuilder.html
/// [the module docs]: index.html
///
/// # Errors
///
/// Because `UploadFuture` represents a potentially long-running upload, it's set up so that if it
/// fails at any point in the process, it will retry its last action upon its next `poll`. This
/// also includes keeping its place in terms of how many chunks it's uploaded so far.
///
/// There's a complicating factor for this, though: Twitter only allows an upload session to be
/// active for a limited time. `UploadFuture` keeps track of when the session expires, and
/// restarts the upload if it's `poll`ed from an error state when the time has elapsed. (Note that
/// timeout is checked only in case of errors. If the last action was a successful one,
/// it will send off the next action to Twitter, likely receiving an error for
/// that, after which it will restart the upload.) This timeout is reflected in the [`UploadError`]
/// it returns in any error case.
///
/// [`UploadError`]: struct.UploadError.html
///
/// To allow for better handling of individual errors and better retry logic, the [`UploadError`]
/// also includes a mention of which state the `UploadFuture` was in before encountering the error.
/// If the Future was attempting to upload an individual chunk, or finalizing the upload session,
/// and the Future encountered a network error, it should be safe to retry the Future if network
/// conditions improve before the timeout elapses. (The precise [`Error`] which was encountered is
/// also included in the returned `UploadError`.)
///
/// [`Error`]: ../error/enum.Error.html
///
/// (Proper mechanisms for actually integrating retry logic into your application is beyond the
/// scope of this library. There are dedicated libraries for retry logic, or you can use the
/// built-in `shared` function on all `Futures`s to get a cloneable handle to a `Future` so you can
/// keep a handle to send back into an executor. This is all just to say that an `UploadFuture` is
/// not invalidated when it returns an `Err` from `poll`.)
///
/// # Lifetimes
///
/// The lifetime parameter to `UploadFuture` is based on the data (and alt text) given to the
/// `UploadBuilder` that created it. If an owned `Vec` (and either no alt text or an owned
/// `String`) was given to the `UploadBuilder`, this future will have a `'static` lifetime.
#[must_use = "futures do nothing unless polled"]
pub struct UploadFuture<'a> {
    data: Cow<'a, [u8]>,
    media_type: mime::Mime,
    media_category: MediaCategory,
    timeout: Instant,
    token: auth::Token,
    chunk_size: usize,
    alt_text: Option<Cow<'a, str>>,
    status: UploadInner,
}

/// The current status of an `UploadFuture`.
enum UploadInner {
    /// The `UploadFuture` has yet to initialize the upload session.
    PreInit,
    /// The `UploadFuture` is waiting to initialize the media upload session.
    WaitingForInit(FutureResponse<RawMedia>),
    /// The `UploadFuture` is in the progress of uploading data.
    UploadingChunk(u64, usize, FutureResponse<()>),
    /// The `UploadFuture` failed to upload a chunk of data and is waiting to re-send it.
    FailedChunk(u64, usize),
    /// The `UploadFuture` is currently finalizing the media with Twitter.
    Finalizing(u64, FutureResponse<RawMedia>),
    /// The `UploadFuture` failed to finalize the upload session, and is waiting to retry.
    FailedFinalize(u64),
    /// The `UploadFuture` is waiting on Twitter to finish processing a video or gif.
    PostProcessing(u64, Delay),
    /// The `UploadFuture` is waiting on Twitter to apply metadata to the uploaded image.
    Metadata(MediaHandle, FutureResponse<()>),
    /// The `UploadFuture` failed to update metadata on the media.
    FailedMetadata(MediaHandle),
    /// The `UploadFuture` has completed, or has encountered an error.
    Invalid,
}

impl<'a> UploadFuture<'a> {
    fn get_chunk(&self, chunk_num: usize) -> Option<&[u8]> {
        let start = chunk_num * self.chunk_size;
        let end = (chunk_num + 1) * self.chunk_size;

        if start >= self.data.len() {
            None
        } else if end >= self.data.len() {
            Some(&self.data[start..])
        } else {
            Some(&self.data[start..end])
        }
    }

    fn init(&self) -> FutureResponse<RawMedia> {
        let mut params = HashMap::new();

        add_param(&mut params, "command", "INIT");
        add_param(&mut params, "total_bytes", self.data.len().to_string());
        add_param(&mut params, "media_type", self.media_type.to_string());
        add_param(
            &mut params,
            "media_category",
            self.media_category.to_string(),
        );

        let req = auth::post(links::media::UPLOAD, &self.token, Some(&params));
        make_parsed_future(req)
    }

    fn append(&self, chunk_num: usize, media_id: u64) -> Option<FutureResponse<()>> {
        let mut chunk = self.get_chunk(chunk_num);
        if chunk.is_none() && chunk_num == 0 {
            chunk = Some(&[][..]);
        }

        if let Some(chunk) = chunk {
            let mut params = HashMap::new();

            add_param(&mut params, "command", "APPEND");
            add_param(&mut params, "media_id", media_id.to_string());
            add_param(&mut params, "media_data", base64::encode(chunk));
            add_param(&mut params, "segment_index", chunk_num.to_string());

            let req = auth::post(links::media::UPLOAD, &self.token, Some(&params));

            fn parse_resp(
                full_resp: String,
                headers: &Headers,
            ) -> Result<Response<()>, error::Error> {
                if full_resp.is_empty() {
                    rate_headers(headers)
                } else {
                    Err(InvalidResponse("Expected empty response", Some(full_resp)))
                }
            }

            Some(make_future(req, parse_resp))
        } else {
            None
        }
    }

    fn finalize(&self, media_id: u64) -> FutureResponse<RawMedia> {
        let mut params = HashMap::new();

        add_param(&mut params, "command", "FINALIZE");
        add_param(&mut params, "media_id", media_id.to_string());

        let req = auth::post(links::media::UPLOAD, &self.token, Some(&params));
        make_parsed_future(req)
    }

    fn status(&self, media_id: u64) -> FutureResponse<RawMedia> {
        let mut params = HashMap::new();

        add_param(&mut params, "command", "STATUS");
        add_param(&mut params, "media_id", media_id.to_string());

        let req = auth::get(links::media::UPLOAD, &self.token, Some(&params));
        make_parsed_future(req)
    }

    fn metadata(&self, media_id: u64, alt_text: &str) -> FutureResponse<()> {
        use serde_json::map::Map;
        use serde_json::Value;

        let mut inner = Map::new();
        inner.insert("text".to_string(), Value::String(alt_text.to_string()));

        let mut outer = Map::new();
        outer.insert("media_id".to_string(), Value::String(media_id.to_string()));
        outer.insert("alt_text".to_string(), Value::Object(inner));

        let body = Value::Object(outer);

        let req = auth::post_json(links::media::METADATA, &self.token, &body);

        fn parse_resp(full_resp: String, headers: &Headers) -> Result<Response<()>, error::Error> {
            if full_resp.is_empty() {
                rate_headers(headers)
            } else {
                Err(InvalidResponse("Expected empty response", Some(full_resp)))
            }
        }

        make_future(req, parse_resp)
    }
}

impl<'a> Future for UploadFuture<'a> {
    type Output = Result<MediaHandle, UploadError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        use std::mem::replace;

        match replace(&mut self.status, UploadInner::Invalid) {
            UploadInner::PreInit => {
                self.status = UploadInner::WaitingForInit(self.init());
                self.poll(cx)
            }
            UploadInner::WaitingForInit(mut init) => {
                match Pin::new(&mut init).poll(cx) {
                    Poll::Pending => {
                        self.status = UploadInner::WaitingForInit(init);
                        Poll::Pending
                    }
                    Poll::Ready(Ok(media)) => {
                        self.timeout = Instant::now() + Duration::from_secs(media.expires_after);
                        let id = media.id;
                        //chunk zero is guaranteed to return *something*, even an empty slice
                        let loader = self.append(0, id).unwrap();
                        self.status = UploadInner::UploadingChunk(id, 0, loader);
                        self.poll(cx)
                    }
                    Poll::Ready(Err(e)) => {
                        self.status = UploadInner::PreInit;
                        Poll::Ready(Err(UploadError::initialize(e)))
                    }
                }
            }
            UploadInner::UploadingChunk(id, chunk_idx, mut upload) => {
                match Pin::new(&mut upload).poll(cx) {
                    Poll::Pending => {
                        self.status = UploadInner::UploadingChunk(id, chunk_idx, upload);
                        Poll::Pending
                    }
                    Poll::Ready(Ok(_)) => {
                        let chunk_idx = chunk_idx + 1;
                        if let Some(upload) = self.append(chunk_idx, id) {
                            self.status = UploadInner::UploadingChunk(id, chunk_idx, upload);
                        } else {
                            let loader = self.finalize(id);
                            self.status = UploadInner::Finalizing(id, loader);
                        }

                        self.poll(cx)
                    }
                    Poll::Ready(Err(e)) => {
                        self.status = UploadInner::FailedChunk(id, chunk_idx);
                        Poll::Ready(Err(UploadError::chunk(self.timeout, e)))
                    }
                }
            }
            UploadInner::FailedChunk(id, chunk_idx) => {
                if Instant::now() >= self.timeout {
                    //we've timed out, restart the upload
                    self.status = UploadInner::PreInit;
                    self.poll(cx)
                } else if let Some(upload) = self.append(chunk_idx, id) {
                    self.status = UploadInner::UploadingChunk(id, chunk_idx, upload);
                    self.poll(cx)
                } else {
                    //this... should never happen? the FailedChunk status means that this specific
                    //id/index should have yielded a chunk before.
                    unreachable!()
                }
            }
            UploadInner::Finalizing(id, mut finalize) => {
                match Pin::new(&mut finalize).poll(cx) {
                    Poll::Pending => {
                        self.status = UploadInner::Finalizing(id, finalize);
                        Poll::Pending
                    }
                    Poll::Ready(Ok(media)) => {
                        if media.progress.is_none() || media.progress == Some(ProgressInfo::Success)
                        {
                            let media = media.response.into_handle();
                            self.timeout = media.valid_until;
                            let loader = self.alt_text.as_ref().map(|txt| self.metadata(id, txt));
                            if let Some(loader) = loader {
                                self.status = UploadInner::Metadata(media, loader);
                                return self.poll(cx);
                            } else {
                                return Poll::Ready(Ok(media));
                            }
                        }

                        match media.response.progress {
                            Some(ProgressInfo::Pending(time))
                            | Some(ProgressInfo::InProgress(time)) => {
                                self.timeout =
                                    Instant::now() + Duration::from_secs(media.expires_after);
                                //TODO: oh hey we needed the handle for something - we need to use
                                //new-tokio to fix this
                                let delay = Duration::from_secs(time);
                                let timer = time::delay_for(delay);
                                self.status = UploadInner::PostProcessing(media.id, timer);
                                self.poll(cx)
                            }
                            Some(ProgressInfo::Failed(err)) => {
                                self.status = UploadInner::FailedFinalize(id);
                                Poll::Ready(Err(UploadError::finalize(
                                    self.timeout,
                                    error::Error::MediaError(err),
                                )))
                            }
                            None | Some(ProgressInfo::Success) => unreachable!(),
                        }
                    }
                    Poll::Ready(Err(e)) => {
                        self.status = UploadInner::FailedFinalize(id);
                        Poll::Ready(Err(UploadError::finalize(self.timeout, e)))
                    }
                }
            }
            UploadInner::FailedFinalize(id) => {
                if Instant::now() >= self.timeout {
                    //we've timed out, restart the upload
                    self.status = UploadInner::PreInit;
                } else {
                    let finalize = self.finalize(id);
                    self.status = UploadInner::Finalizing(id, finalize);
                }
                self.poll(cx)
            }
            UploadInner::PostProcessing(id, mut timer) => match Pin::new(&mut timer).poll(cx) {
                Poll::Pending => {
                    self.status = UploadInner::PostProcessing(id, timer);
                    Poll::Pending
                }
                Poll::Ready(()) => {
                    let loader = self.status(id);
                    self.status = UploadInner::Finalizing(id, loader);
                    self.poll(cx)
                }
            },
            UploadInner::Metadata(media, mut loader) => match Pin::new(&mut loader).poll(cx) {
                Poll::Pending => {
                    self.status = UploadInner::Metadata(media, loader);
                    Poll::Pending
                }
                Poll::Ready(Ok(_)) => Poll::Ready(Ok(media)),
                Poll::Ready(Err(e)) => {
                    self.status = UploadInner::FailedMetadata(media);
                    Poll::Ready(Err(UploadError::metadata(self.timeout, e)))
                }
            },
            UploadInner::FailedMetadata(media) => {
                if Instant::now() >= self.timeout {
                    //we've timed out, restart the upload
                    self.status = UploadInner::PreInit;
                } else if let Some(ref alt_text) = self.alt_text {
                    let loader = self.metadata(media.id, alt_text);
                    self.status = UploadInner::Metadata(media, loader);
                } else {
                    //if we uploaded metadata then we should have had alt text to begin with
                    unreachable!();
                }

                self.poll(cx)
            }
            UploadInner::Invalid => Poll::Ready(Err(UploadError::complete())),
        }
    }
}

/// A wrapper for `UploadFuture` errors, noting at which stage of the upload the error occurred at.
///
/// Since [`UploadFuture`] can retry its last action after an error, the error it returns includes
/// additional information to allow for smarter retry logic if necessary. See the [`UploadFuture`]
/// documentation for more details.
///
/// [`UploadFuture`]: struct.UploadFuture.html
#[derive(Debug)]
pub struct UploadError {
    /// The stage of upload that the error occurred at.
    pub state: UploadState,
    /// The time when the `UploadFuture` will no longer be valid.
    ///
    /// Since Twitter only allows upload sessions to be open for a limited time period,
    /// `UploadFuture` will automatically restart upload sessions if it detects that the timeout
    /// has elapsed after a previous error. Note that even completed upload sessions have a timeout
    /// for how long the ID can be used to attach the media, which is also reflected here if
    /// `state` is `Metadata`.
    ///
    /// Note that if `state` is `Initialize` or `Complete`, this field is invalid, and is set to a
    /// dummy value (specifically `Instant::now()`).
    pub timeout: Instant,
    /// The error that occurred in `UploadFuture`.
    pub error: error::Error,
}

impl UploadError {
    fn initialize(error: error::Error) -> UploadError {
        UploadError {
            state: UploadState::Initialize,
            timeout: Instant::now(),
            error: error,
        }
    }

    fn chunk(timeout: Instant, error: error::Error) -> UploadError {
        UploadError {
            state: UploadState::ChunkUpload,
            timeout: timeout,
            error: error,
        }
    }

    fn finalize(timeout: Instant, error: error::Error) -> UploadError {
        UploadError {
            state: UploadState::Finalize,
            timeout: timeout,
            error: error,
        }
    }

    fn metadata(timeout: Instant, error: error::Error) -> UploadError {
        UploadError {
            state: UploadState::Metadata,
            timeout: timeout,
            error: error,
        }
    }

    fn complete() -> UploadError {
        UploadError {
            state: UploadState::Complete,
            timeout: Instant::now(),
            error: error::Error::FutureAlreadyCompleted,
        }
    }
}

/// Represents the status of an `UploadFuture`.
///
/// This is a representation of the distinct phases of an [`UploadFuture`], given as part of an
/// [`UploadError`]. See the [`UploadFuture`] documentation for details.
///
/// [`UploadFuture`]: struct.UploadFuture.html
/// [`UploadError`]: struct.UploadError.html
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum UploadState {
    /// The `UploadFuture` was trying to initialize the upload session.
    Initialize,
    /// The `UploadFuture` was trying to upload a chunk of the media file.
    ChunkUpload,
    /// The `UploadFuture` was trying to finalize the upload session.
    Finalize,
    /// The `UploadFuture` was trying to apply alt-text metadata to the media after finalizing the
    /// upload session.
    Metadata,
    /// The `UploadFuture` was fully completed, or previously encountered an error that dropped it
    /// out of the upload process.
    Complete,
}

impl fmt::Display for UploadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Upload error during {:?}: \"{}\"",
            self.state, self.error
        )
    }
}

impl StdError for UploadError {
    fn description(&self) -> &str {
        "error occurred while uploading media"
    }

    fn cause(&self) -> Option<&dyn StdError> {
        Some(&self.error)
    }
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

        assert_eq!(media.id, 710511363345354753);
        assert_eq!(media.expires_after, 86400);
    }

    #[test]
    fn parse_media_pending() {
        let media = load_media("sample_payloads/media_pending.json");

        assert_eq!(media.id, 13);
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

        assert_eq!(media.id, 13);
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

        assert_eq!(media.id, 710511363345354753);
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
