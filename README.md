# egg-mode

another twitter library for rust

This is an early library for interacting with Twitter. So far the only thing I've implemented is
getting an access token and looking up a user. The aim with this is complete integration with
Twitter's v1.1 API.

To authenticate a user and request an access token:

```rust
extern crate egg_mode;

let consumer_token = egg_mode::Token::new(consumer_key, consumer_secret);
let request_token = egg_mode::request_token(&consumer_token, "oob").unwrap();
let authorize_url = egg_mode::authorize_url(&request_token);

//show authorize_url to the user, have them sign in to Twitter there, and enter the PIN that
//Twitter gives them

let (access_token, user_id, username) = egg_mode::access_token(&consumer_token, &request_token, pin).unwrap();
```

As the last line shows, this also returns the User ID and username of the user that authenticated
with your application. With this access token, all of the other Twitter functions become available.

The file `examples/basic.rs` shows the process of loading an access token and using that to show
some information about the authenticated user.

## License

This library is licensed under the Apache License, version 2.0. See the LICENSE file for details.
