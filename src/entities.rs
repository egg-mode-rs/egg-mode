pub struct HashtagEntity {
    pub indices: (i32, i32),
    pub text: String,
}

pub struct MediaEntity {
    pub display_url: String,
    pub expanded_url: String,
    pub id: i64,
    pub id_str: String,
    pub indices: (i32, i32),
    pub media_url: String,
    pub media_url_https: String,
    pub sizes: MediaSizes,
    pub source_status_id: i64,
    pub source_status_id_str: String,
    pub media_type: String, //TODO: encoded as "type"
    pub url: String,
}

pub struct MediaSizes {
    pub thumb: MediaSize,
    pub small: MediaSize,
    pub medium: MediaSize,
    pub large: MediaSize,
}

pub struct MediaSize {
    pub w: i32,
    pub h: i32,
    pub resize: String,
}

pub struct UrlEntity {
    pub display_url: String,
    pub expanded_url: String,
    pub indices: (i32, i32),
    pub url: String,
}

pub struct MentionEntity {
    pub id: i64,
    pub id_str: String,
    pub indices: (i32, i32),
    pub name: String,
    pub screen_name: String,
}

pub struct Entites {
    pub hashtags: Vec<HashtagEntity>,
    pub media: Vec<MediaEntity>,
    pub urls: Vec<UrlEntity>,
    pub user_mentions: Vec<MentionEntity>,
}
