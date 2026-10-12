//! W-INVITE (R-GAPS A6/F5): the courier invite is DELIVERED on the venue's own
//! rail, as an outbox entry the existing drain sends, and its delivery state is
//! read back on the courier list. Before this the hub answered a link and the
//! owner had to carry it to the courier by hand: `hiring.rs` had 0 outbox hits.

use super::*;
use crate::edge::mem::{answer_outbound, block_on, sent};
use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::outbox::Entry;
use crate::wire::{Call, Reply};
use serde_json::{json, Value};

const PHONE: &str = "+355 69 123 0000";
const DIGITS: &str = "355691230000";

fn at(path: &str) -> String {
    format!("https://alpha.{PLATFORM_HOST}{path}")
}

fn setting(site: &Site, t: &str, key: &str, value: &str) {
    let r = site.run(crate::services::venue::settings::set_setting, post(&at("/api/owner/settings"), &json!({"key": key, "value": value})).bearer(t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{key}: {}", r.body_str());
}

fn whatsapp(site: &Site, t: &str) {
    for (k, v) in [("notify.whatsapp.token", "wa-token"), ("notify.whatsapp.phone_id", "1234")] {
        setting(site, t, k, v);
    }
}

fn invite(site: &Site, t: &str, body: Value) -> Reply {
    site.run(crate::services::courier::hiring::invite_courier, post(&at("/api/owner/couriers/invite"), &body).bearer(t).on("alpha"), &[])
}

fn waiting(site: &Site) -> Vec<Entry> {
    let place = crate::hubstore::Place::of_authorised(&site.ctx(&[]), "alpha").unwrap();
    block_on(crate::outbox::waiting(&place)).unwrap().into_iter().filter(|e| e.id.starts_with(ID_PREFIX)).collect()
}

fn list(site: &Site, t: &str) -> Value {
    let r = site.run(crate::services::courier::console::couriers, get(&at("/api/owner/couriers")).bearer(t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    r.body_value()
}

fn graph_bodies() -> Vec<Value> {
    sent().iter().filter(|c: &&Call| c.url().unwrap().path().ends_with("/1234/messages")).map(|c| serde_json::from_slice(&c.body_bytes()).unwrap()).collect()
}

/// THE RED (A6): an invite the owner asked to SEND is one outbox entry, to the
/// courier's number, carrying the link -- and the existing drain delivers it.
#[test]
fn an_invite_sent_on_whatsapp_is_one_outbox_entry_that_the_drain_delivers() {
    let site = Site::new();
    let t = site.venue("alpha", "a@x.test");
    whatsapp(&site, &t);
    let r = invite(&site, &t, json!({"phone": PHONE, "name": "Rider", "send": "whatsapp", "lang": "en"}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let v = r.body_value();
    assert_eq!(v["delivery"]["state"], json!("queued"), "{v}");
    let url = v["url"].as_str().expect("the link is still answered").to_string();

    let q = waiting(&site);
    assert_eq!(q.len(), 1, "an invite send that enqueues nothing is the gap: {q:?}");
    assert_eq!((q[0].kind.as_str(), q[0].to.as_str()), ("whatsapp", DIGITS));
    assert!(q[0].text.contains(&url), "the message carries the link: {}", q[0].text);
    assert_eq!(list(&site, &t)["invites"][0]["delivery"]["state"], json!("queued"));

    // THE SAME PHONE AGAIN replaces the pending send: one message, the new link.
    let again = invite(&site, &t, json!({"phone": PHONE, "name": "Rider", "send": "whatsapp", "lang": "en"})).body_value();
    let q = waiting(&site);
    assert_eq!(q.len(), 1, "{q:?}");
    assert!(q[0].text.contains(again["url"].as_str().unwrap()), "{}", q[0].text);

    // THE EXISTING DRAIN sends it, once, and the list reads it as handed over.
    answer_outbound(|_| Reply::from_json(&json!({"messages": [{"id": "wamid.invite"}]})));
    block_on(crate::cron::run(&site.env(), "alpha", site.now_ms + 60_000));
    let got = graph_bodies();
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(got[0]["to"].as_str(), Some(DIGITS));
    assert_eq!(got[0]["text"]["body"].as_str(), Some(q[0].text.as_str()));
    assert!(waiting(&site).is_empty(), "the delivered invite left the queue");
    assert_eq!(list(&site, &t)["invites"][0]["delivery"]["state"], json!("sent"));
}

/// The twin: NO RAIL, NOTHING QUEUED -- the answer says so and the link stays,
/// so the owner's copy/share path is what is left, and the list knows the rails.
#[test]
fn without_a_rail_nothing_is_queued_and_the_answer_says_so_with_the_link_kept() {
    let site = Site::new();
    let t = site.venue("alpha", "a@x.test");
    let v = invite(&site, &t, json!({"phone": PHONE, "name": "Rider", "send": "whatsapp"})).body_value();
    assert_eq!(v["delivery"]["state"], json!("noRail"), "{v}");
    assert!(v["url"].as_str().is_some(), "the copy-link path is kept: {v}");
    assert!(waiting(&site).is_empty());
    let l = list(&site, &t);
    assert_eq!(l["rails"], json!({"whatsapp": false, "telegram": false}), "{l}");
    assert!(l["invites"][0]["delivery"].is_null(), "nothing was sent, nothing is claimed: {l}");
    // And an invite nobody asked to send queues nothing, rail or not.
    whatsapp(&site, &t);
    let v = invite(&site, &t, json!({"phone": "+355690000077", "name": "Other"})).body_value();
    assert!(v["delivery"].is_null(), "{v}");
    assert!(waiting(&site).is_empty());
    assert_eq!(list(&site, &t)["rails"]["whatsapp"], json!(true));
}

/// TELEGRAM goes to the chat the owner names (a bot cannot write to a phone
/// number), and a chat that is not one is refused BEFORE a code is minted.
#[test]
fn a_telegram_send_goes_to_the_named_chat_and_a_bad_chat_mints_nothing() {
    let site = Site::new();
    let t = site.venue("alpha", "a@x.test");
    setting(&site, &t, "notify.telegram.token", "123:abc");
    let bad = invite(&site, &t, json!({"phone": PHONE, "name": "Rider", "send": "telegram", "telegram": "@rider"}));
    assert_eq!(bad.status_code(), 400, "{}", bad.body_str());
    assert_eq!(list(&site, &t)["invites"], json!([]), "a refused send minted no code");
    let v = invite(&site, &t, json!({"phone": PHONE, "name": "Rider", "send": "telegram", "telegram": " 4242001 "})).body_value();
    assert_eq!(v["delivery"]["state"], json!("queued"), "{v}");
    let q = waiting(&site);
    assert_eq!((q.len(), q[0].kind.as_str(), q[0].to.as_str()), (1, "telegram", "4242001"), "{q:?}");
}

/// WITHDRAWN, NOT SENT: a pending send of an invite the owner took back leaves the queue.
#[test]
fn withdrawing_an_invite_drops_its_pending_send() {
    let site = Site::new();
    let t = site.venue("alpha", "a@x.test");
    whatsapp(&site, &t);
    invite(&site, &t, json!({"phone": PHONE, "name": "Rider", "send": "whatsapp"}));
    let id = list(&site, &t)["invites"][0]["id"].as_str().unwrap().to_string();
    assert_eq!(waiting(&site).len(), 1);
    let r = site.run(
        crate::services::courier::hiring::uninvite_courier,
        post(&at(&format!("/api/owner/couriers/{id}/uninvite")), &json!({})).bearer(&t).on("alpha"),
        &[("id", id.as_str())],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert!(waiting(&site).is_empty(), "the withdrawn invite was still going out");
}

fn entry(tries: u32, parked: bool) -> Entry {
    let mut e = Entry::new("courier-invite/abc".into(), "whatsapp", DIGITS.into(), "x".into(), 100);
    e.tries = tries;
    e.parked_ms = parked.then_some(150);
    e
}

/// EVERY STATE THE OWNER CAN READ, from the mark, the queue and the abandon lines.
#[test]
fn the_delivery_state_is_read_from_the_queue_and_the_abandon_lines() {
    let mark = json!({"via": "whatsapp", "outbox": "courier-invite/abc", "atMs": 100});
    let s = |q: &[Entry], f: &[(i64, String)]| state(Some(&mark), q, Some(f))["state"].clone();
    assert_eq!(s(&[entry(0, false)], &[]), json!("queued"));
    assert_eq!(s(&[entry(2, false)], &[]), json!("retrying"));
    assert_eq!(s(&[entry(0, true)], &[]), json!("parked"));
    assert_eq!(s(&[], &[]), json!("sent"));
    assert_eq!(s(&[], &[(200, "courier-invite/abc after 6 attempts".into())]), json!("failed"));
    assert_eq!(s(&[], &[(200, "courier-invite/abc: the bot is gone from chat 1: blocked".into())]), json!("failed"));
    // An older invite's failure, or another phone's, is not this one's.
    assert_eq!(s(&[], &[(50, "courier-invite/abc after 6 attempts".into())]), json!("sent"));
    assert_eq!(s(&[], &[(200, "courier-invite/abcd after 6 attempts".into())]), json!("sent"));
    // The abandon lines could not be read: not "sent", "unknown".
    assert_eq!(state(Some(&mark), &[], None)["state"], json!("unknown"));
    assert!(state(None, &[entry(0, false)], Some(&[])).is_null(), "no mark, nothing claimed");
}

/// THE MESSAGE: in the owner's language, the link last; an unknown language reads English.
#[test]
fn the_message_is_in_the_asked_language_and_ends_with_the_link() {
    let url = "https://alpha.dowiz.org/courier/#claim=X";
    assert!(text(Some("uk"), url).starts_with("Запрошення"), "{}", text(Some("uk"), url));
    assert_eq!(text(Some("xx"), url), text(Some("en"), url));
    assert!(text(None, url).ends_with(url));
    assert_eq!(ask(Some("whatsapp"), PHONE, None), Ok(Some(("whatsapp", DIGITS.to_string()))));
    assert_eq!(ask(None, PHONE, Some("1")), Ok(None), "no send asked, nothing to send");
    assert!(ask(Some("sms"), PHONE, None).is_err(), "an unknown rail is refused, not guessed");
    assert!(ask(Some("telegram"), PHONE, Some("")).is_err());
    assert_eq!(ask(Some("telegram"), PHONE, Some("-100777")), Ok(Some(("telegram", "-100777".to_string()))));
}
