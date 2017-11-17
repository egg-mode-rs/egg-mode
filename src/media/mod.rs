// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//!Media module
//!
//!Provides functionality to upload images, GIFs and videos using Twitter Media API

use std::borrow::Cow;
use std::collections::HashMap;

use futures::{Future, Async, Poll};
use rustc_serialize::{json, base64};
use rustc_serialize::base64::{ToBase64};

use common::*;
use error;
use error::Error::{InvalidResponse, MissingValue};
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

#[derive(Debug)]
///Media's upload progressing info.
pub enum ProgressInfo {
    ///Video is pending for processing. Contains number of seconds after which to check.
    Pending(u64),
    ///Video is beeing processed. Contains number of seconds after which to check.
    InProgress(u64),
    ///Video's processing failed. Contains reason.
    Failed(String),
    ///Video's processing is finished. Media can be used in other API calls.
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
                let error = try!(input.find("error").ok_or(MissingValue("error")));

                let name = try!(error.find("name").ok_or(MissingValue("name")));
                let message = try!(error.find("message").ok_or(MissingValue("message")));

                let name = try!(name.as_string().ok_or(InvalidResponse("Expected string in error's name", None)));
                let message = try!(message.as_string().ok_or(InvalidResponse("Expected string in error's message", None)));
                Ok(ProgressInfo::Failed(format!("{}: {}", name, message)))
            },
            "succeeded" => Ok(ProgressInfo::Success),
            state => Err(InvalidResponse("Unexpected progress info state", Some(state.to_string())))

        }
    }
}

///Represents media file that is uploaded on twitter.
pub struct Media {
    ///ID that can be used in API calls (e.g. attach to tweet).
    pub id: u64,
    ///Number of second the media can be used in other API calls.
    pub expires_after: u64,
    ///Progress information. If present determines whether Media can be used.
    pub progress: Option<ProgressInfo>
}

impl FromJson for Media {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse("Tweet received json that wasn't an object", Some(input.to_string())));
        }

        field_present!(input, media_id);

        Ok(Media {
            id: try!(field(input, "media_id")),
            //We can miss this field on failed upload in which case 0 is pretty reasonable value.
            expires_after: field(input, "expires_after_secs").unwrap_or(0),
            progress: try!(field(input, "processing_info"))
        })
    }
}

///Uploads image using old twitter API.
///
///The image should be raw binary with content of file.
pub fn upload_image(image: &[u8], token: &auth::Token, handle: &Handle) -> FutureResponse<Media> {
    let mut params = HashMap::new();

    let config = base64::Config {
        char_set: base64::CharacterSet::Standard,
        newline: base64::Newline::LF,
        pad: true,
        line_length: None,
    };

    add_param(&mut params, "media_data", image.to_base64(config));

    let req = auth::post(links::medias::UPLOAD, token, Some(&params));
    make_parsed_future(handle, req)
}

/// A `Future` that represents an in-progress media upload.
pub struct UploadFuture<'a> {
    data: Cow<'a, [u8]>,
    token: auth::Token,
    handle: Handle,
    chunk_size: usize,
    status: UploadInner,
}

/// The current status of an `UploadFuture`.
enum UploadInner {
    /// The `UploadFuture` is waiting to initialize the media upload session.
    WaitingForInit(FutureResponse<Media>),
    /// The `UploadFuture` is in the progress of uploading data.
    UploadingChunk(u64, usize, FutureResponse<()>),
    /// The `UploadFuture` is currently finalizing the media with Twitter.
    Finalizing(FutureResponse<Media>),
    /// The `UploadFuture` has completed, or has encountered an error.
    Invalid,
}

impl<'a> UploadFuture<'a> {
    /// Creates a new `UploadFuture` with the given data and media type, and with the default chunk
    /// size of 512 KiB.
    ///
    /// For convenience functions to get known `media_type`s that Twitter will accept, see the
    /// [`media_types`] module.
    ///
    /// [`media_types`]: media_types/index.html
    pub fn new<V: Into<Cow<'a, [u8]>>>(data: V,
                                       media_type: mime::Mime,
                                       token: &auth::Token,
                                       handle: &Handle)
        -> UploadFuture<'a>
    {
        let data = data.into();
        let loader = upload::init(data.len(), media_type, token, handle);
        UploadFuture {
            data,
            token: token.clone(),
            handle: handle.clone(),
            chunk_size: 1024 * 512, // 512 KiB
            status: UploadInner::WaitingForInit(loader),
        }
    }

    /// Creates a new `UploadFuture` with the given data, media type, and chunk size.
    ///
    /// For convenience functions to get known `media_type`s that Twitter will accept, see the
    /// [`media_types`] module.
    ///
    /// [`media_types`]: media_types/index.html
    pub fn with_chunk_size<V: Into<Cow<'a, [u8]>>>(data: V,
                                                   chunk_size: usize,
                                                   media_type: mime::Mime,
                                                   token: &auth::Token,
                                                   handle: &Handle)
        -> UploadFuture<'a>
    {
        let data = data.into();
        let loader = upload::init(data.len(), media_type, token, handle);
        UploadFuture {
            data,
            token: token.clone(),
            handle: handle.clone(),
            chunk_size,
            status: UploadInner::WaitingForInit(loader),
        }
    }

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

    fn append(&self, chunk_num: usize, media_id: u64) -> Option<FutureResponse<()>> {
        let mut chunk = self.get_chunk(chunk_num);
        if chunk.is_none() && chunk_num == 0 {
            chunk = Some(&[][..]);
        }

        if let Some(chunk) = chunk {
            Some(upload::append(media_id, chunk, chunk_num, &self.token, &self.handle))
        } else {
            None
        }
    }
}

impl<'a> Future for UploadFuture<'a> {
    type Item = Response<Media>;
    type Error = error::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        use std::mem::replace;

        match replace(&mut self.status, UploadInner::Invalid) {
            UploadInner::WaitingForInit(mut init) => {
                match init.poll() {
                    Ok(Async::NotReady) => {
                        self.status = UploadInner::WaitingForInit(init);
                        Ok(Async::NotReady)
                    },
                    Ok(Async::Ready(media)) => {
                        let id = media.id;
                        let loader = self.append(0, id).unwrap();
                        self.status = UploadInner::UploadingChunk(id, 0, loader);
                        self.poll()
                    },
                    Err(e) => Err(e),
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
                            let loader = upload::finalize(id, &self.token, &self.handle);
                            self.status = UploadInner::Finalizing(loader);
                        }

                        self.poll()
                    },
                    Err(e) => Err(e),
                }
            },
            UploadInner::Finalizing(mut finalize) => {
                match finalize.poll() {
                    Ok(Async::NotReady) => {
                        self.status = UploadInner::Finalizing(finalize);
                        Ok(Async::NotReady)
                    },
                    status => status,
                }
            },
            UploadInner::Invalid => Err(error::Error::FutureAlreadyCompleted),
        }
    }
}

///Content upload module using new chunked API.
///
///See [example](https://developer.twitter.com/en/docs/media/upload-media/uploading-media/chunked-media-upload)
pub mod upload {
    use super::*;

    ///Sends INIT message to twitter API in order to initiate upload.
    ///
    ///## Parameters:
    ///
    ///* total - The size of media file being uploaded(in bytes).
    ///* mime - File's mime type.
    pub fn init(total: usize, mime: mime::Mime, token: &auth::Token, handle: &Handle) -> FutureResponse<Media> {
        let mut params = HashMap::new();

        add_param(&mut params, "command", "INIT");
        add_param(&mut params, "total_bytes", total.to_string());
        add_param(&mut params, "media_type", mime.to_string());

        let req = auth::post(links::medias::UPLOAD, token, Some(&params));
        make_parsed_future(handle, req)
    }

    ///Sends APPEND message to twitter API in order to send chunk of media.
    ///
    ///## Parameters:
    ///
    ///* id - Media's id returned in response to `INIT` message.
    ///* chunk - Bytes to send.
    ///* index - Ordered index of file chunk.
    pub fn append(id: u64, chunk: &[u8], index: usize, token: &auth::Token, handle: &Handle) -> FutureResponse<()> {
        let mut params = HashMap::new();

        let config = base64::Config {
            char_set: base64::CharacterSet::Standard,
            newline: base64::Newline::LF,
            pad: true,
            line_length: None,
        };

        add_param(&mut params, "command", "APPEND");
        add_param(&mut params, "media_id", id.to_string());
        add_param(&mut params, "media_data", chunk.to_base64(config));
        add_param(&mut params, "segment_index", index.to_string());

        let req = auth::post(links::medias::UPLOAD, token, Some(&params));
        make_parsed_future(handle, req)
    }

    ///Sends FINALIE message to twitter API in order to finish sending of media.
    ///
    ///## Parameters:
    ///
    ///* id - Media's id returned in response to `INIT` message.
    pub fn finalize(id: u64, token: &auth::Token, handle: &Handle) -> FutureResponse<Media> {
        let mut params = HashMap::new();

        add_param(&mut params, "command", "FINALIZE");
        add_param(&mut params, "media_id", id.to_string());

        let req = auth::post(links::medias::UPLOAD, token, Some(&params));
        make_parsed_future(handle, req)
    }

    ///Sends STATUS message to twitter API in order to retrieve media info.
    pub fn status(id: u64, token: &auth::Token, handle: &Handle) -> FutureResponse<Media> {
        let mut params = HashMap::new();

        add_param(&mut params, "command", "STATUS");
        add_param(&mut params, "media_id", id.to_string());

        let req = auth::get(links::medias::UPLOAD, token, Some(&params));
        make_parsed_future(handle, req)
    }
}

#[cfg(test)]
mod tests {
    use common::FromJson;

    use super::Media;

    use std::fs::File;
    use std::io::Read;

    fn load_media(path: &str) -> Media {
        let mut file = File::open(path).unwrap();
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();
        Media::from_str(&content).unwrap()
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
            Some(super::ProgressInfo::Failed(error)) => assert_eq!(error, "InvalidMedia: Unsupported video format"),
            other => assert!(false, format!("Unexpected value of progress={:?}", other))
        }
    }
}
