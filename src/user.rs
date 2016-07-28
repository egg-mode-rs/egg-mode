use std::borrow::Borrow;
use std::collections::HashMap;
use common::*;
use error;
use error::Error::InvalidResponse;
use auth;
use links;
use rustc_serialize::json;

///Represents a Twitter user.
///
///Field-level documentation is mostly ripped wholesale from [Twitter's user
///documentation][api-user].
///
///[api-user]: https://dev.twitter.com/overview/api/users
#[derive(Debug)]
pub struct TwitterUser {
    ///Indicates this user has an account with "contributor mode" enabled, allowing
    ///for Tweets issued by the user to be co-authored by another account. Rarely `true`.
    pub contributors_enabled: bool,
    //TODO: parse as date?
    ///The UTC datetime that this user account was created on Twitter.
    pub created_at: String,
    ///When true, indicates that this user has not altered the theme or background of
    ///their user profile.
    pub default_profile: bool,
    ///When true, indicates that the user has not uploaded their own avatar and a default
    ///egg avatar is used instead.
    pub default_profile_image: bool,
    ///The user-defined string describing their account.
    pub description: Option<String>,
    //Entities that have been parsed out of the `url` or `description` fields given by
    //the user.
    //TODO: pub entities: Entities,
    ///The number of tweets this user has favorited or liked in the account's lifetime.
    ///The term "favourites" and its British spelling are used for historical reasons.
    pub favourites_count: i32,
    ///When true, indicates that the authenticating user has issued a follow request to
    ///this protected account.
    pub follow_request_sent: Option<bool>,
    ///Indicates whether the authenticating user is following this account. Deprecated
    ///(and thus hidden) due to increasing error conditions where this returns None.
    following: Option<bool>,
    ///The number of followers this account has.
    ///
    ///In certain server-stress conditions, this may temporarily mistakenly return 0.
    pub followers_count: i32,
    ///The number of users this account follows, aka its "followings".
    ///
    ///In certain server-stress conditions, this may temporarily mistakenly return 0.
    pub friends_count: i32,
    ///Indicates whether this user as enabled their tweets to be geotagged.
    ///
    ///If this is set for the current user, then they can attach geographic data when
    ///posting a new Tweet.
    pub geo_enabled: bool,
    ///Unique identifier for this user.
    pub id: i64,
    ///User `id` represented as a string, for compatibility with API clients who cannot
    ///properly handle 64-bit integers.
    pub id_str: String,
    ///Indicates whether the user participates in Twitter's translator community.
    pub is_translator: bool,
    ///Language code for the user's self-declared interface language.
    ///
    ///Codes are formatted as a language tag from [BCP 47][]. Only indicates the user's
    ///interface language, not necessarily the content of their Tweets.
    ///
    ///[BCP 47]: https://tools.ietf.org/html/bcp47
    pub lang: String,
    ///The number of public lists the user is a member of.
    pub listed_count: i32,
    ///The user-entered location field from their profile. Not necessarily parseable
    ///or even a location.
    pub location: Option<String>,
    ///The user-entered display name.
    pub name: String,
    ///Indicates whether the authenticated user has chosen to received this user's tweets
    ///via SMS. Deprecated (and thus hidden) due to bugs where this incorrectly returns
    ///false.
    notifications: Option<bool>,
    ///The hex color chosen by the user for their profile background.
    pub profile_background_color: String,
    ///A URL pointing to the background image chosen by the user for their profile. Uses
    ///HTTP as the protocol.
    pub profile_background_image_url: String,
    ///A URL pointing to the background image chosen by the user for their profile. Uses
    ///HTTPS as the protocol.
    pub profile_background_image_url_https: String,
    ///Indicates whether the user's `profile_background_image_url` should be tiled when
    ///displayed.
    pub profile_background_tile: bool,
    ///A URL pointing to the banner image chosen by the user. Uses HTTPS as the protocol.
    ///
    ///This is a base URL that a size specifier can be appended onto to get variously
    ///sized images, with size specifiers according to [Profile Images and Banners][profile-img].
    ///
    ///[profile-img]: https://dev.twitter.com/overview/general/user-profile-images-and-banners
    pub profile_banner_url: String,
    ///A URL pointing to the user's avatar image. Uses HTTP as the protocol. Size
    ///specifiers can be used according to [Profile Images and Banners][profile-img].
    ///
    ///[profile-img]: https://dev.twitter.com/overview/general/user-profile-images-and-banners
    pub profile_image_url: String,
    ///A URL pointing to the user's avatar image. Uses HTTPS as the protocol. Size
    ///specifiers can be used according to [Profile Images and Banners][profile-img].
    ///
    ///[profile-img]: https://dev.twitter.com/overview/general/user-profile-images-and-banners
    pub profile_image_url_https: String,
    ///The hex color chosen by the user to display links in the Twitter UI.
    pub profile_link_color: String,
    ///The hex color chosen by the user to display sidebar borders in the Twitter UI.
    pub profile_sidebar_border_color: String,
    ///The hex color chosen by the user to display sidebar backgrounds in the Twitter UI.
    pub profile_sidebar_fill_color: String,
    ///The hex color chosen by the user to display text in the Twitter UI.
    pub profile_text_color: String,
    ///Indicates whether the user wants their uploaded background image to be used.
    pub profile_use_background_image: bool,
    ///Indicates whether the user is a [protected][] account.
    ///
    ///[protected]: https://support.twitter.com/articles/14016
    pub protected: bool,
    ///The screen name or handle identifying this user.
    ///
    ///Screen names are unique per-user but can be changed. Use `id`/`id_str` for an
    ///immutable identifier for an account.
    ///
    ///Typically a maximum of 15 characters long, but older accounts may exist with
    ///longer screen names.
    pub screen_name: String,
    ///Indicates that the user would like to see media inline. "Somewhat disused."
    pub show_all_inline_media: Option<bool>,
    //If possible, the most recent tweet or reweet from this user.
    //
    //"Perspectival" items within this tweet that depend on the authenticating user
    //[may not be completely reliable][stale-embed] in this embed.
    //
    //[stale-embed]: https://dev.twitter.com/docs/faq/basics/why-are-embedded-objects-stale-or-inaccurate
    //TODO: pub status: Option<Tweet>,
    ///The number of tweets (including retweets) posted by this user.
    pub statuses_count: i32,
    ///The full name of the time zone the user has set their UI preference to.
    pub time_zone: Option<String>,
    ///The website link given by this user in their profile.
    pub url: Option<String>,
    ///The UTC offset of `time_zone` in minutes.
    pub utc_offset: Option<i32>,
    ///Indicates whether this user is a verified account.
    pub verified: bool,
    ///When present, lists the countries this user has been withheld from.
    pub withheld_in_countries: Option<Vec<String>>,
    ///When present, indicates whether the content being withheld is a "status" or "user".
    pub withheld_scope: Option<String>,
}

impl FromJson for TwitterUser {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse);
        }

        Ok(TwitterUser {
            contributors_enabled: field_bool(input, "contributors_enabled").unwrap_or(false),
            created_at: try!(field_string(input, "created_at")),
            default_profile: try!(field_bool(input, "default_profile")),
            default_profile_image: try!(field_bool(input, "default_profile_image")),
            description: field_string(input, "description").ok(),
            //TODO: entities: ???,
            favourites_count: try!(field_i32(input, "favourites_count")),
            follow_request_sent: field_bool(input, "follow_request_sent").ok(),
            following: field_bool(input, "following").ok(),
            followers_count: try!(field_i32(input, "followers_count")),
            friends_count: try!(field_i32(input, "friends_count")),
            geo_enabled: try!(field_bool(input, "geo_enabled")),
            id: try!(field_i64(input, "id")),
            id_str: try!(field_string(input, "id_str")),
            is_translator: try!(field_bool(input, "is_translator")),
            lang: try!(field_string(input, "lang")),
            listed_count: try!(field_i32(input, "listed_count")),
            location: field_string(input, "location").ok(),
            name: try!(field_string(input, "name")),
            notifications: field_bool(input, "notifications").ok(),
            profile_background_color: try!(field_string(input, "profile_background_color")),
            profile_background_image_url: try!(field_string(input, "profile_background_image_url")),
            profile_background_image_url_https: try!(field_string(input, "profile_background_image_url_https")),
            profile_background_tile: try!(field_bool(input, "profile_background_tile")),
            profile_banner_url: try!(field_string(input, "profile_banner_url")),
            profile_image_url: try!(field_string(input, "profile_image_url")),
            profile_image_url_https: try!(field_string(input, "profile_image_url_https")),
            profile_link_color: try!(field_string(input, "profile_link_color")),
            profile_sidebar_border_color: try!(field_string(input, "profile_sidebar_border_color")),
            profile_sidebar_fill_color: try!(field_string(input, "profile_sidebar_fill_color")),
            profile_text_color: try!(field_string(input, "profile_text_color")),
            profile_use_background_image: try!(field_bool(input, "profile_use_background_image")),
            protected: try!(field_bool(input, "protected")),
            screen_name: try!(field_string(input, "screen_name")),
            show_all_inline_media: field_bool(input, "show_all_inline_media").ok(),
            //TODO: status: ???,
            statuses_count: try!(field_i32(input, "statuses_count")),
            time_zone: field_string(input, "time_zone").ok(),
            url: field_string(input, "url").ok(),
            utc_offset: field_i32(input, "utc_offset").ok(),
            verified: try!(field_bool(input, "verified")),
            withheld_in_countries: input.find("withheld_in_countries").and_then(|f| f.as_array())
                                        .and_then(|arr| arr.iter().map(|x| x.as_string().map(|x| x.to_string()))
                                                           .collect::<Option<Vec<String>>>()),
            withheld_scope: field_string(input, "withheld_scope").ok(),
        })
    }
}

impl TwitterUser {
    pub fn lookup_ids(ids: &[i64], con_token: &auth::Token, access_token: &auth::Token)
        -> Result<Response<Vec<TwitterUser>>, error::Error>
    {
        let mut params = HashMap::new();
        let id_param = ids.iter().map(|x| x.to_string()).collect::<Vec<String>>().join(",");
        add_param(&mut params, "user_id", id_param);

        let mut resp = try!(auth::post(links::users::LOOKUP, con_token, access_token, Some(&params)));

        parse_response(&mut resp)
    }

    pub fn lookup_names<S: Borrow<str>>(names: &[S], con_token: &auth::Token, access_token: &auth::Token)
        -> Result<Response<Vec<TwitterUser>>, error::Error>
    {
        let mut params = HashMap::new();
        let id_param = names.join(",");
        add_param(&mut params, "screen_name", id_param);

        let mut resp = try!(auth::post(links::users::LOOKUP, con_token, access_token, Some(&params)));

        parse_response(&mut resp)
    }
}
