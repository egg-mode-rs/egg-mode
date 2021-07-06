// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

//! Raw access to request- and response-building primitives used internally by egg-mode.
//!
//! The functions and types exposed in this module allow you to access Twitter API functions that
//! aren't currently wrapped by egg-mode, or to provide parameters to Twitter that egg-mode doesn't
//! currently use. These functions also allow you to have more power in how you process the data
//! returned by Twitter. In return, much more knowledge of the Twitter API is required to
//! effectively use these functions.
//!
//! The functions in this module can be divided into two categories: assembling a request, and
//! executing it to get a response. Some wrapper types in egg-mode operate directly on requests, or
//! create their own, so constructors for those types are also exposed here.
//!
//! To start using the functions in this module, you'll need a [`Token`] from the authentication
//! flow. The parameters to an endpoint are represented by the [`ParamList`] type. They're
//! separated out so that they can be included as part of the OAuth signature given to Twitter as
//! part of the API call. This also means that the URL you give to the request functions should be
//! the base URL, with no parameters.
//!
//! [`Token`]: ../auth/enum.Token.html
//! [`ParamList`]: struct.ParamList.html
//!
//! There are three basic request functions, based on how the endpoint expects to be called:
//!
//! * `request_get` assembles a GET request, with the given parameters appended to the URL as a
//!   query string. All GET endpoints that egg-mode currently wraps use this function to encode and
//!   sign the request.
//! * `request_post` assembles a POST request, with the given parameters included in the POST body
//!   formatted as `x-www-form-urlencoded` data. Most POST endpoints in the Twitter API are
//!   formatted using this function.
//! * `request_post_json` also assembles a POST request, but instead of taking a `ParamList`, it
//!   takes arbitrary data and formats it in the POST body as JSON. The provided data is *not* used
//!   as part of the OAuth signature. At time of writing (between releases 0.14 and 0.15) the only
//!   egg-mode endpoint that uses this function is [`media::set_metadata`].
//!
//! [`media::set_metadata`]: ../media/fn.set_metadata.html
//!
//! Once you have a `Request`, you can hand it to the `response_*` functions in this module to
//! process it. Which one you select depends on how much processing you want egg-mode to do with
//! the response:
//!
//! * At the most hands-off end, there's [`response_future`,] which is a small wrapper that just
//!   starts the request and hands off the `ResponseFuture` from `hyper` to give you the most power
//!   over handling the response data.
//! * In the middle, there's [`response_raw_bytes`], which wraps the `ResponseFuture` to return the
//!   headers and response body after inspecting the rate-limit headers and response code, and
//!   after inspecting the response to see whether it returned error data from Twitter.
//! * Finally there's [`response_json`], which picks up from `response_raw_bytes` to parse the
//!   response as JSON and deserialize it into the target type, alongside the rate-limit
//!   information from the response headers.
//!
//! [`response_future`]: fn.response_future.html
//! [`response_raw_bytes`]: fn.response_raw_bytes.html
//! [`response_json`]: fn.response_json.html
//!
//! In addition, there are `request_as_*` and `response_as_*` functions available to format a
//! request using one of the wrappers used in egg-mode. If the endpoint you're using is one that
//! currently uses one of these wrapper types or returns and accepts data the same way as one of
//! these endpoints, you can use these functions to get the same experience as the existing
//! wrappers in egg-mode. See the documentation for these functions to see their assumptions and
//! requirements.
//!
//! If you need the ability to assemble a request in a way that `request_get`, `request_post`, or
//! `request_post_json` don't allow, the `RequestBuilder` type available in the `auth` submodule
//! provides the lowest-level control over how a request is built and signed. For more information,
//! see the [`auth`] module.
//!
//! [`auth`]: auth/index.html

use hyper::{Body, Request};

use crate::auth::Token;
use crate::cursor;
use crate::stream::TwitterStream;

use crate::tweet::Timeline as TweetTimeline;

pub use crate::common::Headers;
pub use crate::common::ParamList;

pub use crate::auth::raw::delete as request_delete;
pub use crate::auth::raw::get as request_get;
pub use crate::auth::raw::post as request_post;
pub use crate::auth::raw::post_json as request_post_json;

/// Assemble a GET request and convert it to a `Timeline` of tweets.
///
/// An endpoint wrapped by `tweet::Timeline` returns data as an array of Tweets. In addition, they
/// also take parameters `since_id` and `max_id` to filter the earliest and latest Tweet returned
/// (respectively), as well as a `count` parameter to limit the number of Tweets returned at once.
/// The `Timeline` struct sets these parameters itself; you should not need to hand them to this
/// function. These parameters are manipulated through the `older()` and `newer()` functions, as
/// well as the `with_page_size()` function.
///
/// In addition, the `Timeline` struct also adds `tweet_mode=extended` and
/// `include_ext_alt_text=true` when sending a request, to fill in the data from extended Tweets
/// and media alt-text when returned from Twitter.
///
/// If you do not need to send additional parameters other than these mentioned, you can pass
/// `None` for the `params` to make the `Timeline` manage the parameters itself.
pub fn request_as_tweet_timeline(
    url: &'static str,
    token: &Token,
    params: Option<ParamList>,
) -> TweetTimeline {
    TweetTimeline::new(url, params, token)
}

/// Assemble a GET request and convert it to a `CursorIter`.
///
/// A `CursorIter` is a wrapper around an endpoint that returns data in the following structure:
///
/// ```json
/// {
///   "previous_cursor": int,
///   "previous_cursor_str": "string",
///   "next_cursor": int,
///   "next_cursor_str": "string",
///   "<data>": [ ... ]
/// }
/// ```
///
/// Where `<data>` is named something relevant to the endpoint, and contains an array of objects.
/// `CursorIter` expects to be able to deserialize this response into a type that implements the
/// [`Cursor`] trait to expose these fields. (The cursor struct is the type parameter of
/// `CursorIter` itself.) It uses these fields to set the `cursor` parameter to the given endpoint.
/// It also sets the `count` parameter with the given `page_size`, if present. (Some cursor
/// endpoints do not support setting a page size; an example of such an endpoint is `GET
/// friendships/incoming`.)
///
/// [`Cursor`]: ../cursor/trait.Cursor.html
///
/// An example of a Twitter API endpoint that exposes a cursor interface is [`GET friends/list`].
///
/// [`GET friends/list`]: https://developer.twitter.com/en/docs/accounts-and-users/follow-search-get-users/api-reference/get-friends-list
///
/// If you can supply a Cursor type (or use one of the ones in the `cursor` module), `CursorIter`
/// will wrap the responses into a `Stream` interface that automatically fetches the next page of
/// results on-demand.
pub fn request_as_cursor_iter<T: cursor::Cursor + serde::de::DeserializeOwned>(
    url: &'static str,
    token: &Token,
    params: Option<ParamList>,
    page_size: Option<i32>,
) -> cursor::CursorIter<T> {
    cursor::CursorIter::new(url, token, params, page_size)
}

pub use crate::common::get_response as response_future;
pub use crate::common::raw_request as response_raw_bytes;
pub use crate::common::request_with_empty_response as response_empty;
pub use crate::common::request_with_json_response as response_json;

/// Converts the given request into a `TwitterStream`.
///
/// This function can be used for endpoints that open a persistent stream, like `GET
/// statuses/sample`. If you know that the messages returned by the stream you're using will look
/// the same as `StreamMessage`, this can be a convenient way to customize a stream if you need to
/// use other endpoints or options not available to `StreamBuilder`.
///
/// Since the `TwitterStream` type doesn't need to provide additional parameters to the request, it
/// can take a signed, completed request as its constructor.
pub fn response_as_stream(req: Request<Body>) -> TwitterStream {
    TwitterStream::new(req)
}

pub use crate::common::RoundTrip;

/// Facilities to manually assemble signed requests.
///
/// In case you need to do things that aren't available in the `raw` module, the `RequestBuilder`
/// included here allows you to go deeper into the internals of egg-mode. All of the authentication
/// internals are implemented in terms of `RequestBuilder`, meaning you can fully recreate them
/// using your own parsing logic for the output.
///
/// `RequestBuilder` is designed to allow for easily creating an OAuth signature from the
/// parameters to an API endpoint, and so they collect `ParamList` instances just like the
/// functions in the `raw` module. However, there is also a way to manually set the request body
/// outside of the `ParamList` struct, for endpoints like `POST media/metadata/create` or `POST
/// oauth2/token` which require specific body formats.
///
/// True to its name, all the methods on `RequestBuilder` are meant to be used in a builder
/// pattern. To begin, you need to have the URL you wish to access and the HTTP Method you would
/// like to use. Then you can build up the query string and request body, and accumulate the
/// parameters used in the OAuth signature. Finally, to finish building the request, you need to
/// provide what kind of authorization you would like to use. Since there are several ways to
/// authorize a call to Twitter, there are several options available:
///
/// * For [OAuth 1.0a], you can specify the keys individually in `request_keys`, or provide a
///   complete `Token` using `request_token`.
/// * For [OAuth 2.0 Bearer Token][bearer], you can provide the Bearer token using `request_token`.
/// * For [Basic authentication][basic] used with Enterprise APIs and when requesting a Bearer
///   token, you can provide the credentials as a `KeyPair` in `request_consumer_bearer`.
///
/// [OAuth 1.0a]: https://developer.twitter.com/en/docs/basics/authentication/oauth-1-0a
/// [bearer]: https://developer.twitter.com/en/docs/basics/authentication/oauth-2-0
/// [basic]: https://developer.twitter.com/en/docs/basics/authentication/basic-auth
///
/// For example, if you were using this type to request a specific Tweet:
///
/// ```rust,no_run
/// use egg_mode::raw::auth::{RequestBuilder, Method};
/// use egg_mode::raw::{ParamList, response_json};
/// use egg_mode::Response;
///
/// # #[tokio::main]
/// # async fn main() {
/// # let token: egg_mode::Token = unimplemented!();
/// let params = ParamList::new()
///     .extended_tweets()
///     .add_param("id", 1261253754969640960u64.to_string());
/// let request = RequestBuilder::new(Method::GET, "https://api.twitter.com/1.1/statuses/show.json")
///     .with_query_params(&params)
///     .request_token(&token);
/// let json: Response<serde_json::Value> = response_json(request).await.unwrap();
/// # }
/// ```
///
/// For more information, see the functions available on `RequestBuilder`.
pub mod auth {
    pub use crate::auth::raw::RequestBuilder;

    #[doc(no_inline)]
    pub use hyper::Method;
}

/// Types that can be used for deserialization from the raw API.
///
/// In cases where the types in egg-mode do not directly implement `Deserialize`, types are
/// available here that represent the data sent "across the wire", which can be converted into
/// regular egg-mode types. See the individual module docs for details.
pub mod types {
    pub mod direct;
}
