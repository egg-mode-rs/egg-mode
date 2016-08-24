use std::collections::HashMap;
use rustc_serialize::json;
use auth;
use common::*;
use entities;
use error;
use error::Error::InvalidResponse;
use links;
use tweet;

///Convenience enum to generalize between referring to an account by numeric ID or by screen name.
#[derive(Debug, Clone)]
pub enum UserID<'a> {
    ///Referring via the account's numeric ID.
    ID(i64),
    ///Referring via the account's screen name.
    ScreenName(&'a str),
}

impl<'a> From<i64> for UserID<'a> {
    fn from(id: i64) -> UserID<'a> {
        UserID::ID(id)
    }
}

impl<'a> From<&'a i64> for UserID<'a> {
    fn from(id: &'a i64) -> UserID<'a> {
        UserID::ID(*id)
    }
}

impl<'a> From<&'a str> for UserID<'a> {
    fn from(name: &'a str) -> UserID<'a> {
        UserID::ScreenName(name)
    }
}

impl<'a> From<&'a String> for UserID<'a> {
    fn from(name: &'a String) -> UserID<'a> {
        UserID::ScreenName(name.as_str())
    }
}

impl<'a> From<&'a UserID<'a>> for UserID<'a> {
    fn from(id: &'a UserID<'a>) -> UserID<'a> {
        id.clone()
    }
}

/// Represents a Twitter user.
///
/// Field-level documentation is mostly ripped wholesale from [Twitter's user
/// documentation][api-user].
///
/// [api-user]: https://dev.twitter.com/overview/api/users
///
/// The fields present in this struct can be divided up into a few sections: Profile Information and
/// Settings.
///
/// ## Profile Information
///
/// Information here can be considered part of the user's profile. These fields are the "obvious"
/// visible portion of a profile view.
///
/// * `id`
/// * `screen_name`
/// * `name`
/// * `verified`
/// * `protected`
/// * `description`
/// * `location`
/// * `url`
/// * `statuses_count`
/// * `friends_count`
/// * `followers_count`
/// * `favourites_count`
/// * `listed_count`
/// * `profile_image_url`/`profile_image_url_https`
/// * `profile_banner_url`
///
/// ## Settings Information
///
/// Information here can be used to alter the UI around this user, or to provide further metadata
/// that may not necessarily be user-facing.
///
/// * `contributors_enabled`
/// * `created_at`
/// * `default_profile_image`
/// * `follow_request_sent`
/// * `default_profile`, `profile_background_color`, `profile_background_image_url`,
///   `profile_background_image_url_https`, `profile_background_tile`, `profile_link_color`,
///   `profile_sidebar_border_color`, `profile_sidebar_fill_color`, `profile_text_color`,
///   `profile_use_background_image`: These fields can be used to theme a user's profile page to
///   look like the settings they've set on the Twitter website.
/// * `geo_enabled`
/// * `is_translator`
/// * `lang`
/// * `show_all_inline_media`
/// * `time_zone`/`utc_offset`
/// * `withheld_in_countries`/`withheld_scope`
#[derive(Debug)]
pub struct TwitterUser {
    ///Indicates this user has an account with "contributor mode" enabled, allowing
    ///for Tweets issued by the user to be co-authored by another account. Rarely `true`.
    pub contributors_enabled: bool,
    //TODO: parse as date?
    ///The UTC datetime that this user account was created on Twitter, formatted like "Tue Jan
    ///13 23:37:34 +0000 2015".
    pub created_at: String,
    ///When true, indicates that this user has not altered the theme or background of
    ///their user profile.
    pub default_profile: bool,
    ///When true, indicates that the user has not uploaded their own avatar and a default
    ///egg avatar is used instead.
    pub default_profile_image: bool,
    ///The user-defined string describing their account.
    pub description: Option<String>,
    ///Link information that has been parsed out of the `url` or `description` fields given by the
    ///user.
    pub entities: UserEntities,
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
    pub profile_background_image_url: Option<String>,
    ///A URL pointing to the background image chosen by the user for their profile. Uses
    ///HTTPS as the protocol.
    pub profile_background_image_url_https: Option<String>,
    ///Indicates whether the user's `profile_background_image_url` should be tiled when
    ///displayed.
    pub profile_background_tile: Option<bool>,
    ///A URL pointing to the banner image chosen by the user. Uses HTTPS as the protocol.
    ///
    ///This is a base URL that a size specifier can be appended onto to get variously
    ///sized images, with size specifiers according to [Profile Images and Banners][profile-img].
    ///
    ///[profile-img]: https://dev.twitter.com/overview/general/user-profile-images-and-banners
    pub profile_banner_url: Option<String>,
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
    ///Screen names are unique per-user but can be changed. Use `id` for an immutable identifier
    ///for an account.
    ///
    ///Typically a maximum of 15 characters long, but older accounts may exist with longer screen
    ///names.
    pub screen_name: String,
    ///Indicates that the user would like to see media inline. "Somewhat disused."
    pub show_all_inline_media: Option<bool>,
    ///If possible, the most recent tweet or retweet from this user.
    ///
    ///"In some circumstances, this data cannot be provided and this field will be omitted, null,
    ///or empty." Do not depend on this field being filled. (Consequently, I can't say at the
    ///moment whether this actually refers to their most recent tweet or if this has been
    ///overloaded to display their pinned tweet if available.)
    ///
    ///"Perspectival" items within this tweet that depend on the authenticating user
    ///[may not be completely reliable][stale-embed] in this embed.
    ///
    ///[stale-embed]: https://dev.twitter.com/docs/faq/basics/why-are-embedded-objects-stale-or-inaccurate
    pub status: Option<Box<tweet::Tweet>>,
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

///Container for URL entity information that may be paired with a user's profile.
#[derive(Debug)]
pub struct UserEntities {
    ///URL information that has been parsed out of the user's `description`. If no URLs were
    ///detected, then the contained Vec will be empty.
    pub description: UserEntityDetail,
    ///Link information for the user's `url`.
    ///
    ///If `url` is present on the user's profile, so will this field. Twitter validates the URL
    ///entered to a user's profile when they save it, so this can be reasonably assumed to have URL
    ///information if it's present.
    pub url: Option<UserEntityDetail>,
}

///Represents a collection of URL entity information paired with a specific user profile field.
#[derive(Debug)]
pub struct UserEntityDetail {
    ///Collection of URL entity information.
    ///
    ///There should be one of these per URL in the paired field. In the case of the user's
    ///`description`, if no URLs are present, this field will still be present, but empty.
    pub urls: Vec<entities::UrlEntity>,
}

impl FromJson for TwitterUser {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse);
        }

        Ok(TwitterUser {
            contributors_enabled: field(input, "contributors_enabled").unwrap_or(false),
            created_at: try!(field(input, "created_at")),
            default_profile: try!(field(input, "default_profile")),
            default_profile_image: try!(field(input, "default_profile_image")),
            description: field(input, "description").ok(),
            entities: try!(field(input, "entities")),
            favourites_count: try!(field(input, "favourites_count")),
            follow_request_sent: field(input, "follow_request_sent").ok(),
            following: field(input, "following").ok(),
            followers_count: try!(field(input, "followers_count")),
            friends_count: try!(field(input, "friends_count")),
            geo_enabled: try!(field(input, "geo_enabled")),
            id: try!(field(input, "id")),
            is_translator: try!(field(input, "is_translator")),
            lang: try!(field(input, "lang")),
            listed_count: try!(field(input, "listed_count")),
            location: field(input, "location").ok(),
            name: try!(field(input, "name")),
            notifications: field(input, "notifications").ok(),
            profile_background_color: try!(field(input, "profile_background_color")),
            profile_background_image_url: field(input, "profile_background_image_url").ok(),
            profile_background_image_url_https: field(input, "profile_background_image_url_https").ok(),
            profile_background_tile: field(input, "profile_background_tile").ok(),
            profile_banner_url: field(input, "profile_banner_url").ok(),
            profile_image_url: try!(field(input, "profile_image_url")),
            profile_image_url_https: try!(field(input, "profile_image_url_https")),
            profile_link_color: try!(field(input, "profile_link_color")),
            profile_sidebar_border_color: try!(field(input, "profile_sidebar_border_color")),
            profile_sidebar_fill_color: try!(field(input, "profile_sidebar_fill_color")),
            profile_text_color: try!(field(input, "profile_text_color")),
            profile_use_background_image: try!(field(input, "profile_use_background_image")),
            protected: try!(field(input, "protected")),
            screen_name: try!(field(input, "screen_name")),
            show_all_inline_media: field(input, "show_all_inline_media").ok(),
            status: field(input, "status").map(Box::new).ok(),
            statuses_count: try!(field(input, "statuses_count")),
            time_zone: field(input, "time_zone").ok(),
            url: field(input, "url").ok(),
            utc_offset: field(input, "utc_offset").ok(),
            verified: try!(field(input, "verified")),
            withheld_in_countries: input.find("withheld_in_countries").and_then(|f| f.as_array())
                                        .and_then(|arr| arr.iter().map(|x| x.as_string().map(|x| x.to_string()))
                                                           .collect::<Option<Vec<String>>>()),
            withheld_scope: field(input, "withheld_scope").ok(),
        })
    }
}

impl FromJson for UserEntities {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse);
        }

        Ok(UserEntities {
            description: try!(field(input, "description")),
            url: field(input, "url").ok(),
        })
    }
}

impl FromJson for UserEntityDetail {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse);
        }

        Ok(UserEntityDetail {
            urls: try!(field(input, "urls")),
        })
    }
}

///Represents an active user search.
///
///This struct is returned by [`search`][] and is meant to be used as an iterator. This means that
///all the standard iterator adaptors can be used to work with the results:
///
///[`search`]: fn.search.html
///
///```rust,no_run
///# let con_token = egg_mode::Token::new("", "");
///# let access_token = egg_mode::Token::new("", "");
///for name in egg_mode::user::search("rustlang", &con_token, &access_token)
///                                  .map(|u| u.unwrap().response.screen_name).take(10) {
///    println!("{}", name);
///}
///```
///
///You can even collect the results, letting you get one set of rate-limit information for the
///entire search setup:
///
///```rust,no_run
///# let con_token = egg_mode::Token::new("", "");
///# let access_token = egg_mode::Token::new("", "");
///use egg_mode::Response;
///use egg_mode::user::TwitterUser;
///use egg_mode::error::Error;
///
///let names: Result<Response<Vec<TwitterUser>>, Error> =
///    egg_mode::user::search("rustlang", &con_token, &access_token).take(10).collect();
///```
///
///`UserSearch` has a couple adaptors of its own that you can use before consuming it.
///`with_page_size` will let you set how many users are pulled in with a single network call, and
///`start_at_page` lets you start your search at a specific page. Calling either of these after
///starting iteration will clear any current results.
///
///The type returned by the iterator is `Result<Response<TwitterUser>, Error>`, so network errors,
///rate-limit errors and other issues are passed directly through to `next()`. This also means that
///getting an error while iterating doesn't mean you're at the end of the list; you can wait for
///the network connection to return or for the rate limit to refresh before trying again.
///
///## Manual paging
///
///The iterator works by lazily loading a page of results at a time (with size set by
///`with_page_size` or by directly assigning `page_size`) in the background whenever you ask for
///the next result. This can be nice, but it also means that you can lose track of when your loop
///will block for the next page of results. This is where the extra fields and methods on
///`UserSearch` come in. By using the raw `call()` function and changing `page_num` as necessary,
///you can have full control over when the network calls happen:
///
///```rust,no_run
///# let con_token = egg_mode::Token::new("", "");
///# let access_token = egg_mode::Token::new("", "");
///let mut search = egg_mode::user::search("rustlang", &con_token, &access_token).with_page_size(20);
///let resp = search.call().unwrap();
///
///for user in resp.response {
///    println!("{} (@{})", user.name, user.screen_name);
///}
///
///search.page_num += 1;
///let resp = search.call().unwrap();
///
///for user in resp.response {
///    println!("{} (@{})", user.name, user.screen_name);
///}
///```
#[must_use = "search iterators are lazy and do nothing unless consumed"]
pub struct UserSearch<'a> {
    con_token: &'a auth::Token<'a>,
    access_token: &'a auth::Token<'a>,
    query: &'a str,
    ///The current page of results being returned, starting at 1.
    pub page_num: i32,
    ///The number of user records per page of results. Defaults to 10, maximum of 20.
    pub page_size: i32,
    current_results: Option<ResponseIter<TwitterUser>>,
}

impl<'a> UserSearch<'a> {
    ///Sets the page size used for the search query.
    ///
    ///Calling this will invalidate any current search results, making the next call to `next()`
    ///perform a network call.
    pub fn with_page_size(self, page_size: i32) -> Self {
        UserSearch {
            con_token: self.con_token,
            access_token: self.access_token,
            query: self.query,
            page_num: self.page_num,
            page_size: page_size,
            current_results: None,
        }
    }

    ///Sets the starting page number for the search query.
    ///
    ///Calling this will invalidate any current search results, making the next call to `next()`
    ///perform a network call.
    pub fn start_at_page(self, page_num: i32) -> Self {
        UserSearch {
            con_token: self.con_token,
            access_token: self.access_token,
            query: self.query,
            page_num: page_num,
            page_size: self.page_size,
            current_results: None,
        }
    }

    ///Performs the search for the current page of results.
    ///
    ///This will automatically be called if you use the `UserSearch` as an iterator. This method is
    ///made public for convenience if you want to manage the pagination yourself. Remember to
    ///change `page_num` between calls.
    pub fn call(&self) -> WebResponse<Vec<TwitterUser>> {
        let mut params = HashMap::new();
        add_param(&mut params, "q", self.query);
        add_param(&mut params, "page", self.page_num.to_string());
        add_param(&mut params, "count", self.page_size.to_string());

        let mut resp = try!(auth::get(links::users::SEARCH, self.con_token, self.access_token, Some(&params)));

        parse_response(&mut resp)
    }

    ///Returns a new UserSearch with the given query and tokens, with the default page size of 10.
    pub fn new(query: &'a str, con_token: &'a auth::Token, access_token: &'a auth::Token) -> UserSearch<'a> {
        UserSearch {
            con_token: con_token,
            access_token: access_token,
            query: query,
            page_num: 1,
            page_size: 10,
            current_results: None,
        }
    }
}

impl<'a> Iterator for UserSearch<'a> {
    type Item = WebResponse<TwitterUser>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(ref mut results) = self.current_results {
            if let Some(user) = results.next() {
                return Some(Ok(user));
            }
            else if (results.len() as i32) < self.page_size {
                return None;
            }
            else {
                self.page_num += 1;
            }
        }

        match self.call() {
            Ok(resp) => {
                let mut iter = resp.into_iter();
                let first = iter.next();
                self.current_results = Some(iter);
                match first {
                    Some(user) => Some(Ok(user)),
                    None => None,
                }
            },
            Err(err) => {
                //Invalidate current results so we don't increment the page number again
                self.current_results = None;
                Some(Err(err))
            },
        }
    }
}

///Represents relationship settings between two Twitter accounts.
#[derive(Debug)]
pub struct Relationship {
    ///Contains settings from the perspective of the target account.
    pub target: RelationTarget,
    ///Contains settings from the perspective of the source account.
    ///
    ///This contains more information than `target` if the source account is the same as the
    ///authenticated user. See the [`RelationSource`][] page for details.
    ///
    ///[`RelationSource`]: struct.RelationSource.html
    pub source: RelationSource,
}

impl FromJson for Relationship {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse);
        }

        if let Some(relation) = input.find("relationship") {
            Ok(Relationship {
                target: try!(field(relation, "target")),
                source: try!(field(relation, "source")),
            })
        }
        else {
            Err(error::Error::MissingValue("relationship"))
        }
    }
}

///Represents relationship settings between two Twitter accounts, from the perspective of the
///target user.
#[derive(Debug)]
pub struct RelationTarget {
    ///Numeric ID for this account.
    pub id: i64,
    ///Screen name for this account.
    pub screen_name: String,
    ///Indicates whether the source account follows this target account.
    pub followed_by: bool,
    ///Indicates whether this target account follows the source account.
    pub following: bool,
}

impl FromJson for RelationTarget {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse);
        }

        Ok(RelationTarget {
            id: try!(field(input, "id")),
            screen_name: try!(field(input, "screen_name")),
            followed_by: try!(field(input, "followed_by")),
            following: try!(field(input, "following")),
        })
    }
}

///Represents relationship settings between two Twitter accounts, from the perspective of the
///source user.
///
///This struct holds more information than the `RelationTarget` struct, mainly attributes only
///visible to the user that set them. While you can see relationships between any two arbitrary
///users, if the "source" account is the same one whose access token you're using, you can see
///extra information about this relationship.
#[derive(Debug)]
pub struct RelationSource {
    ///Numeric ID for this account.
    pub id: i64,
    ///Screen name for this account.
    pub screen_name: String,
    ///Indicates whether this source account follows the target account.
    pub following: bool,
    ///Indicates whether the target account follows this source account.
    pub followed_by: bool,
    ///Indicates whether this source account can send a direct message to the target account.
    ///
    ///If `followed_by` is false but this is true, that could indicate that the target account has
    ///allowed anyone to direct-message them.
    pub can_dm: bool,
    ///Indicates whether this source account is blocking the target account. If the source account
    ///is not the authenticated user, holds `None` instead.
    pub blocking: Option<bool>,
    ///Indicates whether this source account has reported the target account for spam. If the source
    ///account is not the authenticated user, holds `None` instead.
    pub marked_spam: Option<bool>,
    ///Indicates whether this source account has decided to receive all replies from the target
    ///account. If the source account is not the authenticated user, holds `None` instead.
    ///
    ///Note that there is no mechanism with which to toggle this setting, at least none that this
    ///author could find, either through the API or through the official site.
    all_replies: Option<bool>,
    ///Indicates whether this source account has decided to show retweets from the target account.
    ///If the source account is not the authenticated user, holds `None` instead.
    pub want_retweets: Option<bool>,
    ///Indicates whether this source account has decided to receive mobile notifications for the
    ///target account. If the source account is not the authenticated user, holds `None` instead.
    pub notifications_enabled: Option<bool>,
}

impl FromJson for RelationSource {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse);
        }

        Ok(RelationSource {
            id: try!(field(input, "id")),
            screen_name: try!(field(input, "screen_name")),
            following: try!(field(input, "following")),
            followed_by: try!(field(input, "followed_by")),
            can_dm: try!(field(input, "can_dm")),
            blocking: field(input, "blocking").ok(),
            marked_spam: field(input, "marked_spam").ok(),
            all_replies: field(input, "all_replies").ok(),
            want_retweets: field(input, "want_retweets").ok(),
            notifications_enabled: field(input, "notifications_enabled").ok(),
        })
    }
}

///Represents the relation the authenticated user has to a given account.
///
///This is returned by `relation_lookup`, as opposed to `Relationship`, which is returned by
///`relation`.
#[derive(Debug)]
pub struct RelationLookup {
    ///The display name of the target account.
    pub name: String,
    ///The screen name of the target account.
    pub screen_name: String,
    ///The numeric ID of the target account.
    pub id: i64,
    ///The ways the target account is connected to the authenticated user.
    ///
    ///If the target account has no relation to the authenticated user, this will not be empty; its
    ///only element will be `None`.
    pub connections: Vec<Connection>,
}

impl FromJson for RelationLookup {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if !input.is_object() {
            return Err(InvalidResponse);
        }

        Ok(RelationLookup {
            name: try!(field(input, "name")),
            screen_name: try!(field(input, "screen_name")),
            id: try!(field(input, "id")),
            connections: try!(field(input, "connections")),
        })
    }
}

///Represents the ways a target account can be connected to another account.
#[derive(Debug)]
pub enum Connection {
    ///The target account has no relation.
    None,
    ///The authenticated user has requested to follow the target account.
    FollowingRequested,
    ///The target account has requested to follow the authenticated user.
    FollowingReceived,
    ///The target account follows the authenticated user.
    FollowedBy,
    ///The authenticated user follows the target account.
    Following,
    ///The authenticated user has blocked the target account.
    Blocking,
    ///The authenticated user has muted the target account.
    Muting,
}

impl FromJson for Connection {
    fn from_json(input: &json::Json) -> Result<Self, error::Error> {
        if let Some(text) = input.as_string() {
            match text {
                "none" => Ok(Connection::None),
                "following_requested" => Ok(Connection::FollowingRequested),
                "following_received" => Ok(Connection::FollowingReceived),
                "followed_by" => Ok(Connection::FollowedBy),
                "following" => Ok(Connection::Following),
                "blocking" => Ok(Connection::Blocking),
                "muting" => Ok(Connection::Muting),
                _ => {
                    println!("{}", text);
                    Err(InvalidResponse)
                },
            }
        }
        else {
            Err(InvalidResponse)
        }
    }
}
