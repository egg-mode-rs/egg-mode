# twitter-rs

another twitter library for rust

This is an early library for interacting with Twitter. So far the only thing I've implemented is
getting an access token and looking up a user. The aim with this is complete integration with
Twitter's v1.1 API.

To authenticate a user and request an access token:

```rust
use twitter::auth;

let consumer_token = auth::Token::new(consumer_key, consumer_secret);
let request_token = auth::request_token(&consumer_token, "oob").unwrap();
let authorize_url = auth::authorize_url(&request_token);

//show authorize_url to the user, have them sign in to Twitter there, and enter the PIN that
//Twitter gives them

let (access_token, user_id, username) = auth::access_token(&consumer_token, &request_token, pin).unwrap();
```

As the last line shows, this also returns the User ID and username of the user that authenticated
with your application. With this access token, all of the other Twitter functions become available.

The file `examples/basic.rs` shows the process of loading an access token and using that to show
some information about the authenticated user.

## License

This library is licensed under the GNU Lesser General Public License v3 or later. See the `LICENSE`
file and [the original GPL][gpl] for gory details. Essentially, if you use this library in an
application, you don't need to distribute its source code, but you do need to link back here.

[gpl]: http://www.gnu.org/licenses/gpl-3.0.txt
