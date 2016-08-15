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
}
