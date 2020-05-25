# contributing to egg-mode

So you'd like to help out with egg-mode? Great! Here are some ways you can contribute, in terms of
"things that take place on github". If you have a suggestion for the library but can't or don't want
to go through the PR process, feel free to leave an issue on github, or contact me on twitter
(@QuietMisdreavus), IRC (misdreavus on the Rust channels), or even email (grey @ (my twitter handle)
. net).

## Documentation Contributions

I want egg-mode to be a gold standard for crate documentation. While I've written a lot of
high-level stuff to guide the crate as a whole, there are still many little things that could use
improvement. Anything that would help bring this crate inline with [RFC 1574] - adding examples to
functions and types, noting error conditions of functions, fleshing out a specific docs page, even
just removing some of the extra empty `[]` on some links - will always be appreciated.

[RFC 1574]: https://github.com/rust-lang/rfcs/blob/master/text/1574-more-api-documentation-conventions.md

I want to make it so that you don't have to consult the Twitter API docs for a given endpoint to
understand everything a given function will do. I also like to be as thorough as I can, even filling
holes in the official docs. For example, in the page for [`friendships/create`] Twitter doesn't
mention the API effect of trying to follow a protected user. When documenting [`user::follow`] I
tried just that, to see what would happen, and I included that note in the documentation. If you
notice such holes in egg-mode documentation, please post an issue or create a pull request!

[`friendships/create`]: https://dev.twitter.com/rest/reference/post/friendships/create
[`user::follow`]: https://docs.rs/egg-mode/0.14.0/egg_mode/user/fn.follow.html

## Code Contributions

I have one very obvious quantifiable goal with egg-mode: complete [TODO.md](./TODO.md). PRs that
help towards that end are greatly appreciated! Even if the code doesn't make a similar API to the
rest of egg-mode, that's still fine! I can tweak it afterward. PRs that refactor some internal code
structure, or update/change dependencies are also fine, depending on the extent of the change. If
you have any questions about something, feel free to open an issue or contact me.

### API Guidelines

As egg-mode is for the most part the work of one person, I've been able to be very opinionated on
the overall design of the library. If you're adding new features or endpoints, making the new API
surface match what's already there will greatly speed up the review process and let me integrate
your code much more easily.

#### "Raw" API Access

For the most part, I've strived to give a dedicated function to each endpoint that I add to
egg-mode. This might be part of a wrapper struct, or have a better binding elsewhere in the library,
but if someone coming from another Twitter library wants to know where they can call some specific
endpoint, I want to make sure there's *something* I can point at, at least so I have some direct
method to reference in TODO.md.

#### Carry State Only When Necessary

A conscious decision I made over time was to make as many things bare functions as I could feasibly
get away with. These functions might all take a `Token` which extends their argument signature some,
but (in my opinion) it makes reading documentation easier when you can stumble on (for example) [a
function to load a single Twitter user][user-show] and understand that you need a `Token` and
something that can become a `UserID`. Thanks to rustdoc, both those types have hyperlinks to their
respective documentation where you can figure out how to get the required arguments.

[user-show]: https://docs.rs/egg-mode/0.14.0/egg_mode/user/fn.show.html

This doesn't mean that absolutely everything needs to be in bare functions. Some actions, like
loading tweets from a timeline, have some explicit state to them, in the form of their cursoring
IDs. In these situations, it can be better to package up the state so the user doesn't have to keep
an opaque token around. This gets to the next point:

#### Wrap Common Patterns With Common Abstractions

Sometimes, there are some common API interaction patterns that Twitter uses that recur in different
places around the API. For example, the [endpoint to load the users who retweeted a
status][retweeters-of], the [endpoint to load the users who follow a given account][followers-of],
and the [endpoint to load the lists a given user has created][list-ownerships] all share some
similarities: They all have a `cursor` parameter, and they all return some list of results,
alongside a handful of fields that relate to that cursor ID. To wrap this in egg-mode, rather than
require the user to juggle those cursor IDs themselves, I created [CursorIter][], which handles the
pagination itself. Similarly, endpoints that load a list of statuses or direct messages all just
return a list of items, and have `since_id`/`max_id` parameters to load various sets of results.
This interaction was wrapped into the respective `Timeline` structs, which handle this pagination
without the user having to juggle these tweet IDs for basic scenarios.

[retweeters-of]: https://dev.twitter.com/rest/reference/get/statuses/retweeters/ids
[followers-of]: https://dev.twitter.com/rest/reference/get/followers/list
[list-ownerships]: https://dev.twitter.com/rest/reference/get/lists/ownerships
[CursorIter]: https://docs.rs/egg-mode/0.14.0/egg_mode/cursor/struct.CursorIter.html

If you're adding new endpoints, check to make sure whether they follow one of these patterns. That
way, you can reuse that code and keep the library from having too many copies of the same thing. (On
a similar note, a refactor to combine `tweet::Timeline` and `direct::Timeline` is on my personal
wishlist! It'll probably require a similar backflip to `CursorIter`, where it has a special trait
and a type parameter that's mostly only used internally.)

#### Going Above The Raw Calls

In some situations, I've been able to add helper methods that smooth over the raw calls in come way.
Things like the `Iterator` implementation of `CursorIter`, or `direct::conversations`, allow the
user to interact with the API in a more natural way, or in a way that more closely mirrors a client
experience. Adding similar facilities can help turn egg-mode into a better library overall.

#### The Anatomy of an API Call

A fair amount of the code in egg-mode is infrastructure to support the API calls. So an individual
function may just be "collect the parameters, call Twitter, parse the result", but the framework
underpinning that may require some additional work to support new functionality.

If you need to parse a new struct from an API response, put it into a plain struct and derive
`Deserialize` for it. It may be that a custom `Deserialize` implementation is necessary, in which case
look at some of the other `Deserialize` implementations for hints. For example, `Tweet` uses a custom
implementation which uses an internal `RawTweet` struct for the initial deserialization, then perfoms
some extra logic (notably having to convert the codepoint indices into byte offsets) before creating the
struct proper.
