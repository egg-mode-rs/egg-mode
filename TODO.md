# The Big List Of API Endpoints

This can be considered a big list of Twitter's API knobs that have and have not been given
corresponding methods in egg-mode. Those that have been implemented will have the method(s) written
alongside them.

I've grouped them by API heading, then by a rough category and data structure. Endpoints that just
return a list of IDs of something are grouped in with the rest of the main structure's list.

## [OAuth](https://dev.twitter.com/oauth/overview)

- [x] oauth/request\_token (`request_token`)
- [x] oauth/authenticate (`authenticate_url`)
- [x] oauth/authorize (`authorize_url`)
- [x] oauth/access\_token (`access_token`)
- [ ] oauth2/token
- [ ] oauth2/invalidate\_token

## [Public API](https://dev.twitter.com/rest/public)

### Statuses

- [ ] statuses/mentions\_timeline
- [ ] statuses/user\_timeline
- [ ] statuses/home\_timeline
- [ ] statuses/retweets\_of\_me
- [ ] statuses/retweets/:id
- [ ] statuses/show/:id
- [ ] statuses/destroy/:id
- [ ] statuses/update
- [ ] statuses/retweet/:id
- [ ] statuses/unretweet/:id
- (statuses/update\_with\_media is deprecated)
- [ ] statuses/retweeters/ids
- [ ] statuses/lookup
- [ ] search/tweets
- [ ] favorites/list
- [ ] favorites/create
- [ ] favorites/destroy

- [ ] statuses/oembed

- [ ] saved\_searches/list
- [ ] saved\_searches/show/:id
- [ ] saved\_searches/create
- [ ] saved\_searches/destroy/:id

- [ ] trends/place
- [ ] trends/available
- [ ] trends/closest

### Direct Messages

From what I can tell, DMs are like Tweets, but with a few extra fields? The docs don't go into much
detail other than the response examples for those endpoints.

- [ ] direct\_messages
- [ ] direct\_messages/sent
- [ ] direct\_messages/show
- [ ] direct\_messages/new
- [ ] direct\_messages/destroy

### Users

- [x] users/show (`users::show`)
- [x] users/lookup (`users::lookup`, `users::lookup_ids`, `users::lookup_names`)
- [x] users/search (`users::search`)
- [x] friends/list (`users::friends_of`)
- [x] friends/ids (`users::friends_ids`)
- [ ] friendships/create
- [ ] friendships/update
- [ ] friendships/destroy
- [ ] friendships/incoming
- [ ] friendships/outgoing
- [ ] friendships/no\_retweets/ids
- [x] followers/list (`users::followers_of`)
- [x] followers/ids (`users::followers_ids`)
- [x] blocks/list (`users::blocks`)
- [x] blocks/ids (`users::blocks_ids`)
- [ ] blocks/create
- [ ] blocks/destroy
- [ ] users/report\_spam
- [x] mutes/users/list (`users::mutes`)
- [x] mutes/users/ids (`users::mutes_ids`)
- [ ] mutes/users/create
- [ ] mutes/users/destroy

- [ ] friendships/show

- [ ] friendships/lookup

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

- [ ] geo/search
- [ ] geo/reverse\_geocode
- [ ] geo/id/:place\_id

### Account Settings/Misc

- [ ] account/settings (GET)
- [ ] account/settings (POST)

- [ ] account/profile\_banner
- [ ] account/update\_profile\_banner
- [ ] account/remove\_profile\_banner

- [ ] application/rate\_limit\_status
- [ ] help/languages
- [ ] help/configuration
- [ ] help/privacy
- [ ] help/tos

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
