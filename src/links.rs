// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

pub mod auth {
    pub const REQUEST_TOKEN: &'static str = "https://api.twitter.com/oauth/request_token";
    pub const ACCESS_TOKEN: &'static str = "https://api.twitter.com/oauth/access_token";
    pub const BEARER_TOKEN: &'static str = "https://api.twitter.com/oauth2/token";
    pub const INVALIDATE_BEARER: &'static str = "https://api.twitter.com/oauth2/invalidate_token";
    pub const AUTHORIZE: &'static str = "https://api.twitter.com/oauth/authorize";
    pub const AUTHENTICATE: &'static str = "https://api.twitter.com/oauth/authenticate";
    pub const VERIFY_CREDENTIALS: &'static str =
        "https://api.twitter.com/1.1/account/verify_credentials.json";
}

pub mod users {
    pub const LOOKUP: &'static str = "https://api.twitter.com/1.1/users/lookup.json";
    pub const SHOW: &'static str = "https://api.twitter.com/1.1/users/show.json";
    pub const SEARCH: &'static str = "https://api.twitter.com/1.1/users/search.json";
    pub const FRIENDS_LIST: &'static str = "https://api.twitter.com/1.1/friends/list.json";
    pub const FRIENDS_IDS: &'static str = "https://api.twitter.com/1.1/friends/ids.json";
    pub const FOLLOWERS_LIST: &'static str = "https://api.twitter.com/1.1/followers/list.json";
    pub const FOLLOWERS_IDS: &'static str = "https://api.twitter.com/1.1/followers/ids.json";
    pub const BLOCKS_LIST: &'static str = "https://api.twitter.com/1.1/blocks/list.json";
    pub const BLOCKS_IDS: &'static str = "https://api.twitter.com/1.1/blocks/ids.json";
    pub const MUTES_LIST: &'static str = "https://api.twitter.com/1.1/mutes/users/list.json";
    pub const MUTES_IDS: &'static str = "https://api.twitter.com/1.1/mutes/users/ids.json";
    pub const FOLLOW: &'static str = "https://api.twitter.com/1.1/friendships/create.json";
    pub const UNFOLLOW: &'static str = "https://api.twitter.com/1.1/friendships/destroy.json";
    pub const FRIENDSHIPS_INCOMING: &'static str =
        "https://api.twitter.com/1.1/friendships/incoming.json";
    pub const FRIENDSHIPS_OUTGOING: &'static str =
        "https://api.twitter.com/1.1/friendships/outgoing.json";
    pub const FRIENDSHIP_SHOW: &'static str = "https://api.twitter.com/1.1/friendships/show.json";
    pub const FRIENDSHIP_UPDATE: &'static str =
        "https://api.twitter.com/1.1/friendships/update.json";
    pub const FRIENDS_NO_RETWEETS: &'static str =
        "https://api.twitter.com/1.1/friendships/no_retweets/ids.json";
    pub const FRIENDSHIP_LOOKUP: &'static str =
        "https://api.twitter.com/1.1/friendships/lookup.json";
    pub const BLOCK: &'static str = "https://api.twitter.com/1.1/blocks/create.json";
    pub const UNBLOCK: &'static str = "https://api.twitter.com/1.1/blocks/destroy.json";
    pub const REPORT_SPAM: &'static str = "https://api.twitter.com/1.1/users/report_spam.json";
    pub const MUTE: &'static str = "https://api.twitter.com/1.1/mutes/users/create.json";
    pub const UNMUTE: &'static str = "https://api.twitter.com/1.1/mutes/users/destroy.json";
}

pub mod statuses {
    pub const SHOW: &'static str = "https://api.twitter.com/1.1/statuses/show.json";
    pub const RETWEETS_OF_STEM: &'static str = "https://api.twitter.com/1.1/statuses/retweets";
    pub const LOOKUP: &'static str = "https://api.twitter.com/1.1/statuses/lookup.json";
    pub const HOME_TIMELINE: &'static str =
        "https://api.twitter.com/1.1/statuses/home_timeline.json";
    pub const MENTIONS_TIMELINE: &'static str =
        "https://api.twitter.com/1.1/statuses/mentions_timeline.json";
    pub const USER_TIMELINE: &'static str =
        "https://api.twitter.com/1.1/statuses/user_timeline.json";
    pub const RETWEETS_OF_ME: &'static str =
        "https://api.twitter.com/1.1/statuses/retweets_of_me.json";
    pub const RETWEETERS_OF: &'static str =
        "https://api.twitter.com/1.1/statuses/retweeters/ids.json";
    pub const LIKES_OF: &'static str = "https://api.twitter.com/1.1/favorites/list.json";
    pub const SEARCH: &'static str = "https://api.twitter.com/1.1/search/tweets.json";
    pub const RETWEET_STEM: &'static str = "https://api.twitter.com/1.1/statuses/retweet";
    pub const UNRETWEET_STEM: &'static str = "https://api.twitter.com/1.1/statuses/unretweet";
    pub const LIKE: &'static str = "https://api.twitter.com/1.1/favorites/create.json";
    pub const UNLIKE: &'static str = "https://api.twitter.com/1.1/favorites/destroy.json";
    pub const UPDATE: &'static str = "https://api.twitter.com/1.1/statuses/update.json";
    pub const DELETE_STEM: &'static str = "https://api.twitter.com/1.1/statuses/destroy";
}

pub mod media {
    pub const UPLOAD: &'static str = "https://upload.twitter.com/1.1/media/upload.json";
    pub const METADATA: &'static str = "https://upload.twitter.com/1.1/media/metadata/create.json";
}

pub mod lists {
    pub const STATUSES: &'static str = "https://api.twitter.com/1.1/lists/statuses.json";
    pub const MEMBERS: &'static str = "https://api.twitter.com/1.1/lists/members.json";
    pub const IS_MEMBER: &'static str = "https://api.twitter.com/1.1/lists/members/show.json";
    pub const LIST: &'static str = "https://api.twitter.com/1.1/lists/list.json";
    pub const MEMBERSHIPS: &'static str = "https://api.twitter.com/1.1/lists/memberships.json";
    pub const OWNERSHIPS: &'static str = "https://api.twitter.com/1.1/lists/ownerships.json";
    pub const SHOW: &'static str = "https://api.twitter.com/1.1/lists/show.json";
    pub const SUBSCRIBERS: &'static str = "https://api.twitter.com/1.1/lists/subscribers.json";
    pub const IS_SUBSCRIBER: &'static str =
        "https://api.twitter.com/1.1/lists/subscribers/show.json";
    pub const SUBSCRIPTIONS: &'static str = "https://api.twitter.com/1.1/lists/subscriptions.json";
    pub const ADD: &'static str = "https://api.twitter.com/1.1/lists/members/create.json";
    pub const REMOVE_MEMBER: &'static str =
        "https://api.twitter.com/1.1/lists/members/destroy.json";
    pub const CREATE: &'static str = "https://api.twitter.com/1.1/lists/create.json";
    pub const DELETE: &'static str = "https://api.twitter.com/1.1/lists/destroy.json";
    pub const SUBSCRIBE: &'static str = "https://api.twitter.com/1.1/lists/subscribers/create.json";
    pub const UNSUBSCRIBE: &'static str =
        "https://api.twitter.com/1.1/lists/subscribers/destroy.json";
    pub const ADD_LIST: &'static str = "https://api.twitter.com/1.1/lists/members/create_all.json";
    pub const REMOVE_LIST: &'static str =
        "https://api.twitter.com/1.1/lists/members/destroy_all.json";
    pub const UPDATE: &'static str = "https://api.twitter.com/1.1/lists/update.json";
}

pub mod place {
    pub const SHOW_STEM: &'static str = "https://api.twitter.com/1.1/geo/id";
    pub const REVERSE_GEOCODE: &'static str =
        "https://api.twitter.com/1.1/geo/reverse_geocode.json";
    pub const SEARCH: &'static str = "https://api.twitter.com/1.1/geo/search.json";
}

pub mod direct {
    pub const SHOW: &'static str = "https://api.twitter.com/1.1/direct_messages/events/show.json";
    pub const LIST: &'static str = "https://api.twitter.com/1.1/direct_messages/events/list.json";
    pub const SEND: &'static str = "https://api.twitter.com/1.1/direct_messages/events/new.json";
    pub const DELETE: &'static str = "https://api.twitter.com/1.1/direct_messages/events/destroy.json";
    pub const MARK_READ: &'static str = "https://api.twitter.com/1.1/direct_messages/mark_read.json";
    pub const INDICATE_TYPING: &'static str = "https://api.twitter.com/1.1/direct_messages/indicate_typing.json";
}

pub mod service {
    pub const TERMS: &'static str = "https://api.twitter.com/1.1/help/tos.json";
    pub const PRIVACY: &'static str = "https://api.twitter.com/1.1/help/privacy.json";
    pub const CONFIG: &'static str = "https://api.twitter.com/1.1/help/configuration.json";
    pub const RATE_LIMIT_STATUS: &'static str =
        "https://api.twitter.com/1.1/application/rate_limit_status.json";
}

pub mod stream {
    pub const SAMPLE: &'static str = "https://stream.twitter.com/1.1/statuses/sample.json";
    pub const FILTER: &'static str = "https://stream.twitter.com/1.1/statuses/filter.json";
}
