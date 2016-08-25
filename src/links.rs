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
}
