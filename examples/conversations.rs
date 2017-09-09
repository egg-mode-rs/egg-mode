// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.

extern crate egg_mode;
extern crate chrono;

mod common;

use common::tokio_core::reactor;

fn main() {
    let mut core = reactor::Core::new().unwrap();

    let c = common::Config::load(&mut core);
    let handle = core.handle();

    let convos = egg_mode::direct::conversations(&c.token, &handle);
    let convos = core.run(convos.newest()).unwrap();

    for (id, convo) in &convos.conversations {
        let user = core.run(egg_mode::user::show(id, &c.token, &handle)).unwrap();
        println!("-----");
        println!("Conversation with @{}:", user.screen_name);
        for msg in convo {
            println!("--@{} sent at {}:",
                     msg.sender_screen_name,
                     msg.created_at.with_timezone(&chrono::Local));
            println!("    {}", msg.text);
        }
        println!("");
    }
}
