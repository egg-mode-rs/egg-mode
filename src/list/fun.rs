// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

use super::*;

use crate::cursor::{CursorIter, ListCursor, UserCursor};
use crate::error::{Error::TwitterError, Result};
use crate::user::{TwitterUser, UserID};
use crate::{auth, links, tweet};

///Look up the lists the given user has been added to.
///
///This function returns a `Stream` over the lists returned by Twitter. This method defaults to
///reeturning 20 lists in a single network call; the maximum is 1000.
pub fn memberships<'a, T: Into<UserID<'a>>>(
    user: T,
    token: &auth::Token,
) -> CursorIter<'a, ListCursor> {
    let params = ParamList::new().add_name_param(&user.into());
    CursorIter::new(links::lists::MEMBERSHIPS, token, Some(params), Some(20))
}

///Return up to 100 lists the given user is subscribed to, including those the user made
///themselves.
///
///This function can be used to get a snapshot of a user's lists, but if they've created or
///subscribed to a lot of lists, then the limitations of this function can get in the way.
///If the `owned_first` parameter is `true`, Twitter will load the lists the given user created,
///then the ones they've subscribed to, stopping when it reaches 100 lists. If it's `false`, then
///the lists are loaded in the opposite order.
///
///If the user has more than 100 lists total like this, you'll need to call `ownerships` and
///`subscriptions` separately to be able to properly load everything.
pub async fn list<'id, T: Into<UserID<'id>>>(
    user: T,
    owned_first: bool,
    token: &auth::Token,
) -> Result<Response<Vec<List>>> {
    let params = ParamList::new()
        .add_name_param(&user.into())
        .add_param("reverse", owned_first.to_string());

    let req = auth::get(links::lists::LIST, token, Some(&params));

    make_parsed_future(req).await
}

///Look up the lists the given user is subscribed to, but not ones the user made themselves.
///
///This function returns a `Stream` over the lists returned by Twitter. This method defaults to
///reeturning 20 lists in a single network call; the maximum is 1000.
pub fn subscriptions<'a, T: Into<UserID<'a>>>(
    user: T,
    token: &auth::Token,
) -> CursorIter<'a, ListCursor> {
    let params = ParamList::new().add_name_param(&user.into());
    CursorIter::new(links::lists::SUBSCRIPTIONS, token, Some(params), Some(20))
}

///Look up the lists created by the given user.
///
///This function returns a `Stream` over the lists returned by Twitter. This method defaults to
///reeturning 20 lists in a single network call; the maximum is 1000.
pub fn ownerships<'a, T: Into<UserID<'a>>>(
    user: T,
    token: &auth::Token,
) -> CursorIter<'a, ListCursor> {
    let params = ParamList::new().add_name_param(&user.into());
    CursorIter::new(links::lists::OWNERSHIPS, token, Some(params), Some(20))
}

///Look up information for a single list.
pub async fn show(list: ListID<'_>, token: &auth::Token) -> Result<Response<List>> {
    let params = ParamList::new().add_list_param(&list);

    let req = auth::get(links::lists::SHOW, token, Some(&params));

    make_parsed_future(req).await
}

///Look up the users that have been added to the given list.
///
///This function returns a `Stream` over the users returned by Twitter. This method defaults to
///reeturning 20 users in a single network call; the maximum is 5000.
pub fn members<'a>(list: ListID<'a>, token: &auth::Token) -> CursorIter<'a, UserCursor> {
    let params = ParamList::new().add_list_param(&list);

    CursorIter::new(links::lists::MEMBERS, token, Some(params), Some(20))
}

///Look up the users that have subscribed to the given list.
///
///This function returns a `Stream` over the users returned by Twitter. This method defaults to
///reeturning 20 users in a single network call; the maximum is 5000.
pub fn subscribers<'a>(list: ListID<'a>, token: &auth::Token) -> CursorIter<'a, UserCursor> {
    let params = ParamList::new().add_list_param(&list);

    CursorIter::new(links::lists::SUBSCRIBERS, token, Some(params), Some(20))
}

///Check whether the given user is subscribed to the given list.
pub async fn is_subscribed<'id, T: Into<UserID<'id>>>(
    user: T,
    list: ListID<'_>,
    token: &auth::Token,
) -> Result<Response<bool>> {
    let params = ParamList::new()
        .add_list_param(&list)
        .add_name_param(&user.into());

    let req = auth::get(links::lists::IS_SUBSCRIBER, token, Some(&params));

    fn parse_resp(full_resp: String, headers: &Headers) -> Result<Response<bool>> {
        let out: WebResponse<TwitterUser> = make_response(full_resp, headers);

        match out {
            Ok(user) => Ok(Response::map(user, |_| true)),
            Err(TwitterError(terrs)) => {
                if terrs.errors.iter().any(|e| e.code == 109) {
                    //here's a fun conundrum: since "is not in this list" is returned as an error code,
                    //the rate limit info that would otherwise be part of the response isn't there. the
                    //rate_headers method was factored out specifically for this location, since it's
                    //still there, just accompanying an error response instead of a user.
                    Ok(Response::map(rate_headers(headers)?, |_| false))
                } else {
                    Err(TwitterError(terrs))
                }
            }
            Err(err) => Err(err),
        }
    }

    make_future(req, parse_resp).await
}

///Check whether the given user has been added to the given list.
pub async fn is_member<'id, T: Into<UserID<'id>>>(
    user: T,
    list: ListID<'_>,
    token: &auth::Token,
) -> Result<Response<bool>> {
    let params = ParamList::new()
        .add_list_param(&list)
        .add_name_param(&user.into());

    let req = auth::get(links::lists::IS_MEMBER, token, Some(&params));

    fn parse_resp(full_resp: String, headers: &Headers) -> Result<Response<bool>> {
        let out: WebResponse<TwitterUser> = make_response(full_resp, headers);

        match out {
            Ok(user) => Ok(Response::map(user, |_| true)),
            Err(TwitterError(terrs)) => {
                if terrs.errors.iter().any(|e| e.code == 109) {
                    //here's a fun conundrum: since "is not in this list" is returned as an error code,
                    //the rate limit info that would otherwise be part of the response isn't there. the
                    //rate_headers method was factored out specifically for this location, since it's
                    //still there, just accompanying an error response instead of a user.
                    Ok(Response::map(rate_headers(headers)?, |_| false))
                } else {
                    Err(TwitterError(terrs))
                }
            }
            Err(err) => Err(err),
        }
    }

    make_future(req, parse_resp).await
}

///Begin navigating the collection of tweets made by the users added to the given list.
///
///The interface for loading statuses from a list is exactly the same as loading from a personal
///timeline. see the [`Timeline`] docs for details.
///
///[`Timeline`]: ../tweet/struct.Timeline.html
pub fn statuses<'a>(list: ListID<'a>, with_rts: bool, token: &auth::Token) -> tweet::Timeline<'a> {
    let params = ParamList::new()
        .add_list_param(&list)
        .add_param("include_rts", with_rts.to_string());

    tweet::Timeline::new(links::lists::STATUSES, Some(params), token)
}

///Adds the given user to the given list.
///
///Note that lists cannot have more than 5000 members.
///
///Upon success, the future returned by this function yields the freshly-modified list.
pub async fn add_member<'id, T: Into<UserID<'id>>>(
    list: ListID<'_>,
    user: T,
    token: &auth::Token,
) -> Result<Response<List>> {
    let params = ParamList::new()
        .add_list_param(&list)
        .add_name_param(&user.into());

    let req = auth::post(links::lists::ADD, token, Some(&params));

    make_parsed_future(req).await
}

///Adds a set of users to the given list.
///
///The `members` param can be used the same way as the `accts` param in [`user::lookup`]. See that
///method's documentation for details.
///
///[`user::lookup`]: ../user/fn.lookup.html
///
///Note that you cannot add more than 100 members to a list at a time, and that lists in general
///cannot have more than 5000 members.
///
///When using this method, take care not to add and remove many members in rapid succession; there
///are no guarantees that the result of a `add_member_list` or `remove_member_list` will be
///immediately available for a corresponding removal or addition, respectively.
pub async fn add_member_list<'id, T, I>(
    members: I,
    list: ListID<'_>,
    token: &auth::Token,
) -> Result<Response<List>>
where
    T: Into<UserID<'id>>,
    I: IntoIterator<Item = T>,
{
    let (id_param, name_param) = multiple_names_param(members);
    let params = ParamList::new()
        .add_list_param(&list)
        .add_opt_param(
            "user_id",
            if !id_param.is_empty() {
                Some(id_param)
            } else {
                None
            },
        )
        .add_opt_param(
            "screen_name",
            if !name_param.is_empty() {
                Some(name_param)
            } else {
                None
            },
        );

    let req = auth::post(links::lists::ADD_LIST, token, Some(&params));

    make_parsed_future(req).await
}

///Removes the given user from the given list.
pub async fn remove_member<'id, T: Into<UserID<'id>>>(
    list: ListID<'_>,
    user: T,
    token: &auth::Token,
) -> Result<Response<List>> {
    let params = ParamList::new()
        .add_list_param(&list)
        .add_name_param(&user.into());

    let req = auth::post(links::lists::REMOVE_MEMBER, token, Some(&params));

    make_parsed_future(req).await
}

///Removes a set of users from the given list.
///
///The `members` param can be used the same way as the `accts` param in [`user::lookup`]. See that
///method's documentation for details.
///
///[`user::lookup`]: ../user/fn.lookup.html
///
///This method is limited to removing 100 members at a time.
///
///When using this method, take care not to add and remove many members in rapid succession; there
///are no guarantees that the result of a `add_member_list` or `remove_member_list` will be
///immediately available for a corresponding removal or addition, respectively.
pub async fn remove_member_list<'a, T, I>(
    members: I,
    list: ListID<'_>,
    token: &auth::Token,
) -> Result<Response<List>>
where
    T: Into<UserID<'a>>,
    I: IntoIterator<Item = T>,
{
    let (id_param, name_param) = multiple_names_param(members);
    let params = ParamList::new()
        .add_list_param(&list)
        .add_opt_param(
            "user_id",
            if !id_param.is_empty() {
                Some(id_param)
            } else {
                None
            },
        )
        .add_opt_param(
            "screen_name",
            if !name_param.is_empty() {
                Some(name_param)
            } else {
                None
            },
        );

    let req = auth::post(links::lists::REMOVE_LIST, token, Some(&params));

    make_parsed_future(req).await
}

///Creates a list, with the given name, visibility, and description.
///
///The new list is owned by the authenticated user, and its slug can be created with their handle
///and the name given to `name`. Twitter places an upper limit on 1000 lists owned by a single
///account.
pub async fn create(
    name: &str,
    public: bool,
    desc: Option<&str>,
    token: &auth::Token,
) -> Result<Response<List>> {
    let params = ParamList::new()
        .add_param("name", name)
        .add_param("mode", if public { "public" } else { "private" })
        .add_opt_param("description", desc);

    let req = auth::post(links::lists::CREATE, token, Some(&params));

    make_parsed_future(req).await
}

///Deletes the given list.
///
///The authenticated user must have created the list.
pub async fn delete(list: ListID<'_>, token: &auth::Token) -> Result<Response<List>> {
    let params = ParamList::new().add_list_param(&list);

    let req = auth::post(links::lists::DELETE, token, Some(&params));

    make_parsed_future(req).await
}

///Subscribes the authenticated user to the given list.
///
///Subscribing to a list is a way to make it available in the "Lists" section of a user's profile
///without having to create it themselves.
pub async fn subscribe(list: ListID<'_>, token: &auth::Token) -> Result<Response<List>> {
    let params = ParamList::new().add_list_param(&list);

    let req = auth::post(links::lists::SUBSCRIBE, token, Some(&params));

    make_parsed_future(req).await
}

///Unsubscribes the authenticated user from the given list.
pub async fn unsubscribe(list: ListID<'_>, token: &auth::Token) -> Result<Response<List>> {
    let params = ParamList::new().add_list_param(&list);

    let req = auth::post(links::lists::UNSUBSCRIBE, token, Some(&params));

    make_parsed_future(req).await
}

///Begins updating a list's metadata.
///
///This method is exposed using a builder struct. See the [`ListUpdate`] docs for details.
///
///[`ListUpdate`]: struct.ListUpdate.html
pub fn update<'a>(list: ListID<'a>) -> ListUpdate<'a> {
    ListUpdate {
        list: list,
        name: None,
        public: None,
        desc: None,
    }
}
