# Changelog for egg-mode

## Pending
### Added
- `media` module and all its contents, for uploading pictures/video/gifs to Twitter
  - Thanks to @DoumanAsh for the initial implementation!
  - `UploadBuilder` and `UploadFuture`, for uploading images, GIFs, and videos to Twitter
  - `MediaHandle`, a media ID wrapped with a marker of how long it's valid (returned by
    `UploadFuture`)
  - `UploadError` and `UploadState`, a special wrapper for `UploadFuture` errors
  - `media::media_types`, convenience methods to get media types for formats Twitter can support
- New methods on `DraftTweet`:
  - `media_ids` to attach media to a tweet
  - `possibly_sensitive` to mark attached media as "possibly sensitive", giving it a click-through
    mask when posted

### Changed
- `TwitterFuture` clones the `Handle` internally, removing its lifetime parameter
- `SearchBuilder`, `UserSearch`, and `DraftTweet` now use Cows instead of plain references
  internally, allowing for owned data to be passed in to make them `'static`
  - As `SearchFuture` and `SearchResult` also use the same components as their base `SearchBuilder`,
    a `'static` `SearchBuilder` will also create '`static` versions of those structs
- `CursorIter`, `direct::Timeline`, `tweet::Timeline`, `CachedSearchFuture`, and `UserSearch` now
  clone the `Token` that they're handed, instead of keeping a reference, allowing them to be
  `'static`
  - As `ConversationFuture` holds instances of `direct::Timeline`, it has also become `'static`
- `tweet::Timeline`'s futures now consume the parent `Timeline` and return it (alongside the chunk
  of posts) upon success. This is a **breaking change**

### Removed
- The `text` module has been removed, in favor of a separate `egg-mode-text` crate
  - This is a **breaking change**, and the updated egg-mode-text has a different signature for
    `character_count` and `characters_remaining` due to the 280-character update

## [0.11.0] - 2017-10-16
### Changed
- The Great Async Refactor of 0.11.0
  - egg-mode now uses async i/o!!!
  - This is **a massively breaking change**!!!
  - All functions that need to contact the network now take a `tokio_core::reactor::Handle` now, to
    represent which event loop to run the requests on
  - All functions that need to contact the network now return a new type `TwitterFuture`, which
    represents the in-progress request
  - All the iterator wrappers are now Streams
  - In the refactor, `invalidate_bearer` was changed to panic on receiving non-Bearer tokens, rather
    than short-circuiting an error.
  - In the refactor, the methods on `direct::ConversationTimeline` were changed to consume the
    Timeline and return it at the end of a successful load.
  - There's a new variant of `Error`, `FutureAlreadyCompleted`, for when a Future was polled after
    it already returned a value. This is a **breaking change** if you were exhaustively matching on
    `Error` before.
- Several dependencies have been updated
  - Notably, the type for all the timestamps was renamed, since chrono changed it from `UTC` to
    `Utc`
- `Token`s and `KeyPair`s are now always `'static`. Only string literals and owned strings from now
  on.
  - This is a **breaking change** if you were using short-lived `Token`s with `&str`s in them -
    clone or `.to_owned()` the Strings when you hand them to the `KeyPair`, please.

## [0.10.0] = 2017-08-08

0.10.0 is a quick patch to change out my dependencies: `ring` has been removed, in favor of `hmac`
and `sha1`, which were the only operations i needed from it. This should solve any linker errors
when multiple versions of `ring` are in the same tree, by removing one instance of `ring` entirely.

## [0.9.0] - 2017-06-23
### Changed
- **The library is now licensed under the Mozilla Public License, 2.0.**
- `tweet::lookup`, `tweet::lookup_map`, `user::lookup`, and `user::relation_lookup` now all take
  `IntoIterator`s instead of slices.
  - Only a **breaking change** if something that coerces into a slice doesn't also impl
    `IntoIterator<Item=&'a T>`. As Vecs and slices already do this, the common cases are
    source-compatible.
- `user::UserID` is Copy now. It was originally Clone to support the `From<&UserID> for UserID`
  impl, but all the elements inside were Copy anyway, so it makes sense to add it there.
- A bunch of structs and enums now impl Clone, and a few also now impl Copy, where applicable.

### Added
- Added module `list` with `List` and `ListID` structs
  - Thanks to @jkarns275 for the initial implementation!
  - New function `show` to pull up information for a single list
  - New function `ownerships` to pull up lists created by a given user
  - New function `subscriptions` to pull up lists a given user is following that they didn't make
    themselves
  - New function `list` to pull up the combination of `ownerships` and `subscriptions`, up to 100
    entries
  - New function `members` to pull up the users included in a given list
  - New function `memberships` to pull up lists the given user has been added to
  - New function `is_member` to check whether a given user has been included in a given list
  - New function `statuses` to load the tweets posted by the members of a given list
  - New function `subscribers` to pull up the users subscribed to a given list
  - New function `is_subscribed` to check whether a given user is subscribed to a given list
  - New function `add_member` to add a user to a list
  - New function `create` to create a new list
  - New function `delete` to delete a list
  - New function `remove_member` to remove a member from a list
  - New function `subscribe` to subscribe to a list
  - New function `unsubscribe` to unsubscribe from a list
  - New function `add_member_list` to add multiple users to a list
  - New function `remove_member_list` to remove multiple users from a list
  - New function `update` to update the name/visibility/description of an already-existing list
- New enum `service::ListMethod` and `list` field in `service::RateLimitStatus` to contain the rate
  limit info for these methods

## [0.8.1] - 2017-05-18

0.8.1 is a quick patch to update to `*ring*` version 0.9.4, to prevent linking issues when used with
`*ring*` 0.9.x.

## [0.8.0] - 2017-01-30
### Added
- New authentication functions `bearer_token` and `invalidate_bearer`, to perform Application-only
  authentication
- New function `direct::conversations` and related structs `ConversationTimeline` and
  `DMConversations` to load direct messages as a pre-sorted set of conversation threads

### Changed
- The Great Token Refactor of 0.8
  - Rather than forcing consumers of the library to handle their consumer token and access token
    separately, the key/token pairs have been renamed to `KeyPair` while the `Token` struct has been
    turned into an enum that contains the two `KeyPair`s necessary to authenticate to Twitter.
  - Every function that connects to twitter no longer takes the consumer token and access token
    separately, instead taking the new `Token` enum to authenticate.
  - The `access_token` function now returns the `Token` enum which includes the newly-minted access
    token, in addition to consuming the consumer token given to it.
  - `KeyPair` and `Token` implement Clone, so the consumer token can be cloned when given to
    `access_token` if multiple account support is necessary.
  - All of the above amounts to **an enormous breaking change**, for which I must apologize. It's
    this way so I can support application-only authentication. Hopefully the loss of extra text for
    all those function calls can make up for it.
- Quality-of-life changes to various structs, all of which are **breaking changes**, semantically if
  not syntactically:
  - `Tweet::display_text_range` is now `Option<(usize, usize)>` and counts byte offsets instead of
    character indices
  - All IDs used in any API struct are now u64, except for cursor IDs, since they need to be able to
    be -1
  - All `indices` for Entity structs are now called `range`, are now `(usize, usize)`, and refer to
    byte offsets instead of codepoint offsets
- `user::lookup_ids` and `user::lookup_names`, which were deprecated in 0.4.0, have been removed, in
  favor of `user::lookup`
  - This is a **breaking change**
- Updated `hyper` to 0.10 and added `hyper-native-tls` for TLS connections to Twitter
  - This should allow for much easier building on Windows now!

## [0.7.0] - 2016-11-30
### Added
- New iterator structs `ResponseIterRef` and `ResponseIterMut` to iterate over
  references in a response
- New module `text` to handle entity extraction and character count of arbitrary text
  - New function `url_entities` to parse URLs from given text
  - New function `character_count` to count characters in the given text after
    accounting for URL shortening
  - New function `characters_remaining` to provide a convenience method for displaying
    the number of characters remaining in a 140-character tweet after including the given text
  - New function `mention_list_entities` to parse user and list mentions from given text
  - New function `mention_entities` to parse just user mentions from given text
  - New function `hashtag_entities` to parse just hashtags from given text
  - New function `symbol_entities` to parse just cashtags from given text
  - New function `reply_mention_entity` to parse a screen name if the given text is a reply
  - New function `entities` to parse out all of the above

### Changed
- `Place::contained_within` is now a `Option<Vec<Place>>`, because I had the wrong type for that before
  - This is a **breaking change** if you were examining that field as a single location before
- Struct fields which are optional now only return None for their absence, instead of absorbing all their errors
  - This led to the following two changes:
- `Tweet::user` is now optional
  - Twitter stopped returning users for the tweet in `TwitterUser::status`, so that field is optional now
  - This is a **breaking change**
- `Tweet::coordinates` is now properly parsed

## [0.6.0] - 2016-10-20
### Added
- New module `direct` and `DirectMessage`/`DMEntities` structs
  - New function to load a single direct message (`show`)
  - New function to load DMs received by the authenticated user (`received`)
  - New function to load DMs sent by the authenticated user (`sent`)
  - New function to send a DM (`send`)
  - New function to delete a previously-sent DM (`delete`)
- New module `service` for miscellaneous broad methods about Twitter as a whole
  - New function `terms` to load the Terms of Service
  - New function `privacy` to load the Privacy Policy
  - New function `config` to load broad service-level configuration elements
  - New function `rate_limit_status` to load current rate-limit information for most methods

### Changed
- All `created_at` timestamps are now parsed through `chrono`
  - This is a notable **breaking change** if you were handling this parsing yourself
- `tweet::source` is now a new `TweetSource` struct and no longer an HTML String
  - This is a notable **breaking change** if you were parsing this yourself
- `Response<T>` now implements Deref, so you don't have to use `.response` all the time

## [0.5.0] - 2016-10-02
### Added
- New methods on `SearchBuilder` to constrain initial searches to be before or after given tweet IDs
- New module `place` and `Place` struct
- New function to load a single place from ID (`show`)
- New functions to load a list of places from a specific latitude/longitude (`reverse_geocode`,
  `reverse_geocode_url`)
- New functions to load a list of places from a search query (`search_point`, `search_query`,
  `search_ip`, `search_url`)

### Changed
- Fields on `DraftTweet` are public to facilitate UI for long-term draft storage
- `DraftTweet`'s coordinates are now f64 instead of f32
  - This is a **breaking change** if you had f32 bindings for this purpose
- New error enum, `BadUrl`, telling you that you passed a bad URL to
  `reverse_geocode_url` or `search_url`
  - This is a **breaking change** if you were matching on the error types before
- Made `tweet::Timeline::new()` and `user::UserSearch::new()` non-public
  - This is a **breaking change** if you were using these functions instead of any of the real
    initializers
- Added fields `coordinates` and `place` to the `Tweet` structure
- Made tweet loading functions request and parse extended tweets
- Added fields `display_text_range` and `truncated` to the `Tweet` structure
- Added fields and methods `auto_populate_reply_metadata`,
  `exclude_reply_user_ids`, and `attachment_url` to the `DraftTweet` structure

## [0.4.0] - 2016-09-02
### Added
- New module `tweet` and `Tweet` struct
- New function to load a single tweet (`show`)
- New function to load recent retweets of a single tweet (`retweets_of`)
- New functions to look up a list of tweet IDs (`lookup`, `lookup_map`)
- New `Timeline` struct to navigate timelines and other relatively-indexed collections of tweets
- New function to load user's home timeline (`home_timeline`)
- New function to load user's mentions timeline (`mentions_timeline`)
- New function to load the posts by a given user (`user_timeline`)
- New function to load the user's posts that have been retweeted (`retweets_of_me`)
- New function to load the user IDs who have retweeted a given tweet (`retweeters_of`)
- New function to load the posts liked by a given user (`liked_by`)
- New module `search` to contain all the tweet-search structs and methods
- New functions to retweet and unretweet statuses (`retweet`, `unretweet`)
- New functions to like and un-like statuses (`like`, `unlike`)
- New struct `DraftTweet` to handle assembling new statuses to post
- New function to delete a given tweet (`delete`)

### Changed
- Moved `UserID` into the user module
  - This is a **breaking change** if you used the type directly (like in the lookup example)
- Changed the signature of `user::relation_lookup` to match `user::lookup`
- New field in `user::TwitterUser`: `status`
- New error enum, `RateLimit(i32)`, telling you that you hit your rate limit
  and when it will lapse
  - This is a **breaking change** if you were exhaustively matching on these before
- Introduce a type alias `WebResponse` for `Result<Response<T>, Error>` which was *everywhere*
- Error::InvalidResponse now contains information about where in the code the error occurred
  - This is a **breaking change** if you were matching on the error types before

## [0.3.0] - 2016-08-19
### Added
- New functions to load the muted users list (`mutes`, `mutes_ids`)
- New functions to follow/unfollow a user (`follow`, `unfollow`)
- New function to test the validity of access tokens (`verify_tokens`)
- New functions to see incoming/outgoing follow requests (`incoming_requests`, `outgoing_requests`)
- New function to see friendship status between users (`relation`)
- New function to change notification/retweet settings (`update_follow`)
- New function to list users that the user has disabled retweets from (`friends_no_retweets`)
- New function to look up friendship status for several users (`relation_lookup`)
- New functions to block/unblock users (`block`, `unblock`, `report_spam`)
- New functions to mute/unmute users (`mute`, `unmute`)

### Changed
- Combined IDLoader and UserLoader into CursorIter (with the same interface)
  - This is a **breaking change** if you assigned these results to variables with explicit types
  - (If you merely used them as iterators or didn't explicitly declare their type, the interface is
    the same)
- Moved `TwitterErrors` and `TwitterErrorCode` into the error module
- Moved `Cursor`, `CursorIter`, `UserCursor`, `IDCursor` into a separate module
  - This is a **breaking change** if you used these types directly
  - (If you merely used the iterators and skipped straight to the users/IDs being returned, the
    interface is the same)

## [0.2.0] - 2016-08-08
### Added
- Entity structs, so you can parse URL's from user bios (and from tweets in the future)

### Changed
- Added entity information to the user struct
- Removed dependency on the `time` crate (Thanks, serprex!)

## [0.1.1] - 2016-08-04
### Changed
- Relicense with Apache2 while I figure out how to make LGPL work with Rust

## [0.1.0] - 2016-08-04
### Added
- Initial version
- Auth methods
- User lookup, search, friend/follower list
- "basic" example showing various user lookups
- "reciprocal" example showing the users you mutually follow

<!-- vim: set tw=100 expandtab: -->
