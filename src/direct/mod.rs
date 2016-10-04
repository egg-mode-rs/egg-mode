//! Structs and methods for working with direct messages.

use common::*;

use rustc_serialize::json;

use user;
use entities;
use error;
use error::Error::InvalidResponse;

///Represents a single direct message.
pub struct DirectMessage {
    ///Numeric ID for this DM.
    pub id: i64,
    ///UTC timestamp showing when this DM was created, formatted like "Mon Aug 27 17:21:03 +0000
    ///2012".
    pub created_at: String,
    ///The text of the DM.
    pub text: String,
    ///Link, hashtag, and user mention information parsed out of the DM.
    pub entities: DMEntities,
    ///The screen name of the user who sent the DM.
    pub sender_screen_name: String,
    ///The ID of the user who sent the DM.
    pub sender_id: i64,
    ///Full information of the user who sent the DM.
    pub sender: Box<user::TwitterUser>,
    ///The screen name of the user who received the DM.
    pub recipient_screen_name: String,
    ///The ID of the user who received the DM.
    pub recipient_id: i64,
    ///Full information for the user who received the DM.
    pub recipient: Box<user::TwitterUser>,
}

///Container for URL, hashtag, and user mention information associated with a direct message.
pub struct DMEntities {
    ///Collection of hashtags parsed from the DM.
    pub hashtags: Vec<entities::HashtagEntity>,
    ///Collection of URLs parsed from the DM.
    pub urls: Vec<entities::UrlEntity>,
    ///Collection of user mentions parsed from the DM.
    pub user_mentions: Vec<entities::MentionEntity>,
}

impl FromJson for DirectMessage {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse("DirectMessage received json that wasn't an object",
                                       Some(input.to_string())));
        }

        Ok(DirectMessage {
            id: try!(field(input, "id")),
            created_at: try!(field(input, "created_at")),
            text: try!(field(input, "text")),
            entities: try!(field(input, "entities")),
            sender_screen_name: try!(field(input, "sender_screen_name")),
            sender_id: try!(field(input, "sender_id")),
            sender: try!(field(input, "sender")),
            recipient_screen_name: try!(field(input, "recipient_screen_name")),
            recipient_id: try!(field(input, "recipient_id")),
            recipient: try!(field(input, "recipient")),
        })
    }
}

impl FromJson for DMEntities {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse("DMEntities received json that wasn't an object",
                                       Some(input.to_string())));
        }

        Ok(DMEntities {
            hashtags: try!(field(input, "hashtags")),
            urls: try!(field(input, "urls")),
            user_mentions: try!(field(input, "user_mentions")),
        })
    }
}
