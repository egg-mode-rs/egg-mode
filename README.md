# abandoned

![status: abandoned](https://img.shields.io/badge/status-abandoned-red)

**NOTE**: This crate is abandoned. See https://github.com/egg-mode-rs/egg-mode/issues/132 for
details.

# egg-mode

Twitter library for Rust ![Build Status](https://github.com/egg-mode-rs/egg-mode/workflows/CI/badge.svg)

[Documentation](https://docs.rs/egg-mode/)

This is a library for interacting with Twitter from Rust. You can see how much of the Public API is
available in the file [TODO.md]. In addition to eventually implementing the entire Public API, an
explicit goal for egg-mode is to make it as easy as possible for a client of this library to
interact with the Twitter API. Parts of this library are added as a convenience on top of the API
mechanisms; for example, cursored lists of users and tweets can be used as an iterator in addition
to being able to manually load a page at a time.


[TODO.md]: https://github.com/egg-mode-rs/egg-mode/blob/master/TODO.md

**NOTE**: Previous versions of egg-mode contained a port of twitter-text to use for character
counting and mention/hashtag/url extraction. That has since been extracted into its own crate,
[egg-mode-text].

[egg-mode-text]: https://github.com/egg-mode-rs/egg-mode-text

## MSRV

Rust **1.46** or higher

## Usage

Add the following to your Cargo.toml:

```TOML
[dependencies]
egg-mode = "0.16"
```

By default, `egg-mode` uses `native-tls` for encryption, but also supports `rustls`.
This may be helpful if you wish to avoid linking against `OpenSSL`.
To enable, modify your `Cargo.toml` entry:

```
egg-mode = { version = "0.16", features = ["rustls"], default-features = false }
```

If you also want to avoid using the root certificates on your operating system, the feature
`rustls_webpki` can be used instead to enable `rustls` and compile the Mozilla root certificates
into the final binary, bypassing the operating system's root certificates. To use this feature, put
this in your `Cargo.toml` instead:

```
egg-mode = { version = "0.16", features = ["rustls_webpki"], default-features = false }
```

See available methods and tips to get started in the [Documentation](https://docs.rs/egg-mode/).

### Authentication

To use the twitter API, you must first create an 'application' within twitter's developer portal.
We recommened having a browse of the
[Getting started guide](https://developer.twitter.com/en/docs/twitter-api/getting-started/guide)
before proceeding. Once you have registered your app, you will be given a consumer key and secret
(also known as API key and secret), which you can use to log in.

To authenticate a user and request an access token:

```rust
// NOTE: this assumes you are running inside an `async` function

let con_token = egg_mode::KeyPair::new("consumer key", "consumer secret");
// "oob" is needed for PIN-based auth; see docs for `request_token` for more info
let request_token = egg_mode::auth::request_token(&con_token, "oob").await.unwrap();
let auth_url = egg_mode::auth::authorize_url(&request_token);

// give auth_url to the user, they can sign in to Twitter and accept your app's permissions.
// they'll receive a PIN in return, they need to give this to your application

let verifier = "123456"; //read the PIN from the user here

// note this consumes con_token; if you want to sign in multiple accounts, clone it here
let (token, user_id, screen_name) =
    egg_mode::auth::access_token(con_token, &request_token, verifier).await.unwrap();

// token can be given to any egg_mode method that asks for a token
// user_id and screen_name refer to the user who signed in
```

As the last line shows, this also returns the User ID and username of the user that authenticated
with your application. With this access token, all of the other Twitter functions become available.

With this token in hand, you can get a user's profile information like this:

```rust
let rustlang = egg_mode::user::show("rustlang", &token).await.unwrap();

println!("{} (@{})", rustlang.name, rustlang.screen_name);
```

### Examples

There are more examples in `examples` folder. To run them you will need to create two files
`examples/common/consumer_key` and `examples/common/consumer_secret` containing your consumer
key and secret respectively.

The authentication code for most of them is in `examples/common/mod.rs`, though that's also mostly
wrapped up in code to write the access token to disk and load it back in.

`examples/bearer.rs` is an example of using application-only authentication to get a Bearer token
and use it to load a user's posts. Other examples showcase a handful of actions from their related
module.

If you've found egg-mode useful, or just want to communicate your first impressions of it, please
[track me down on Twitter][qm-twitter] and let me know!

[qm-twitter]: https://twitter.com/QuietMisdreavus

## License

This library is licensed under the Mozilla Public License, version 2.0. See the LICENSE file for details.
