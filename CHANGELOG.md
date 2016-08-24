# Changelog for egg-mode

## Pending
### Added
- New module `tweet` and `Tweet` struct
- New function to load a single tweet (`show`)

### Changed
- Moved `UserID` into the user module
  - This is a **breaking change** if you used the type directly (like in the lookup example)
- Changed the signature of `user::relation_lookup` to match `user::lookup`

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