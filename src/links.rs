// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

pub mod auth {
    pub const REQUEST_TOKEN: &str = "https://api.twitter.com/oauth/request_token";
    pub const ACCESS_TOKEN: &str = "https://api.twitter.com/oauth/access_token";
    pub const BEARER_TOKEN: &str = "https://api.twitter.com/oauth2/token";
    pub const INVALIDATE_BEARER: &str = "https://api.twitter.com/oauth2/invalidate_token";
    pub const AUTHORIZE: &str = "https://api.twitter.com/oauth/authorize";
    pub const AUTHENTICATE: &str = "https://api.twitter.com/oauth/authenticate";
    pub const VERIFY_CREDENTIALS: &str =
        "https://api.twitter.com/1.1/account/verify_credentials.json";
}

pub mod users {
    pub const LOOKUP: &str = "https://api.twitter.com/1.1/users/lookup.json";
    pub const SHOW: &str = "https://api.twitter.com/1.1/users/show.json";
    pub const SEARCH: &str = "https://api.twitter.com/1.1/users/search.json";
    pub const FRIENDS_LIST: &str = "https://api.twitter.com/1.1/friends/list.json";
    pub const FRIENDS_IDS: &str = "https://api.twitter.com/1.1/friends/ids.json";
    pub const FOLLOWERS_LIST: &str = "https://api.twitter.com/1.1/followers/list.json";
    pub const FOLLOWERS_IDS: &str = "https://api.twitter.com/1.1/followers/ids.json";
    pub const BLOCKS_LIST: &str = "https://api.twitter.com/1.1/blocks/list.json";
    pub const BLOCKS_IDS: &str = "https://api.twitter.com/1.1/blocks/ids.json";
    pub const MUTES_LIST: &str = "https://api.twitter.com/1.1/mutes/users/list.json";
    pub const MUTES_IDS: &str = "https://api.twitter.com/1.1/mutes/users/ids.json";
    pub const FOLLOW: &str = "https://api.twitter.com/1.1/friendships/create.json";
    pub const UNFOLLOW: &str = "https://api.twitter.com/1.1/friendships/destroy.json";
    pub const FRIENDSHIPS_INCOMING: &str = "https://api.twitter.com/1.1/friendships/incoming.json";
    pub const FRIENDSHIPS_OUTGOING: &str = "https://api.twitter.com/1.1/friendships/outgoing.json";
    pub const FRIENDSHIP_SHOW: &str = "https://api.twitter.com/1.1/friendships/show.json";
    pub const FRIENDSHIP_UPDATE: &str = "https://api.twitter.com/1.1/friendships/update.json";
    pub const FRIENDS_NO_RETWEETS: &str =
        "https://api.twitter.com/1.1/friendships/no_retweets/ids.json";
    pub const FRIENDSHIP_LOOKUP: &str = "https://api.twitter.com/1.1/friendships/lookup.json";
    pub const BLOCK: &str = "https://api.twitter.com/1.1/blocks/create.json";
    pub const UNBLOCK: &str = "https://api.twitter.com/1.1/blocks/destroy.json";
    pub const REPORT_SPAM: &str = "https://api.twitter.com/1.1/users/report_spam.json";
    pub const MUTE: &str = "https://api.twitter.com/1.1/mutes/users/create.json";
    pub const UNMUTE: &str = "https://api.twitter.com/1.1/mutes/users/destroy.json";
}

pub mod statuses {
    pub const SHOW: &str = "https://api.twitter.com/1.1/statuses/show.json";
    pub const RETWEETS_OF_STEM: &str = "https://api.twitter.com/1.1/statuses/retweets";
    pub const LOOKUP: &str = "https://api.twitter.com/1.1/statuses/lookup.json";
    pub const HOME_TIMELINE: &str = "https://api.twitter.com/1.1/statuses/home_timeline.json";
    pub const MENTIONS_TIMELINE: &str =
        "https://api.twitter.com/1.1/statuses/mentions_timeline.json";
    pub const USER_TIMELINE: &str = "https://api.twitter.com/1.1/statuses/user_timeline.json";
    pub const RETWEETS_OF_ME: &str = "https://api.twitter.com/1.1/statuses/retweets_of_me.json";
    pub const RETWEETERS_OF: &str = "https://api.twitter.com/1.1/statuses/retweeters/ids.json";
    pub const LIKES_OF: &str = "https://api.twitter.com/1.1/favorites/list.json";
    pub const SEARCH: &str = "https://api.twitter.com/1.1/search/tweets.json";
    pub const RETWEET_STEM: &str = "https://api.twitter.com/1.1/statuses/retweet";
    pub const UNRETWEET_STEM: &str = "https://api.twitter.com/1.1/statuses/unretweet";
    pub const LIKE: &str = "https://api.twitter.com/1.1/favorites/create.json";
    pub const UNLIKE: &str = "https://api.twitter.com/1.1/favorites/destroy.json";
    pub const UPDATE: &str = "https://api.twitter.com/1.1/statuses/update.json";
    pub const DELETE_STEM: &str = "https://api.twitter.com/1.1/statuses/destroy";
}

pub mod media {
    pub const UPLOAD: &str = "https://upload.twitter.com/1.1/media/upload.json";
    pub const METADATA: &str = "https://upload.twitter.com/1.1/media/metadata/create.json";
}

pub mod lists {
    pub const STATUSES: &str = "https://api.twitter.com/1.1/lists/statuses.json";
    pub const MEMBERS: &str = "https://api.twitter.com/1.1/lists/members.json";
    pub const IS_MEMBER: &str = "https://api.twitter.com/1.1/lists/members/show.json";
    pub const LIST: &str = "https://api.twitter.com/1.1/lists/list.json";
    pub const MEMBERSHIPS: &str = "https://api.twitter.com/1.1/lists/memberships.json";
    pub const OWNERSHIPS: &str = "https://api.twitter.com/1.1/lists/ownerships.json";
    pub const SHOW: &str = "https://api.twitter.com/1.1/lists/show.json";
    pub const SUBSCRIBERS: &str = "https://api.twitter.com/1.1/lists/subscribers.json";
    pub const IS_SUBSCRIBER: &str = "https://api.twitter.com/1.1/lists/subscribers/show.json";
    pub const SUBSCRIPTIONS: &str = "https://api.twitter.com/1.1/lists/subscriptions.json";
    pub const ADD: &str = "https://api.twitter.com/1.1/lists/members/create.json";
    pub const REMOVE_MEMBER: &str = "https://api.twitter.com/1.1/lists/members/destroy.json";
    pub const CREATE: &str = "https://api.twitter.com/1.1/lists/create.json";
    pub const DELETE: &str = "https://api.twitter.com/1.1/lists/destroy.json";
    pub const SUBSCRIBE: &str = "https://api.twitter.com/1.1/lists/subscribers/create.json";
    pub const UNSUBSCRIBE: &str = "https://api.twitter.com/1.1/lists/subscribers/destroy.json";
    pub const ADD_LIST: &str = "https://api.twitter.com/1.1/lists/members/create_all.json";
    pub const REMOVE_LIST: &str = "https://api.twitter.com/1.1/lists/members/destroy_all.json";
    pub const UPDATE: &str = "https://api.twitter.com/1.1/lists/update.json";
}

pub mod account {
    pub const UPDATE_PROFILE_IMAGE: &str =
        "https://api.twitter.com/1.1/account/update_profile_image.json";
    pub const UPDATE_PROFILE_BNNER: &str =
        "https://api.twitter.com/1.1/account/update_profile_banner.json";
    pub const UPDATE_PROFILE: &str = "https://api.twitter.com/1.1/account/update_profile.json";
}

pub mod place {
    pub const SHOW_STEM: &str = "https://api.twitter.com/1.1/geo/id";
    pub const REVERSE_GEOCODE: &str = "https://api.twitter.com/1.1/geo/reverse_geocode.json";
    pub const SEARCH: &str = "https://api.twitter.com/1.1/geo/search.json";
}

pub mod direct {
    pub const SHOW: &str = "https://api.twitter.com/1.1/direct_messages/events/show.json";
    pub const LIST: &str = "https://api.twitter.com/1.1/direct_messages/events/list.json";
    pub const SEND: &str = "https://api.twitter.com/1.1/direct_messages/events/new.json";
    pub const DELETE: &str = "https://api.twitter.com/1.1/direct_messages/events/destroy.json";
    pub const MARK_READ: &str = "https://api.twitter.com/1.1/direct_messages/mark_read.json";
    pub const INDICATE_TYPING: &str =
        "https://api.twitter.com/1.1/direct_messages/indicate_typing.json";
}

pub mod service {
    pub const TERMS: &str = "https://api.twitter.com/1.1/help/tos.json";
    pub const PRIVACY: &str = "https://api.twitter.com/1.1/help/privacy.json";
    pub const CONFIG: &str = "https://api.twitter.com/1.1/help/configuration.json";
    pub const RATE_LIMIT_STATUS: &str =
        "https://api.twitter.com/1.1/application/rate_limit_status.json";
}

pub mod stream {
    pub const SAMPLE: &str = "https://stream.twitter.com/1.1/statuses/sample.json";
    pub const FILTER: &str = "https://stream.twitter.com/1.1/statuses/filter.json";
}

pub mod trend {
    pub const CLOSEST: &str = "https://api.twitter.com/1.1/trends/closest.json";
    pub const AVAILABLE: &str = "https://api.twitter.com/1.1/trends/available.json";
}
