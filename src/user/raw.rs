// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use common::*;
use tweet;

use chrono;

use super::UserEntities;

#[derive(Debug, Clone, Deserialize)]
pub struct RawTwitterUser {
    /// Indicates this user has an account with "contributor mode" enabled, allowing
    /// for Tweets issued by the user to be co-authored by another account. Rarely `true`.
    pub contributors_enabled: bool,
    /// The UTC timestamp for when this user account was created on Twitter.
    #[serde(deserialize_with = "deserialize_datetime")]
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// When true, indicates that this user has not altered the theme or background of
    /// their user profile.
    pub default_profile: bool,
    /// When true, indicates that the user has not uploaded their own avatar and a default
    /// egg avatar is used instead.
    pub default_profile_image: bool,
    /// The user-defined string describing their account.
    pub description: Option<String>,
    /// Link information that has been parsed out of the `url` or `description` fields given by the
    /// user.
    #[serde(default)]
    pub entities: UserEntities,
    /// The number of tweets this user has favorited or liked in the account's lifetime.
    /// The term "favourites" and its British spelling are used for historical reasons.
    pub favourites_count: i32,
    /// When true, indicates that the authenticating user has issued a follow request to
    /// this protected account.
    pub follow_request_sent: Option<bool>,
    /// The number of followers this account has.
    ///
    /// In certain server-stress conditions, this may temporarily mistakenly return 0.
    pub followers_count: i32,
    /// The number of users this account follows, aka its "followings".
    ///
    /// In certain server-stress conditions, this may temporarily mistakenly return 0.
    pub friends_count: i32,
    /// Indicates whether this user as enabled their tweets to be geotagged.
    ///
    /// If this is set for the current user, then they can attach geographic data when
    /// posting a new Tweet.
    pub geo_enabled: bool,
    /// Unique identifier for this user.
    pub id: u64,
    /// Indicates whether the user participates in Twitter's translator community.
    pub is_translator: bool,
    /// Language code for the user's self-declared interface language.
    ///
    /// Codes are formatted as a language tag from [BCP 47][]. Only indicates the user's
    /// interface language, not necessarily the content of their Tweets.
    ///
    /// [BCP 47]: https://tools.ietf.org/html/bcp47
    pub lang: String,
    /// The number of public lists the user is a member of.
    pub listed_count: i32,
    /// The user-entered location field from their profile. Not necessarily parseable
    /// or even a location.
    pub location: Option<String>,
    /// The user-entered display name.
    pub name: String,
    /// The hex color chosen by the user for their profile background.
    pub profile_background_color: String,
    /// A URL pointing to the background image chosen by the user for their profile. Uses
    /// HTTP as the protocol.
    pub profile_background_image_url: Option<String>,
    /// A URL pointing to the background image chosen by the user for their profile. Uses
    /// HTTPS as the protocol.
    pub profile_background_image_url_https: Option<String>,
    /// Indicates whether the user's `profile_background_image_url` should be tiled when
    /// displayed.
    pub profile_background_tile: Option<bool>,
    /// A URL pointing to the banner image chosen by the user. Uses HTTPS as the protocol.
    ///
    /// This is a base URL that a size specifier can be appended onto to get variously
    /// sized images, with size specifiers according to [Profile Images and Banners][profile-img].
    ///
    /// [profile-img]: https://dev.twitter.com/overview/general/user-profile-images-and-banners
    pub profile_banner_url: Option<String>,
    /// A URL pointing to the user's avatar image. Uses HTTP as the protocol. Size
    /// specifiers can be used according to [Profile Images and Banners][profile-img].
    ///
    /// [profile-img]: https://dev.twitter.com/overview/general/user-profile-images-and-banners
    pub profile_image_url: String,
    /// A URL pointing to the user's avatar image. Uses HTTPS as the protocol. Size
    /// specifiers can be used according to [Profile Images and Banners][profile-img].
    ///
    /// [profile-img]: https://dev.twitter.com/overview/general/user-profile-images-and-banners
    pub profile_image_url_https: String,
    /// The hex color chosen by the user to display links in the Twitter UI.
    pub profile_link_color: String,
    /// The hex color chosen by the user to display sidebar borders in the Twitter UI.
    pub profile_sidebar_border_color: String,
    /// The hex color chosen by the user to display sidebar backgrounds in the Twitter UI.
    pub profile_sidebar_fill_color: String,
    /// The hex color chosen by the user to display text in the Twitter UI.
    pub profile_text_color: String,
    /// Indicates whether the user wants their uploaded background image to be used.
    pub profile_use_background_image: bool,
    /// Indicates whether the user is a [protected][] account.
    ///
    /// [protected]: https://support.twitter.com/articles/14016
    pub protected: bool,
    /// The screen name or handle identifying this user.
    ///
    /// Screen names are unique per-user but can be changed. Use `id` for an immutable identifier
    /// for an account.
    ///
    /// Typically a maximum of 15 characters long, but older accounts may exist with longer screen
    /// names.
    pub screen_name: String,
    /// Indicates that the user would like to see media inline. "Somewhat disused."
    pub show_all_inline_media: Option<bool>,
    /// If possible, the most recent tweet or retweet from this user.
    ///
    /// "In some circumstances, this data cannot be provided and this field will be omitted, null,
    /// or empty." Do not depend on this field being filled. Also note that this is actually their
    /// most-recent tweet, not the status pinned to their profile.
    ///
    /// "Perspectival" items within this tweet that depend on the authenticating user
    /// [may not be completely reliable][stale-embed] in this embed.
    ///
    /// [stale-embed]: https://dev.twitter.com/docs/faq/basics/why-are-embedded-objects-stale-or-inaccurate
    pub status: Option<Box<tweet::Tweet>>,
    /// The number of tweets (including retweets) posted by this user.
    pub statuses_count: i32,
    /// The full name of the time zone the user has set their UI preference to.
    pub time_zone: Option<String>,
    /// The website link given by this user in their profile.
    pub url: Option<String>,
    /// The UTC offset of `time_zone` in minutes.
    pub utc_offset: Option<i32>,
    /// Indicates whether this user is a verified account.
    pub verified: bool,
    /// When present, lists the countries this user has been withheld from.
    pub withheld_in_countries: Option<Vec<String>>,
    /// When present, indicates whether the content being withheld is a "status" or "user".
    pub withheld_scope: Option<String>,
}
