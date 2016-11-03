# The Big List Of API Endpoints

This can be considered a big list of Twitter's API knobs that have and have not been given
corresponding methods in egg-mode. Those that have been implemented will have the method(s) written
alongside them.

I've grouped them by API heading, then by a rough category and data structure. Endpoints that just
return a list of IDs of something are grouped in with the rest of the main structure's list.

## [OAuth](https://dev.twitter.com/oauth/overview)

Implementing OAuth2/Bearer tokens/Application-based auth would need some parallel infrastructure or
some kind of overloading of Tokens, since they send a different header than regular consumer/access
tokens.

- [x] oauth/request\_token (`request_token`)
- [x] oauth/authenticate (`authenticate_url`)
- [x] oauth/authorize (`authorize_url`)
- [x] oauth/access\_token (`access_token`)
- [ ] oauth2/token
- [ ] oauth2/invalidate\_token

## [Public API](https://dev.twitter.com/rest/public)

### Statuses

- [x] statuses/mentions\_timeline (`tweet::mentions_timeline`)
- [x] statuses/user\_timeline (`tweet::user_timeline`)
- [x] statuses/home\_timeline (`tweet::home_timeline`)
- [x] statuses/retweets\_of\_me (`tweet::retweets_of_me`)
- [x] statuses/retweets/:id (`tweet::retweets_of`)
- [x] statuses/show/:id (`tweet::show`)
- [x] statuses/destroy/:id (`tweet::delete`)
- [x] statuses/update (`tweet::DraftTweet::send`)
- [x] statuses/retweet/:id (`tweet::retweet`)
- [x] statuses/unretweet/:id (`tweet::unretweet`)
- [x] statuses/retweeters/ids (`tweet::retweeters_of`)
- [x] statuses/lookup (`tweet::lookup`, `tweet::lookup_map`)
- [x] search/tweets (`search::search`)
- [x] favorites/list (`tweet::liked_by`)
- [x] favorites/create (`tweet::like`)
- [x] favorites/destroy (`tweet::unlike`)

- [ ] statuses/oembed

- [ ] saved\_searches/list
- [ ] saved\_searches/show/:id
- [ ] saved\_searches/create
- [ ] saved\_searches/destroy/:id

- [ ] trends/place
- [ ] trends/available
- [ ] trends/closest

### Direct Messages

- [x] direct\_messages (`direct::received`)
- [x] direct\_messages/sent (`direct::sent`)
- [x] direct\_messages/show (`direct::show`)
- [x] direct\_messages/new (`direct::send`)
- [x] direct\_messages/destroy (`direct::delete`)

### Users

- [x] users/show (`user::show`)
- [x] users/lookup (`user::lookup`)
- [x] users/search (`user::search`)
- [x] friends/list (`user::friends_of`)
- [x] friends/ids (`user::friends_ids`)
- [x] friendships/create (`user::follow`)
- [x] friendships/update (`user::update_follow`)
- [x] friendships/destroy (`user::unfollow`)
- [x] friendships/incoming (`user::incoming_requests`)
- [x] friendships/outgoing (`user::outgoing_requests`)
- [x] friendships/no\_retweets/ids (`user::friends_no_retweets`)
- [x] followers/list (`user::followers_of`)
- [x] followers/ids (`user::followers_ids`)
- [x] blocks/list (`user::blocks`)
- [x] blocks/ids (`user::blocks_ids`)
- [x] blocks/create (`user::block`)
- [x] blocks/destroy (`user::unblock`)
- [x] users/report\_spam (`user::report_spam`)
- [x] mutes/users/list (`user::mutes`)
- [x] mutes/users/ids (`user::mutes_ids`)
- [x] mutes/users/create (`user::mute`)
- [x] mutes/users/destroy (`user::unmute`)

- [x] friendships/show (`user::relation`)

- [x] friendships/lookup (`user::relation_lookup`)

- [ ] users/suggestions
- [ ] users/suggestions/:slug
- [ ] users/suggestions/:slug/members

- [ ] account/update\_profile
- [ ] account/update\_profile\_image

### Lists

- [ ] lists/list
- [ ] lists/show
- [ ] lists/statuses
- [ ] lists/memberships
- [ ] lists/subscriptions
- [ ] lists/ownerships
- [ ] lists/create
- [ ] lists/update
- [ ] lists/destroy
- [ ] lists/members
- [ ] lists/members/create
- [ ] lists/members/create\_all
- [ ] lists/members/destroy
- [ ] lists/members/destroy\_all
- [ ] lists/subscribers
- [ ] lists/subscribers/show
- [ ] lists/subscribers/create
- [ ] lists/subscribers/destroy

### Places

- [x] geo/search (`place::search_point`/`place::search_query`/`place::search_ip`/`place::search_url`)
- [x] geo/reverse\_geocode (`place::reverse_geocode`/`place::reverse_geocode_url`)
- [x] geo/id/:place\_id (`place::show`)

### Account Settings/Misc

- [ ] account/settings (GET)
- [ ] account/settings (POST)

- [ ] account/profile\_banner
- [ ] account/update\_profile\_banner
- [ ] account/remove\_profile\_banner

- [x] application/rate\_limit\_status (`service::rate_limit_status`)
- [ ] help/languages
- [x] help/configuration (`service::config`)
- [x] help/privacy (`service::privacy`)
- [x] help/tos (`service::terms`)

- [x] account/verify\_credentials (`verify_tokens`)

## [Media API](https://dev.twitter.com/rest/media)

- [ ] media/upload (Simple)
- [ ] media/upload (INIT)
- [ ] media/upload (APPEND)
- [ ] media/upload (FINALIZE)
- [ ] media/upload (STATUS)
- [ ] media/metadata/create

## [Collections API](https://dev.twitter.com/rest/collections)

- [ ] collections/list
- [ ] collections/show
- [ ] collections/entries
- [ ] collections/create
- [ ] collections/update
- [ ] collections/destroy
- [ ] collections/entries/add
- [ ] collections/entries/remove
- [ ] collections/entries/curate
- [ ] collections/entries/move

## [Streaming API](https://dev.twitter.com/streaming/overview)

Site Streams are apparently in a closed beta, and the public firehose is unavailable to the general
public, so I don't plan to implement them unless asked. They shouldn't be much different from the
other streams, though, so once I have these ones working, all I'd need is the request to implement
them.

- [ ] statuses/sample
- [ ] statuses/filter
- [ ] user

## [twitter-text](https://github.com/twitter/twitter-text)

From various excursions into the official twitter-text implementations, it seems that the JavaScript
version of twitter-text includes some helper functions to auto-format URLs and possibly other
HTML-focused helper functions. Upon request, I could include some or all of these helpers (if I had
some guidance about the requested methods `>_>`). The functions listed below are based on the
Objective-C implementation, which has a much more trimmed-down interface.

- [x] "character count" (`text::character_count`, `text::characters_remaining`)
- [x] "extract URL entities" (`text::url_entities`)
- [ ] "extract hashtag entities"
- [ ] "extract symbol ("cashtag") entities"
- [ ] "extract user mention entities"
- [x] "extract user and list mention entities" (`text::mention_list_entities`)
- [ ] "extract leading mention from reply"
- [ ] "extract all entities"
