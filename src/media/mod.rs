// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//!Media module
//!
//!Provides functionality to upload images, GIFs and videos using Twitter Media API

use std::collections::HashMap;
use error;
use error::Error::{
    InvalidResponse,
    MissingValue
};

use rustc_serialize::{
    json,
    base64
};
use self::base64::{ToBase64};

use common::*;
use links;
use auth;

pub use hyper::mime;

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
pub fn upload_image<'a>(image: &[u8], token: &auth::Token, handle: &'a Handle) -> FutureResponse<'a, Media> {
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
    pub fn init<'a>(total: usize, mime: mime::Mime, token: &auth::Token, handle: &'a Handle) -> FutureResponse<'a, Media> {
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
    pub fn append<'a>(id: u64, chunk: &[u8], index: usize, token: &auth::Token, handle: &'a Handle) -> FutureResponse<'a, ()> {
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
    pub fn finalize<'a>(id: u64, token: &auth::Token, handle: &'a Handle) -> FutureResponse<'a, Media> {
        let mut params = HashMap::new();

        add_param(&mut params, "command", "FINALIZE");
        add_param(&mut params, "media_id", id.to_string());

        let req = auth::post(links::medias::UPLOAD, token, Some(&params));
        make_parsed_future(handle, req)
    }

    ///Sends STATUS message to twitter API in order to retrieve media info.
    pub fn status<'a>(id: u64, token: &auth::Token, handle: &'a Handle) -> FutureResponse<'a, Media> {
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
