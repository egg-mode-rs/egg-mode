# egg-mode

another twitter library for rust [![Build Status](https://travis-ci.org/QuietMisdreavus/twitter-rs.svg?branch=master)](https://travis-ci.org/QuietMisdreavus/twitter-rs)

[Documentation][] | [(In-development pre-release documentation)][doc-dev]

[Documentation]: https://shiva.icesoldier.me/doc/egg_mode/
[doc-dev]: https://shiva.icesoldier.me/doc-dev/egg_mode/

This is a library for interacting with Twitter from Rust. You can see how much of the Public API is
available in the file [TODO.md]. In addition to eventually implementing the entire Public API, an
explicit goal for egg-mode is to make it as easy as possible for a client of this library to
interact with the Twitter API. Parts of this library are added as a convenience on top of the API
mechanisms; for example, cursored lists of users and tweets can be used as an iterator in addition
to being able to manually load a page at a time.

[TODO.md]: https://github.com/QuietMisdreavus/twitter-rs/blob/master/TODO.md

To start using this library, put the following into your Cargo.toml:

```TOML
[dependencies]
egg-mode = "0.7.0"
```

...and the following in your lib.rs or main.rs:

```rust
extern crate egg_mode;
```

See available methods and tips to get started in the [Documentation][].

To authenticate a user and request an access token:

```rust
let consumer_token = egg_mode::Token::new(consumer_key, consumer_secret);
let request_token = egg_mode::request_token(&consumer_token, "oob").unwrap();
let authorize_url = egg_mode::authorize_url(&request_token);

//show authorize_url to the user, have them sign in to Twitter there, and enter the PIN that
//Twitter gives them

let (access_token, user_id, username) = egg_mode::access_token(&consumer_token, &request_token, pin).unwrap();
```

As the last line shows, this also returns the User ID and username of the user that authenticated
with your application. With this access token, all of the other Twitter functions become available.

**NOTE**: Starting in 0.8, the method of authenticating calls is changing slightly.
`egg_mode::access_token` will take ownership of the consumer token passed in, so that it can return
a new Token enum that contains both the consumer and access tokens. This combined Token is then
passed to all the library functions in lieu of the separate key pairs.

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

This library is licensed under the Apache License, version 2.0. See the LICENSE file for details.
