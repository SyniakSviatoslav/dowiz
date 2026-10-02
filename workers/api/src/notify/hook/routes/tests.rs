//! The venue's own Telegram bot through its routes (W-COV C2): connected with a token Telegram
//! accepts (the outbound calls answered by the thread's hook and read back), a group linked by
//! the code the owner was given, a test message, an unlink -- and the webhook refusing a caller
//! without the venue's secret.

use crate::edge::mem::{answer_outbound, sent};
use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::wire::Reply;
use serde_json::json;

fn at(path: &str) -> String {
    format!("https://alpha.{PLATFORM_HOST}{path}")
}

/// A Telegram that says yes to everything, and names the bot.
fn telegram_says_yes() {
    answer_outbound(|c| {
        let url = c.url().map(|u| u.to_string()).unwrap_or_default();
        let result = if url.ends_with("/getMe") { json!({"username": "alpha_bot"}) } else { json!(true) };
        Reply::from_json(&json!({"ok": true, "result": result}))
    });
}

#[test]
fn the_bot_is_connected_a_group_linked_by_code_tested_and_unlinked() {
    let site = Site::new();
    let t = site.venue("alpha", "a@x.test");
    let no_token = site.run(crate::notify::hook::owner::connect, post(&at("/api/owner/telegram/connect"), &json!({})).bearer(&t).on("alpha"), &[]);
    assert!(no_token.status_code() >= 400, "{}", no_token.body_str());
    let early = site.run(crate::notify::hook::owner::link, post(&at("/api/owner/telegram/link"), &json!({})).bearer(&t).on("alpha"), &[]);
    assert!(early.status_code() >= 400, "no bot, no link: {}", early.body_str());

    // Telegram unreachable: the owner is told, nothing is stored.
    let r = site.run(crate::notify::hook::owner::connect, post(&at("/api/owner/telegram/connect"), &json!({"token": "123:abc"})).bearer(&t).on("alpha"), &[]);
    assert!(r.status_code() >= 400, "{}", r.body_str());

    telegram_says_yes();
    let r = site.run(crate::notify::hook::owner::connect, post(&at("/api/owner/telegram/connect"), &json!({"token": "123:abc"})).bearer(&t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(r.body_value()["bot"], "alpha_bot");
    let calls: Vec<String> = sent().iter().map(|c| c.url().unwrap().path().to_string()).collect();
    assert!(calls.iter().any(|p| p.ends_with("/setWebhook")), "the webhook was registered: {calls:?}");

    let state = site.run(crate::notify::hook::owner::state, get(&at("/api/owner/telegram")).bearer(&t).on("alpha"), &[]);
    assert!(state.body_str().contains("alpha_bot"), "{}", state.body_str());

    let link = site.run(crate::notify::hook::owner::link, post(&at("/api/owner/telegram/link"), &json!({})).bearer(&t).on("alpha"), &[]);
    assert_eq!(link.status_code(), 200, "{}", link.body_str());
    let code = link.body_value()["code"].as_str().unwrap().to_string();

    // The webhook: without the secret, refused; with it, the group's /link lands.
    let update = json!({"message": {"chat": {"id": -100555, "type": "supergroup", "title": "Kitchen"}, "text": format!("/link@alpha_bot {code}"), "from": {"id": 7}}});
    let unsigned = site.run(crate::notify::hook::webhook, post(&at("/api/webhooks/telegram"), &update).on("alpha"), &[]);
    assert_eq!(unsigned.status_code(), 401);
    let wrong = site.run(
        crate::notify::hook::webhook,
        post(&at("/api/webhooks/telegram"), &update).with_header(crate::notify::hook::SECRET_HEADER, "nope").on("alpha"),
        &[],
    );
    assert_eq!(wrong.status_code(), 401);
    let settings = crate::edge::mem::block_on(crate::hubstore::load_settings(&crate::hubstore::Place::of_authorised(&site.ctx(&[]), "alpha").unwrap()))
        .unwrap()
        .settings;
    let secret = settings.get(crate::notify::route::groups::KEY_SECRET).expect("secret stored");
    let r = site.run(
        crate::notify::hook::webhook,
        post(&at("/api/webhooks/telegram"), &update).with_header(crate::notify::hook::SECRET_HEADER, &secret).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let state = site.run(crate::notify::hook::owner::state, get(&at("/api/owner/telegram")).bearer(&t).on("alpha"), &[]);
    assert!(state.body_str().contains("-100555"), "the group is linked: {}", state.body_str());

    // A group's `id` is its own; `chat` is Telegram's (-100555).
    let groups = state.body_value()["groups"].clone();
    let g = groups.as_array().and_then(|a| a.iter().find(|g| g["chat"] == "-100555")).expect("the linked group").clone();
    let id = g["id"].as_str().expect("group id").to_string();
    let id = id.as_str();
    let r = site.run(crate::notify::hook::owner::test, post(&at("/api/owner/telegram/test"), &json!({"id": id})).bearer(&t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert!(sent().iter().any(|c| c.url().unwrap().path().ends_with("/sendMessage")), "a test message went out");
    let r = site.run(crate::notify::hook::owner::unlink, post(&at("/api/owner/telegram/unlink"), &json!({"id": id})).bearer(&t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let state = site.run(crate::notify::hook::owner::state, get(&at("/api/owner/telegram")).bearer(&t).on("alpha"), &[]);
    assert!(!state.body_str().contains("-100555"), "{}", state.body_str());
}

/// A venue with a dish, its bot connected and one group (chat -100555) linked by code:
/// (owner token, dish, the group's id).
fn linked_venue(site: &Site) -> (String, String, String) {
    let (t, dish) = crate::storefront::route_tests::open_venue(site, "alpha", "a@x.test");
    telegram_says_yes();
    let r = site.run(crate::notify::hook::owner::connect, post(&at("/api/owner/telegram/connect"), &json!({"token": "123:abc"})).bearer(&t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let link = site.run(crate::notify::hook::owner::link, post(&at("/api/owner/telegram/link"), &json!({})).bearer(&t).on("alpha"), &[]);
    let code = link.body_value()["code"].as_str().unwrap().to_string();
    let settings = crate::edge::mem::block_on(crate::hubstore::load_settings(&crate::hubstore::Place::of_authorised(&site.ctx(&[]), "alpha").unwrap()))
        .unwrap()
        .settings;
    let secret = settings.get(crate::notify::route::groups::KEY_SECRET).expect("secret stored");
    let update = json!({"message": {"chat": {"id": -100555, "type": "supergroup", "title": "Kitchen"}, "text": format!("/link@alpha_bot {code}"), "from": {"id": 7}}});
    let r = site.run(
        crate::notify::hook::webhook,
        post(&at("/api/webhooks/telegram"), &update).with_header(crate::notify::hook::SECRET_HEADER, &secret).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let state = site.run(crate::notify::hook::owner::state, get(&at("/api/owner/telegram")).bearer(&t).on("alpha"), &[]).body_value();
    let id = state["groups"].as_array().unwrap().iter().find(|g| g["chat"] == "-100555").expect("linked")["id"].as_str().unwrap().to_string();
    (t, dish, id)
}

fn order(site: &Site, dish: &str) -> String {
    let r = crate::storefront::route_tests::place_pickup(site, "alpha", dish, 1);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    r.body_value()["id"].as_str().unwrap().to_string()
}

fn drain(site: &Site, at_ms: i64) {
    crate::edge::mem::block_on(crate::outbox::drain_venue(&site.env(), "alpha", at_ms));
}

fn waiting(site: &Site) -> Vec<crate::outbox::Entry> {
    crate::edge::mem::block_on(crate::outbox::waiting(&crate::hubstore::Place::of_authorised(&site.ctx(&[]), "alpha").unwrap())).unwrap()
}

/// Every sendMessage the drain made, as (chat, text).
fn messages() -> Vec<(String, String)> {
    sent()
        .into_iter()
        .filter(|c| c.url().unwrap().path().ends_with("/sendMessage"))
        .map(|c| {
            let v: serde_json::Value = serde_json::from_slice(&c.body_bytes()).unwrap_or_default();
            (v["chat_id"].to_string().trim_matches('"').to_string(), v["text"].as_str().unwrap_or("").to_string())
        })
        .collect()
}

#[test]
fn a_new_order_reaches_the_linked_group_and_telegrams_answers_are_each_obeyed() {
    let site = Site::new();
    let (t, dish, gid) = linked_venue(&site);
    let r = site.run(
        crate::notify::hook::owner::group,
        post(&at("/api/owner/telegram/group"), &json!({"id": gid, "patch": {"subs": {"order.placed": "now"}, "lang": "en"}})).bearer(&t).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let bad = site.run(
        crate::notify::hook::owner::group,
        post(&at("/api/owner/telegram/group"), &json!({"id": gid, "patch": {"subs": {"order.eaten": "now"}}})).bearer(&t).on("alpha"),
        &[],
    );
    assert_eq!(bad.status_code(), 400, "an unknown event is refused: {}", bad.body_str());

    // Delivered: one message to the group's chat, naming the dish; the outbox is empty after.
    let before = messages().len();
    order(&site, &dish);
    assert!(!waiting(&site).is_empty(), "the order's message was written in the order's turn");
    drain(&site, site.now_ms);
    let m = messages();
    assert_eq!(m.len(), before + 1, "{m:?}");
    assert_eq!(m.last().unwrap().0, "-100555");
    assert!(m.last().unwrap().1.contains("Futomaki"), "{m:?}");
    assert!(waiting(&site).iter().all(|e| e.kind != "telegram"), "sent entries leave the outbox");

    // 429: the message waits Telegram's number of seconds and the try does not count.
    answer_outbound(|c| {
        if c.url().unwrap().path().ends_with("/sendMessage") {
            Ok(Reply::from_json(&json!({"ok": false, "error_code": 429, "description": "Too Many Requests", "parameters": {"retry_after": 30}})).unwrap().with_status(429))
        } else {
            Reply::from_json(&json!({"ok": true, "result": true}))
        }
    });
    order(&site, &dish);
    drain(&site, site.now_ms);
    let held: Vec<_> = waiting(&site).into_iter().filter(|e| e.kind == "telegram").collect();
    assert_eq!(held.len(), 1, "{held:?}");
    assert_eq!(held[0].tries, 0, "a 429 does not spend a try");
    assert!(held[0].next_at_ms >= site.now_ms + 30_000, "{held:?}");
    // Before its time nothing is sent.
    let n = messages().len();
    drain(&site, site.now_ms + 1_000);
    assert_eq!(messages().len(), n);

    // 400 + migrate_to_chat_id: the group became a supergroup; the message follows it.
    answer_outbound(|c| {
        let v: serde_json::Value = serde_json::from_slice(&c.body_bytes()).unwrap_or_default();
        if c.url().unwrap().path().ends_with("/sendMessage") && v["chat_id"].to_string().contains("-100555") {
            Ok(Reply::from_json(&json!({"ok": false, "error_code": 400, "description": "group chat was upgraded", "parameters": {"migrate_to_chat_id": -100777}})).unwrap().with_status(400))
        } else {
            Reply::from_json(&json!({"ok": true, "result": true}))
        }
    });
    drain(&site, site.now_ms + 31_000);
    drain(&site, site.now_ms + 31_001);
    assert!(messages().iter().any(|(chat, _)| chat == "-100777"), "{:?}", messages());
    let state = site.run(crate::notify::hook::owner::state, get(&at("/api/owner/telegram")).bearer(&t).on("alpha"), &[]).body_value();
    assert!(state.to_string().contains("-100777"), "the group's chat id was rewritten: {state}");

    // 403: the bot was removed. The group is kept but marked as left, its health says why, and
    // nothing retries the chat.
    answer_outbound(|c| {
        if c.url().unwrap().path().ends_with("/sendMessage") {
            Ok(Reply::from_json(&json!({"ok": false, "error_code": 403, "description": "Forbidden: bot was kicked from the supergroup chat"})).unwrap().with_status(403))
        } else {
            Reply::from_json(&json!({"ok": true, "result": true}))
        }
    });
    order(&site, &dish);
    drain(&site, site.now_ms + 40_000);
    let state = site.run(crate::notify::hook::owner::state, get(&at("/api/owner/telegram")).bearer(&t).on("alpha"), &[]).body_value();
    let g = state["groups"].as_array().unwrap().iter().find(|g| g["chat"] == "-100777").cloned().unwrap_or_else(|| panic!("{state}"));
    assert_eq!(g["state"], "left", "{g}");
    assert_eq!(g["health"]["failing"], true, "{g}");
    assert!(g["health"]["err"].as_str().unwrap().contains("kicked"), "{g}");
    assert!(waiting(&site).iter().all(|e| !(e.kind == "telegram" && e.to.contains("-100777"))), "nothing retries a gone chat");
}

#[test]
fn a_digest_group_gets_one_summary_at_its_own_minute_and_the_next_one_tomorrow() {
    let site = Site::new();
    let (t, dish, gid) = linked_venue(&site);
    let r = site.run(
        crate::notify::hook::owner::group,
        post(&at("/api/owner/telegram/group"), &json!({"id": gid, "patch": {"subs": {"order.placed": "digest"}, "digest_at": 600, "lang": "en"}})).bearer(&t).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let before = messages().len();
    order(&site, &dish);
    order(&site, &dish);
    drain(&site, site.now_ms);
    assert_eq!(messages().len(), before, "a digest group is not told each order as it comes");
    let digest = || waiting(&site).into_iter().find(|e| e.kind == "digest").unwrap_or_else(|| panic!("{:?}", waiting(&site)));
    let due = digest().next_at_ms;
    assert!(due > site.now_ms, "the summary waits for its minute");

    // 429 at the minute: it waits Telegram's seconds, the summary is not lost.
    answer_outbound(|c| {
        if c.url().unwrap().path().ends_with("/sendMessage") {
            Ok(Reply::from_json(&json!({"ok": false, "error_code": 429, "description": "Too Many Requests", "parameters": {"retry_after": 20}})).unwrap().with_status(429))
        } else {
            Reply::from_json(&json!({"ok": true, "result": true}))
        }
    });
    drain(&site, due);
    assert_eq!(digest().next_at_ms, due + 20_000);

    telegram_says_yes();
    let n = messages().len();
    drain(&site, due + 20_000);
    let m = messages();
    assert_eq!(m.len(), n + 1, "{m:?}");
    assert_eq!(m.last().unwrap().0, "-100555");
    assert!(m.last().unwrap().1.contains("Futomaki"), "the summary names what was ordered: {}", m.last().unwrap().1);
    let next = digest().next_at_ms;
    assert!(next > due + 12 * 3_600_000 && next <= due + 25 * 3_600_000, "tomorrow, at the same minute: {due} -> {next}");
    // Drained again before tomorrow: nothing more.
    drain(&site, due + 60_000);
    assert_eq!(messages().len(), n + 1);

    // A failure that is not Telegram's wait is retried with a backoff, not dropped.
    answer_outbound(|c| {
        if c.url().unwrap().path().ends_with("/sendMessage") {
            Ok(Reply::from_json(&json!({"ok": false, "error_code": 500, "description": "Internal"})).unwrap().with_status(500))
        } else {
            Reply::from_json(&json!({"ok": true, "result": true}))
        }
    });
    drain(&site, next);
    let d = digest();
    assert_eq!(d.tries, 1, "{d:?}");
    assert!(d.next_at_ms > next && d.next_at_ms < next + 3_600_000, "{d:?}");

    // The bot kicked at the minute: the group is marked left and the summary moves to tomorrow.
    answer_outbound(|c| {
        if c.url().unwrap().path().ends_with("/sendMessage") {
            Ok(Reply::from_json(&json!({"ok": false, "error_code": 403, "description": "Forbidden: bot was kicked"})).unwrap().with_status(403))
        } else {
            Reply::from_json(&json!({"ok": true, "result": true}))
        }
    });
    drain(&site, d.next_at_ms);
    let state = site.run(crate::notify::hook::owner::state, get(&at("/api/owner/telegram")).bearer(&t).on("alpha"), &[]).body_value();
    let g = state["groups"].as_array().unwrap().iter().find(|g| g["id"] == gid.as_str()).cloned().unwrap();
    assert_eq!(g["state"], "left", "{g}");
}
