// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//!Media module
//!
//!Provides functionality to upload images, GIFs and videos using Twitter Media API

use std::borrow::Cow;
use std::collections::{HashMap, BTreeMap};
use std::error::Error as StdError;
use std::fmt;
use std::time::{Instant, Duration};

use futures::{Future, Async, Poll};
use rustc_serialize::{json, base64};
use rustc_serialize::base64::{ToBase64};
use tokio_core::reactor::Timeout;

use common::*;
use error;
use error::Error::InvalidResponse;
use links;
use auth;

use mime;

/// A collection of convenience functions that return media types accepted by Twitter.
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

    /// GIF images, both animated and static.
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
    Success
}

impl FromJson for ProgressInfo {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        field_present!(input, state);
        let state: String = try!(field(input, "state"));

        match state.as_ref() {
            "pending" => {
                field_present!(input, check_after_secs);
                Ok(ProgressInfo::Pending(try!(field(input, "check_after_secs"))))
            },
            "in_progress" => {
                field_present!(input, check_after_secs);
                Ok(ProgressInfo::InProgress(try!(field(input, "check_after_secs"))))
            },
            "failed" => {
                field_present!(input, error);
                Ok(ProgressInfo::Failed(try!(field(input, "error"))))
            },
            "succeeded" => Ok(ProgressInfo::Success),
            state => Err(InvalidResponse("Unexpected progress info state", Some(state.to_string())))
        }
    }
}

/// A media ID returned by twitter upon successful media upload.
#[derive(Copy, Clone, Debug)]
pub struct MediaHandle {
    /// The numeric ID that can be used to reference the media.
    pub media_id: u64,
    /// The time after which the media ID will be rendered unusable from the API. You can use
    /// `media_id` to attach the media to a tweet while `Instant::now() < handle.valid_until`.
    pub valid_until: Instant,
}

///Represents media file that is uploaded on twitter.
struct RawMedia {
    ///ID that can be used in API calls (e.g. attach to tweet).
    pub id: u64,
    ///Number of second the media can be used in other API calls.
    pub expires_after: u64,
    ///Progress information. If present determines whether RawMedia can be used.
    pub progress: Option<ProgressInfo>
}

impl RawMedia {
    fn into_handle(self) -> MediaHandle {
        MediaHandle {
            media_id: self.id,
            valid_until: Instant::now() + Duration::from_secs(self.expires_after),
        }
    }
}

impl FromJson for RawMedia {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse("Tweet received json that wasn't an object", Some(input.to_string())));
        }

        field_present!(input, media_id);

        Ok(RawMedia {
            id: try!(field(input, "media_id")),
            //We can miss this field on failed upload in which case 0 is pretty reasonable value.
            expires_after: field(input, "expires_after_secs").unwrap_or(0),
            progress: try!(field(input, "processing_info"))
        })
    }
}

/// Represents the kinda of media that Twitter will accept.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum MediaCategory {
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

/// A builder struct that allows you to build up parameters to a media upload before initiating it.
pub struct UploadBuilder<'a> {
    data: Cow<'a, [u8]>,
    media_type: mime::Mime,
    chunk_size: Option<usize>,
    category: Option<MediaCategory>,
    alt_text: Option<Cow<'a, str>>,
}

impl<'a> UploadBuilder<'a> {
    /// Begins setting up a media upload call, with the given data and media type.
    ///
    /// For convenience functions to get known `media_type`s that Twitter will accept, see the
    /// [`media_types`] module.
    ///
    /// [`media_types`]: media_types/index.html
    pub fn new<V: Into<Cow<'a, [u8]>>>(data: V, media_type: mime::Mime) -> UploadBuilder<'a> {
        UploadBuilder {
            data: data.into(),
            media_type,
            chunk_size: None,
            category: None,
            alt_text: None,
        }
    }

    /// Sets how many bytes to upload in one network call. By default this is set to 512 KiB.
    pub fn chunk_size(self, chunk_size: usize) -> Self {
        UploadBuilder {
            chunk_size: Some(chunk_size),
            ..self
        }
    }

    /// Sets the `media_category` sent to Twitter. When unset, it behaves as if you sent
    /// `MediaCategory::Image`.
    pub fn category(self, category: MediaCategory) -> Self {
        UploadBuilder {
            category: Some(category),
            ..self
        }
    }

    /// Applies the given alt text to an image or GIF when the image finishes uploading.
    pub fn alt_text<S: Into<Cow<'a, str>>>(self, alt_text: S) -> Self {
        UploadBuilder {
            alt_text: Some(alt_text.into()),
            ..self
        }
    }

    /// Collects the built-up parameters and begins the chunked upload.
    pub fn call(self, token: &auth::Token, handle: &Handle) -> UploadFuture<'a> {
        UploadFuture {
            data: self.data,
            media_type: self.media_type,
            media_category: self.category,
            timeout: Instant::now(),
            token: token.clone(),
            handle: handle.clone(),
            chunk_size: self.chunk_size.unwrap_or(1024 * 512), // 512 KiB default
            alt_text: self.alt_text,
            status: UploadInner::PreInit,
        }
    }
}

/// A `Future` that represents an in-progress media upload.
#[must_use = "futures do nothing unless polled"]
pub struct UploadFuture<'a> {
    data: Cow<'a, [u8]>,
    media_type: mime::Mime,
    media_category: Option<MediaCategory>,
    timeout: Instant,
    token: auth::Token,
    handle: Handle,
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
    PostProcessing(u64, Timeout),
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

        if let Some(category) = self.media_category {
            add_param(&mut params, "media_category", category.to_string());
        }

        let req = auth::post(links::media::UPLOAD, &self.token, Some(&params));
        make_parsed_future(&self.handle, req)
    }

    fn append(&self, chunk_num: usize, media_id: u64) -> Option<FutureResponse<()>> {
        let mut chunk = self.get_chunk(chunk_num);
        if chunk.is_none() && chunk_num == 0 {
            chunk = Some(&[][..]);
        }

        if let Some(chunk) = chunk {
            let mut params = HashMap::new();

            let config = base64::Config {
                char_set: base64::CharacterSet::Standard,
                newline: base64::Newline::LF,
                pad: true,
                line_length: None,
            };

            add_param(&mut params, "command", "APPEND");
            add_param(&mut params, "media_id", media_id.to_string());
            add_param(&mut params, "media_data", chunk.to_base64(config));
            add_param(&mut params, "segment_index", chunk_num.to_string());

            let req = auth::post(links::media::UPLOAD, &self.token, Some(&params));

            fn parse_resp(full_resp: String, headers: &Headers) -> Result<Response<()>, error::Error> {
                if full_resp.is_empty() {
                    Ok(rate_headers(headers))
                } else {
                    Err(InvalidResponse("Expected empty response", Some(full_resp)))
                }
            }

            Some(make_future(&self.handle, req, parse_resp))
        } else {
            None
        }
    }

    fn finalize(&self, media_id: u64) -> FutureResponse<RawMedia> {
        let mut params = HashMap::new();

        add_param(&mut params, "command", "FINALIZE");
        add_param(&mut params, "media_id", media_id.to_string());

        let req = auth::post(links::media::UPLOAD, &self.token, Some(&params));
        make_parsed_future(&self.handle, req)
    }

    fn status(&self, media_id: u64) -> FutureResponse<RawMedia> {
        let mut params = HashMap::new();

        add_param(&mut params, "command", "STATUS");
        add_param(&mut params, "media_id", media_id.to_string());

        let req = auth::get(links::media::UPLOAD, &self.token, Some(&params));
        make_parsed_future(&self.handle, req)
    }

    fn metadata(&self, media_id: u64, alt_text: &str) -> FutureResponse<()> {
        use rustc_serialize::json::Json;

        let mut inner = BTreeMap::new();
        inner.insert("text".to_string(), Json::String(alt_text.to_string()));

        let mut outer = BTreeMap::new();
        outer.insert("media_id".to_string(), Json::String(media_id.to_string()));
        outer.insert("alt_text".to_string(), Json::Object(inner));

        let body = Json::Object(outer);

        let req = auth::post_json(links::media::METADATA, &self.token, &body);

        fn parse_resp(full_resp: String, headers: &Headers) -> Result<Response<()>, error::Error> {
            if full_resp.is_empty() {
                Ok(rate_headers(headers))
            } else {
                Err(InvalidResponse("Expected empty response", Some(full_resp)))
            }
        }

        make_future(&self.handle, req, parse_resp)
    }
}

impl<'a> Future for UploadFuture<'a> {
    type Item = MediaHandle;
    type Error = UploadError;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        use std::mem::replace;

        match replace(&mut self.status, UploadInner::Invalid) {
            UploadInner::PreInit => {
                self.status = UploadInner::WaitingForInit(self.init());
                self.poll()
            },
            UploadInner::WaitingForInit(mut init) => {
                match init.poll() {
                    Ok(Async::NotReady) => {
                        self.status = UploadInner::WaitingForInit(init);
                        Ok(Async::NotReady)
                    },
                    Ok(Async::Ready(media)) => {
                        self.timeout = Instant::now() + Duration::from_secs(media.expires_after);
                        let id = media.id;
                        //chunk zero is guaranteed to return *something*, even an empty slice
                        let loader = self.append(0, id).unwrap();
                        self.status = UploadInner::UploadingChunk(id, 0, loader);
                        self.poll()
                    },
                    Err(e) => {
                        self.status = UploadInner::PreInit;
                        Err(UploadError::initialize(e))
                    },
                }
            },
            UploadInner::UploadingChunk(id, chunk_idx, mut upload) => {
                match upload.poll() {
                    Ok(Async::NotReady) => {
                        self.status = UploadInner::UploadingChunk(id, chunk_idx, upload);
                        Ok(Async::NotReady)
                    },
                    Ok(Async::Ready(_)) => {
                        let chunk_idx = chunk_idx + 1;
                        if let Some(upload) = self.append(chunk_idx, id) {
                            self.status = UploadInner::UploadingChunk(id, chunk_idx, upload);
                        } else {
                            let loader = self.finalize(id);
                            self.status = UploadInner::Finalizing(id, loader);
                        }

                        self.poll()
                    },
                    Err(e) => {
                        self.status = UploadInner::FailedChunk(id, chunk_idx);
                        Err(UploadError::chunk(self.timeout, e))
                    },
                }
            },
            UploadInner::FailedChunk(id, chunk_idx) => {
                if Instant::now() >= self.timeout {
                    //we've timed out, restart the upload
                    self.status = UploadInner::PreInit;
                    self.poll()
                } else if let Some(upload) = self.append(chunk_idx, id) {
                    self.status = UploadInner::UploadingChunk(id, chunk_idx, upload);
                    self.poll()
                } else {
                    //this... should never happen? the FailedChunk status means that this specific
                    //id/index should have yielded a chunk before. However, instead of panicking,
                    //i'll just invalidate the future
                    Err(UploadError::complete())
                }
            },
            UploadInner::Finalizing(id, mut finalize) => {
                match finalize.poll() {
                    Ok(Async::NotReady) => {
                        self.status = UploadInner::Finalizing(id, finalize);
                        Ok(Async::NotReady)
                    },
                    Ok(Async::Ready(media)) => {
                        if media.progress.is_none() || media.progress == Some(ProgressInfo::Success) {
                            let media = media.response.into_handle();
                            self.timeout = media.valid_until;
                            let loader = self.alt_text.as_ref().map(|txt| self.metadata(id, txt));
                            if let Some(loader) = loader {
                                self.status = UploadInner::Metadata(media, loader);
                                return self.poll();
                            } else {
                                return Ok(Async::Ready(media));
                            }
                        }

                        match media.response.progress {
                            Some(ProgressInfo::Pending(time)) |
                                Some(ProgressInfo::InProgress(time)) =>
                            {
                                self.timeout = Instant::now() + Duration::from_secs(media.expires_after);
                                let timer = match Timeout::new(Duration::from_secs(time), &self.handle) {
                                    Ok(timer) => timer,
                                    //this error will occur if the Core has been dropped
                                    Err(e) => return Err(UploadError::finalize(self.timeout, e.into())),
                                };
                                self.status = UploadInner::PostProcessing(media.id, timer);
                                self.poll()
                            },
                            Some(ProgressInfo::Failed(err)) =>
                                Err(UploadError::finalize(self.timeout,
                                                          error::Error::MediaError(err))),
                            None | Some(ProgressInfo::Success) => unreachable!(),
                        }
                    },
                    Err(e) => {
                        self.status = UploadInner::FailedFinalize(id);
                        Err(UploadError::finalize(self.timeout, e))
                    },
                }
            },
            UploadInner::FailedFinalize(id) => {
                if Instant::now() >= self.timeout {
                    //we've timed out, restart the upload
                    self.status = UploadInner::PreInit;
                } else {
                    let finalize = self.finalize(id);
                    self.status = UploadInner::Finalizing(id, finalize);
                }
                self.poll()
            },
            UploadInner::PostProcessing(id, mut timer) => {
                match timer.poll() {
                    Ok(Async::NotReady) => {
                        self.status = UploadInner::PostProcessing(id, timer);
                        Ok(Async::NotReady)
                    },
                    Ok(Async::Ready(())) => {
                        let loader = self.status(id);
                        self.status = UploadInner::Finalizing(id, loader);
                        self.poll()
                    },
                    //tokio's Timeout will literally never return an error, so don't bother
                    //rerouting the state here
                    Err(e) => Err(UploadError::finalize(self.timeout, e.into())),
                }
            },
            UploadInner::Metadata(media, mut loader) => {
                match loader.poll() {
                    Ok(Async::NotReady) => {
                        self.status = UploadInner::Metadata(media, loader);
                        Ok(Async::NotReady)
                    },
                    Ok(Async::Ready(_)) => {
                        Ok(Async::Ready(media))
                    },
                    Err(e) => {
                        self.status = UploadInner::FailedMetadata(media);
                        Err(UploadError::metadata(self.timeout, e))
                    },
                }
            },
            UploadInner::FailedMetadata(media) => {
                if Instant::now() >= self.timeout {
                    //we've timed out, restart the upload
                    self.status = UploadInner::PreInit;
                } else if let Some(ref alt_text) = self.alt_text {
                    let loader = self.metadata(media.media_id, alt_text);
                    self.status = UploadInner::Metadata(media, loader);
                } else {
                    //what... are we even doing here??? if we uploaded metadata then we should
                    //have had alt text to begin with
                    return Ok(Async::Ready(media));
                }

                self.poll()
            },
            UploadInner::Invalid => Err(UploadError::complete()),
        }
    }
}

/// An error wrapper for `UploadFuture`, noting what stage of the upload an error occurred at.
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
    /// Note that if `state` is `Initialize`, this field doesn't apply, and is set to a dummy value
    /// (specifically `Instant::now()`).
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

/// Represents the status of an `UploadFuture` when it encountered an error.
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
        write!(f, "Upload error during {:?}: \"{}\"", self.state, self.error)
    }
}

impl StdError for UploadError {
    fn description(&self) -> &str {
        "error occurred while uploading media"
    }

    fn cause(&self) -> Option<&StdError> {
        Some(&self.error)
    }
}

#[cfg(test)]
mod tests {
    use common::FromJson;

    use super::RawMedia;

    use std::fs::File;
    use std::io::Read;

    fn load_media(path: &str) -> RawMedia {
        let mut file = File::open(path).unwrap();
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();
        RawMedia::from_str(&content).unwrap()
    }

    #[test]
    fn parse_media() {
        let media = load_media("src/media/media.json");

        assert_eq!(media.id, 710511363345354753);
        assert_eq!(media.expires_after, 86400);
    }

    #[test]
    fn parse_media_pending() {
        let media = load_media("src/media/media_pending.json");

        assert_eq!(media.id, 13);
        assert_eq!(media.expires_after, 86400);
        assert!(media.progress.is_some());

        match media.progress {
            Some(super::ProgressInfo::Pending(5)) => (),
            other => assert!(false, format!("Unexpected value of progress={:?}", other))
        }
    }

    #[test]
    fn parse_media_in_progress() {
        let media = load_media("src/media/media_in_progress.json");

        assert_eq!(media.id, 13);
        assert_eq!(media.expires_after, 3595);
        assert!(media.progress.is_some());

        match media.progress {
            Some(super::ProgressInfo::InProgress(10)) => (),
            other => assert!(false, format!("Unexpected value of progress={:?}", other))
        }
    }

    #[test]
    fn parse_media_fail() {
        let media = load_media("src/media/media_fail.json");

        assert_eq!(media.id, 710511363345354753);
        assert_eq!(media.expires_after, 0);
        assert!(media.progress.is_some());

        match media.progress {
            Some(super::ProgressInfo::Failed(error)) =>
                assert_eq!(error, ::error::MediaError {
                    code: 1,
                    name: "InvalidMedia".to_string(),
                    message: "Unsupported video format".to_string(),
                }),
            other => assert!(false, format!("Unexpected value of progress={:?}", other))
        }
    }
}
