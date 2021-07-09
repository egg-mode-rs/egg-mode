//! Functionality to alter a user's public profile.
//!
//! Specifically, this module contains functions which update the information
//! that is publically visible on a user's timeline (e.g. name, location). This module does *not*
//! modify a user's private account settings (e.g. email, password).

use crate::{
    auth,
    common::{post, request_with_empty_response, request_with_json_response, ParamList},
    error, links,
    user::TwitterUser,
    Response,
};

/// Options for updating the profile banner
#[derive(Debug, Default)]
pub struct ProfileBannerOption {
    /// The width of the preferred section of the image being uploaded in pixels.
    /// Use with height , offset_left , and offset_top to select the desired region of the image to use.
    pub width: Option<String>,
    /// The height of the preferred section of the image being uploaded in pixels.
    /// Use with width , offset_left , and offset_top to select the desired region of the image to use.
    pub height: Option<String>,
    /// The number of pixels by which to offset the uploaded image from the left.
    /// Use with height , width , and offset_top to select the desired region of the image to use.
    pub offset_left: Option<String>,
    /// The number of pixels by which to offset the uploaded image from the top.
    /// Use with height , width , and offset_left to select the desired region of the image to use.
    pub offset_top: Option<String>,
}

/// Options for updating the user profile
#[derive(Debug, Default)]
pub struct UserProfile {
    /// Full name associated with the profile.
    pub name: Option<String>,
    /// URL associated with the profile. Will be prepended with http:// if not present.
    pub url: Option<String>,
    /// The city or country describing where the user of the account is located. The contents are not normalized or geocoded in any way.
    pub location: Option<String>,
    /// A description of the user owning the account.
    pub description: Option<String>,
    /// Sets a hex value that controls the color scheme of links used on the authenticating user's profile page on twitter.com.
    /// This must be a valid hexadecimal value, and may be either three or six characters (ex: F00 or FF0000).
    /// This parameter replaces the deprecated (and separate) update_profile_colors API method.
    pub profile_link_color: Option<String>,
}

/// Updates the authenticating user's profile image.
///
/// This function takes the image as a slice of bytes. This slice must be a valid GIF, JPG or PNG image.
/// Note that this method expects raw multipart data, not a URL to an image.
///
/// This method asynchronously processes the uploaded file before updating the user's profile image URL.
/// You can either update your local cache the next time you request the user's information, or, at least 5 seconds after uploading the image, ask for the updated URL using GET users / show.
pub async fn update_profile_image(
    image: &[u8],
    token: &auth::Token,
) -> error::Result<Response<TwitterUser>> {
    let params = ParamList::new().add_param("image", base64::encode(image));
    let req = post(links::account::UPDATE_PROFILE_IMAGE, token, Some(&params));
    request_with_json_response(req).await
}

/// Uploads a profile banner on behalf of the authenticating user.
///
/// This function takes the banner as a slice of bytes. This slice must be a valid GIF, JPG or PNG image.
/// More information about sizing variations can be found in User Profile Images and Banners and GET users / profile_banner.
///
/// Profile banner images are processed asynchronously.
/// The profile_banner_url and its variant sizes will not necessary be available directly after upload.
pub async fn update_profile_banner(
    banner: &[u8],
    options: Option<ProfileBannerOption>,
    token: &auth::Token,
) -> error::Result<Response<()>> {
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

    request_with_empty_response(req).await
}

/// Sets some values that users are able to set under the "Account" tab of their settings page.
/// Only the parameters specified will be updated.
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
