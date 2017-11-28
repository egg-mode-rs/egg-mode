# egg-mode

another twitter library for rust [![Build Status](https://travis-ci.org/QuietMisdreavus/twitter-rs.svg?branch=master)](https://travis-ci.org/QuietMisdreavus/twitter-rs) [![Build status](https://ci.appveyor.com/api/projects/status/3oi86ir82kj1rxu3/branch/master?svg=true)](https://ci.appveyor.com/project/QuietMisdreavus/twitter-rs/branch/master)

[v0.11.0 Documentation][documentation] | [(Pending release documentation)][doc-dev]

[Documentation]: https://tonberry.quietmisdreavus.net/doc/egg_mode/
[doc-dev]: https://tonberry.quietmisdreavus.net/doc-dev/egg_mode/

This is a library for interacting with Twitter from Rust. You can see how much of the Public API is
available in the file [TODO.md]. In addition to eventually implementing the entire Public API, an
explicit goal for egg-mode is to make it as easy as possible for a client of this library to
interact with the Twitter API. Parts of this library are added as a convenience on top of the API
mechanisms; for example, cursored lists of users and tweets can be used as an iterator in addition
to being able to manually load a page at a time.

[TODO.md]: https://github.com/QuietMisdreavus/twitter-rs/blob/master/TODO.md

**NOTE**: Previous versions of egg-mode contained a port of twitter-text to use for character
counting and mention/hashtag/url extraction. That has since been extracted into its own crate,
[egg-mode-text].

[egg-mode-text]: https://github.com/QuietMisdreavus/twitter-text-rs

Compatibility note: egg-mode is tested to run on Rust 1.17.0 and later. On Windows, both the -msvc
and -gnu environments are tested.

To start using this library, put the following into your Cargo.toml:

```TOML
[dependencies]
egg-mode = "0.11.0"
```

...and the following in your lib.rs or main.rs:

```rust
extern crate egg_mode;
```

See available methods and tips to get started in the [Documentation][].

**Note about these code samples:** This README reflects the current release, which uses Hyper v0.11
to provide asynchronous network calls for the interface. The last synchronous release was
[`v0.10.0`], and documentation and code samples for that version can be found on that tag on this
repo.

[`v0.10.0`]: https://github.com/QuietMisdreavus/twitter-rs/tree/v0.10.0

To authenticate a user and request an access token:

```rust
// NOTE: this assumes you have a Tokio `core` and its `handle` sitting around already

let con_token = egg_mode::KeyPair::new("consumer key", "consumer secret");
// "oob" is needed for PIN-based auth; see docs for `request_token` for more info
let request_token = core.run(egg_mode::request_token(&con_token, "oob", &handle)).unwrap();
let auth_url = egg_mode::authorize_url(&request_token);

// give auth_url to the user, they can sign in to Twitter and accept your app's permissions.
// they'll receive a PIN in return, they need to give this to your application

let verifier = "123456"; //read the PIN from the user here

// note this consumes con_token; if you want to sign in multiple accounts, clone it here
let (token, user_id, screen_name) =
    core.run(egg_mode::access_token(con_token, &request_token, verifier, &handle)).unwrap();

// token can be given to any egg_mode method that asks for a token
// user_id and screen_name refer to the user who signed in
```

As the last line shows, this also returns the User ID and username of the user that authenticated
with your application. With this access token, all of the other Twitter functions become available.

With this token in hand, you can get a user's profile information like this:

```rust
// NOTE: as above, this assumes you have the Tokio `core` and `handle` available

let rustlang = core.run(egg_mode::user::show("rustlang", &token, &handle)).unwrap();

println!("{} (@{})", rustlang.name, rustlang.screen_name);
```

If you'd like to see the examples and implementation for the version currently on crates.io, check
the [`v0.11.0`] tag.

[`v0.11.0`]: https://github.com/QuietMisdreavus/twitter-rs/tree/v0.11.0

For more examples of how to use this library, check the files in the examples folder. The
authentication code for most of them is in `examples/common/mod.rs`, though that's also mostly
wrapped up in code to write the access token to disk and load it back in. `examples/bearer.rs` is an
example of using application-only authentication to get a Bearer token and use it to load a user's
posts. Other examples showcase a handful of actions from their related module. To run any of the
examples for yourself, see the notes in `examples/common/mod.rs`.

If you've found egg-mode useful, or just want to communicate your first impressions of it, please
[track me down on Twitter][qm-twitter] and let me know!

[qm-twitter]: https://twitter.com/QuietMisdreavus

## License

This library is licensed under the Mozilla Public License, version 2.0. See the LICENSE file for details.
