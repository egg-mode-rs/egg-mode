# egg-mode

another twitter library for rust [![Build Status](https://travis-ci.org/QuietMisdreavus/twitter-rs.svg?branch=master)](https://travis-ci.org/QuietMisdreavus/twitter-rs) [![Build status](https://ci.appveyor.com/api/projects/status/3oi86ir82kj1rxu3?svg=true)](https://ci.appveyor.com/project/QuietMisdreavus/twitter-rs)

[v0.8.1 Documentation][documentation] | [(Pending release documentation)][doc-dev]

[Documentation]: https://tonberry.quietmisdreavus.net/doc/egg_mode/
[doc-dev]: https://tonberry.quietmisdreavus.net/doc-dev/egg_mode/

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
egg-mode = "0.8.1"
```

...and the following in your lib.rs or main.rs:

```rust
extern crate egg_mode;
```

See available methods and tips to get started in the [Documentation][].

To authenticate a user and request an access token:

```rust
let con_token = egg_mode::KeyPair::new("consumer key", "consumer secret");
let request_token = egg_mode::request_token(&con_token, "oob").unwrap();
let auth_url = egg_mode::authorize_url(&request_token);

// give auth_url to the user, they can sign in to Twitter and accept your app's permissions.
// they'll receive a PIN in return, they need to give this to your application

let (token, user_id, screen_name) =
    egg_mode::access_token(con_token, &request_token, pin).unwrap();
```

As the last line shows, this also returns the User ID and username of the user that authenticated
with your application. With this access token, all of the other Twitter functions become available.

With this token in hand, you can get a user's profile information like this:

```rust
let rustlang = egg_mode::user::show("rustlang", &token).unwrap();

println!("{} (@{})", rustlang.name, rustlang.screen_name);
```

If you'd like to see the examples and implementation for the version currently on crates.io, check
the [`v0.8.1`] tag.

[`v0.8.1`]: https://github.com/QuietMisdreavus/twitter-rs/tree/v0.8.1

For more examples of how to use this library, check the files in the examples folder. The
authentication code for most of them is in `examples/common/mod.rs`, though that's also mostly
wrapped up in code to write the access token to disk and load it back in. `examples/bearer.rs` is an
example of using application-only authentication to get a Bearer token and use it to load a user's
posts. Other examples showcase a handful of actions from their related module. To run any of the
examples for yourself, see the notes in `examples/common/mod.rs`.

A note about Windows: As egg-mode uses *ring* as a dependency, egg-mode will not build on
windows-gnu targets. It builds just fine on windows-msvc targets, though.

If you've found egg-mode useful, or just want to communicate your first impressions of it, please
[track me down on Twitter][qm-twitter] and let me know!

[qm-twitter]: https://twitter.com/QuietMisdreavus

## License

This library is licensed under the Mozilla Public License, version 2.0. See the LICENSE file for details.
