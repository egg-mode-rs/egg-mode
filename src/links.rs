pub mod auth {
    pub const REQUEST_TOKEN: &'static str = "https://api.twitter.com/oauth/request_token";
    pub const ACCESS_TOKEN: &'static str = "https://api.twitter.com/oauth/access_token";
    pub const AUTHORIZE: &'static str = "https://api.twitter.com/oauth/authorize";
    pub const AUTHENTICATE: &'static str = "https://api.twitter.com/oauth/authenticate";
    pub const VERIFY_CREDENTIALS: &'static str = "https://api.twitter.com/1.1/account/verify_credentials.json";
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
    pub const FRIENDSHIPS_INCOMING: &'static str = "https://api.twitter.com/1.1/friendships/incoming.json";
    pub const FRIENDSHIPS_OUTGOING: &'static str = "https://api.twitter.com/1.1/friendships/outgoing.json";
    pub const FRIENDSHIP_SHOW: &'static str = "https://api.twitter.com/1.1/friendships/show.json";
    pub const FRIENDSHIP_UPDATE: &'static str = "https://api.twitter.com/1.1/friendships/update.json";
    pub const FRIENDS_NO_RETWEETS: &'static str = "https://api.twitter.com/1.1/friendships/no_retweets/ids.json";
    pub const FRIENDSHIP_LOOKUP: &'static str = "https://api.twitter.com/1.1/friendships/lookup.json";
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
    pub const HOME_TIMELINE: &'static str = "https://api.twitter.com/1.1/statuses/home_timeline.json";
    pub const MENTIONS_TIMELINE: &'static str = "https://api.twitter.com/1.1/statuses/mentions_timeline.json";
    pub const USER_TIMELINE: &'static str = "https://api.twitter.com/1.1/statuses/user_timeline.json";
    pub const RETWEETS_OF_ME: &'static str = "https://api.twitter.com/1.1/statuses/retweets_of_me.json";
    pub const RETWEETERS_OF: &'static str = "https://api.twitter.com/1.1/statuses/retweeters/ids.json";
    pub const LIKES_OF: &'static str = "https://api.twitter.com/1.1/favorites/list.json";
    pub const SEARCH: &'static str = "https://api.twitter.com/1.1/search/tweets.json";
    pub const RETWEET_STEM: &'static str = "https://api.twitter.com/1.1/statuses/retweet";
    pub const UNRETWEET_STEM: &'static str = "https://api.twitter.com/1.1/statuses/unretweet";
    pub const LIKE: &'static str = "https://api.twitter.com/1.1/favorites/create.json";
    pub const UNLIKE: &'static str = "https://api.twitter.com/1.1/favorites/destroy.json";
    pub const UPDATE: &'static str = "https://api.twitter.com/1.1/statuses/update.json";
    pub const DELETE_STEM: &'static str = "https://api.twitter.com/1.1/statuses/destroy";
}

pub mod place {
    pub const SHOW_STEM: &'static str = "https://api.twitter.com/1.1/geo/id";
    pub const REVERSE_GEOCODE: &'static str = "https://api.twitter.com/1.1/geo/reverse_geocode.json";
    pub const SEARCH: &'static str = "https://api.twitter.com/1.1/geo/search.json";
}

pub mod direct {
    pub const SHOW: &'static str = "https://api.twitter.com/1.1/direct_messages/show.json";
    pub const RECEIVED: &'static str = "https://api.twitter.com/1.1/direct_messages.json";
    pub const SENT: &'static str = "https://api.twitter.com/1.1/direct_messages/sent.json";
    pub const SEND: &'static str = "https://api.twitter.com/1.1/direct_messages/new.json";
    pub const DELETE: &'static str = "https://api.twitter.com/1.1/direct_messages/destroy.json";
}

pub mod service {
    pub const TERMS: &'static str = "https://api.twitter.com/1.1/help/tos.json";
    pub const PRIVACY: &'static str = "https://api.twitter.com/1.1/help/privacy.json";
    pub const CONFIG: &'static str = "https://api.twitter.com/1.1/help/configuration.json";
}
