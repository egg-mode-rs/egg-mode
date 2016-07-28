pub mod auth {
    pub const REQUEST_TOKEN: &'static str = "https://api.twitter.com/oauth/request_token";
    pub const ACCESS_TOKEN: &'static str = "https://api.twitter.com/oauth/access_token";
    pub const AUTHORIZE: &'static str = "https://api.twitter.com/oauth/authorize";
}

pub mod users {
    pub const LOOKUP: &'static str = "https://api.twitter.com/1.1/users/lookup.json";
}
