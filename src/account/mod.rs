use crate::common::ParamList;

use crate::common::*;
use crate::{auth, error, links, user::TwitterUser};

use serde::{Deserialize, Serialize};

/// TODO
pub struct ProfileBannerOption {
    pub width: Option<String>,
    pub height: Option<String>,
    pub offset_left: Option<String>,
    pub offset_top: Option<String>,
}

/// TODO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileBanner {
    sizes: Sizes,
}

/// TODO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sizes {
    ipad: ImageSize,
    ipad_retina: ImageSize,
    web: ImageSize,
    web_retina: ImageSize,
    mobile: ImageSize,
    mobile_retina: ImageSize,
    #[serde(rename = "300x100")]
    s300x100: ImageSize,
    #[serde(rename = "600x200")]
    s600x200: ImageSize,
    #[serde(rename = "1500x500")]
    s1500x500: ImageSize,
}

/// TODO
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageSize {
    pub h: String,
    pub w: String,
    pub url: String,
}

/// TODO
pub struct UserProfile {
    pub name: Option<String>,
    pub url: Option<String>,
    pub location: Option<String>,
    pub description: Option<String>,
    pub profile_link_color: Option<String>,
}

/// TODO
pub async fn update_profile_image(
    image: &[u8],
    token: &auth::Token,
) -> error::Result<Response<TwitterUser>> {
    let params = ParamList::new().add_param("image", base64::encode(image));
    let req = post(links::account::UPDATE_PROFILE_IMAGE, token, Some(&params));
    request_with_json_response(req).await
}

/// TODO
pub async fn update_profile_banner(
    banner: &[u8],
    options: Option<ProfileBannerOption>,
    token: &auth::Token,
) -> error::Result<Response<TwitterUser>> {
    let params = match options {
        Some(o) => ParamList::new()
            .add_param("banner", base64::encode(banner))
            .add_opt_param("width", o.width)
            .add_opt_param("height", o.height)
            .add_opt_param("offset_top", o.offset_top)
            .add_opt_param("offset_left", o.offset_left),
        None => ParamList::new().add_param("banner", base64::encode(banner)),
    };

    let req = post(links::account::UPDATE_PROFILE_BNNER, token, Some(&params));

    request_with_json_response(req).await
}

/// TODO
pub async fn update_profile(
    user_profile: UserProfile,
    token: &auth::Token,
) -> error::Result<Response<TwitterUser>> {
    let params = ParamList::new()
        .add_opt_param("name", user_profile.name)
        .add_opt_param("url", user_profile.url)
        .add_opt_param("location", user_profile.location)
        .add_opt_param("description", user_profile.description)
        .add_opt_param("profile_link_color", user_profile.profile_link_color);

    let req = post(links::account::UPDATE_PROFILE, token, Some(&params));

    request_with_json_response(req).await
}
