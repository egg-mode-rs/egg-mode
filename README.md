# egg-mode

another twitter library for rust [![Build Status](https://travis-ci.org/QuietMisdreavus/twitter-rs.svg?branch=master)](https://travis-ci.org/QuietMisdreavus/twitter-rs)

[Documentation][] | [(In-development pre-release documentation)][doc-dev]

[Documentation]: https://shiva.icesoldier.me/doc/egg_mode/
[doc-dev]: https://shiva.icesoldier.me/doc-dev/egg_mode/

This is an early library for interacting with Twitter. It's still pretty early days, but it's also
being actively developed. The aim with this is complete integration with Twitter's v1.1 API.

To start using this library, put the following into your Cargo.toml:

```TOML
[dependencies]
egg-mode = "0.3.0"
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
