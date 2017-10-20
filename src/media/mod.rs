// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//!Media module
//!
//!Provides functionality to upload images, GIFs and videos using Twitter Media API

use std::collections::HashMap;
use error;
use error::Error::InvalidResponse;

use rustc_serialize::{
    json,
    base64
};
use self::base64::{ToBase64};

use common::*;
use links;
use auth;

///Represents media file that is uploaded on twitter.
pub struct Media {
    ///ID that can be used in API calls (e.g. attach to tweet).
    pub id: u64,
    ///Number of second the media can be used in other API calls.
    pub expires_after: u64,
}

impl FromJson for Media {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse("Tweet received json that wasn't an object", Some(input.to_string())));
        }

        field_present!(input, media_id);
        field_present!(input, expires_after_secs);

        Ok(Media {
            id: try!(field(input, "media_id")),
            expires_after: try!(field(input, "expires_after_secs"))
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

#[cfg(test)]
mod tests {
    use common::FromJson;

    use super::Media;

    use std::fs::File;
    use std::io::Read;

    #[test]
    fn parse_media() {
        let media = {
            let mut file = File::open("src/media/media.json").unwrap();
            let mut content = String::new();
            file.read_to_string(&mut content).unwrap();
            Media::from_str(&content).unwrap()
        };

        assert_eq!(media.id, 710511363345354753);
        assert_eq!(media.expires_after, 86400);
    }
}
