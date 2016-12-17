# egg-mode

another twitter library for rust [![Build Status](https://travis-ci.org/QuietMisdreavus/twitter-rs.svg?branch=master)](https://travis-ci.org/QuietMisdreavus/twitter-rs)

[Documentation][] | [(In-development pre-release documentation)][doc-dev]

[Documentation]: https://shiva.icesoldier.me/doc/egg_mode/
[doc-dev]: https://shiva.icesoldier.me/doc-dev/egg_mode/

This is a library for interacting with Twitter from Rust. You can see how much of the Public API is
available in the file [TODO.md]. In addition to implementing the entire Public API, an explicit goal
for egg-mode is to make it as easy as possible for a client of this library to interact with the
Twitter API. Parts of this library are added as a convenience on top of the API mechanisms; for
example, cursored lists of users and tweets can be used as an iterator in addition to being able to
manually load a page at a time.

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

For more examples of how to use this library, check the files in the examples folder.

## License

This library is licensed under the Apache License, version 2.0. See the LICENSE file for details.
