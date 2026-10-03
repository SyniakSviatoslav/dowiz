//! W-INT2 #5 / #7: the outbox END TO END, natively -- an order placed through the storefront
//! route lands in the venue's outbox beside the order, the venue's runner (`cron::run`, what the
//! alarm asks for) drains it, and the fake provider at the edge (`edge::mem::answer_outbound`)
//! receives the exact text that was queued. Without the rail's credentials the entry WAITS: it
//! is still queued after the run, and nothing was sent.

use crate::edge::mem::{answer_outbound, block_on, sent};
use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::{open_venue, place_pickup};
use crate::wire::{Call, Reply};
use serde_json::{json, Value};

fn at(path: &str) -> String {
    format!("https://alpha.{PLATFORM_HOST}{path}")
}

fn setting(site: &Site, t: &str, key: &str, value: &str) {
    let r = site.run(crate::services::venue::settings::set_setting, post(&at("/api/owner/settings"), &json!({"key": key, "value": value})).bearer(t).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{key}: {}", r.body_str());
}

fn place(site: &Site) -> crate::hubstore::Place {
    crate::hubstore::Place::of_authorised(&site.ctx(&[]), "alpha").unwrap()
}

fn waiting(site: &Site) -> Vec<crate::outbox::Entry> {
    block_on(crate::outbox::waiting(&place(site))).unwrap()
}

fn bodies(path_end: &str) -> Vec<Value> {
    sent()
        .iter()
        .filter(|c| c.url().unwrap().path().ends_with(path_end))
        .map(|c| serde_json::from_slice::<Value>(&c.body_bytes()).unwrap())
        .collect()
}

#[test]
fn an_order_rings_telegram_through_the_outbox_and_waits_without_a_token() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    setting(&site, &t, "notify.telegram.chat", "-4242");
    let id = place_pickup(&site, "alpha", &dish, 2).body_value()["id"].as_str().expect("order id").to_string();

    // PRODUCED: the order's bell is in the outbox, beside the order.
    let queued: Vec<_> = waiting(&site).into_iter().filter(|e| e.kind == "telegram").collect();
    assert_eq!(queued.len(), 1, "{queued:?}");
    let text = queued[0].text.clone();
    assert!(text.contains("Futomaki") && text.contains(&id[..8]), "{text}");

    // NO TOKEN: the runner leaves it queued and calls nobody.
    answer_outbound(|_| Reply::from_json(&json!({"ok": true, "result": {"message_id": 1}})));
    let later = site.now_ms + 60_000;
    block_on(crate::cron::run(&site.env(), "alpha", later));
    assert!(bodies("/sendMessage").is_empty(), "sent without a token");
    assert_eq!(waiting(&site).iter().filter(|e| e.kind == "telegram").count(), 1, "a rail with no token dropped the bell");

    // CONSUMED: with the venue's token, the next run sends exactly that text to that chat, once.
    setting(&site, &t, "notify.telegram.token", "123:abc");
    block_on(crate::cron::run(&site.env(), "alpha", later + 60_000));
    let got = bodies("/sendMessage");
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(got[0]["chat_id"].as_str().map(str::to_string).unwrap_or_else(|| got[0]["chat_id"].to_string()), "-4242");
    assert_eq!(got[0]["text"].as_str(), Some(text.as_str()), "the provider got other words than the outbox held");
    assert!(sent().iter().any(|c| c.url().unwrap().path().starts_with("/bot123:abc/")), "the venue's own bot");
    assert!(waiting(&site).iter().all(|e| e.kind != "telegram"), "the delivered bell is gone");
}

#[test]
fn an_order_announced_on_whatsapp_waits_for_the_number_and_then_reaches_graph() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    for (k, v) in [("notify.whatsapp.status", "on"), ("notify.whatsapp.token", "wa-token"), ("notify.whatsapp.phone_id", "1234"), ("notify.whatsapp.to", "355690000009")] {
        setting(&site, &t, k, v);
    }
    let id = place_pickup(&site, "alpha", &dish, 1).body_value()["id"].as_str().expect("order id").to_string();
    let wa: Vec<_> = waiting(&site).into_iter().filter(|e| e.kind == "whatsapp").collect();
    assert_eq!(wa.len(), 1, "{wa:?}");
    assert_eq!(wa[0].to, "355690000009");

    // The number is taken away before the run: the entry waits, Graph is not called.
    setting(&site, &t, "notify.whatsapp.token", "");
    answer_outbound(|_| Reply::from_json(&json!({"messages": [{"id": "wamid.bell"}]})));
    block_on(crate::cron::run(&site.env(), "alpha", site.now_ms + 60_000));
    assert!(bodies("/messages").is_empty(), "sent without a token");
    assert_eq!(waiting(&site).iter().filter(|e| e.kind == "whatsapp").count(), 1, "dropped instead of waiting");

    setting(&site, &t, "notify.whatsapp.token", "wa-token");
    block_on(crate::cron::run(&site.env(), "alpha", site.now_ms + 120_000));
    let got = bodies("/1234/messages");
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!((got[0]["to"].as_str(), got[0]["type"].as_str()), (Some("355690000009"), Some("text")));
    assert_eq!(got[0]["text"]["body"].as_str(), Some(wa[0].text.as_str()));
    assert!(wa[0].text.contains(&id[..8]), "{}", wa[0].text);
    assert!(waiting(&site).iter().all(|e| e.kind != "whatsapp"));
}

#[test]
fn a_campaign_reaches_a_consented_customer_through_the_outbox_as_a_template() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    let r = site.run(
        crate::storefront::place,
        post(
            &at("/api/public/locations/alpha/orders"),
            &json!({"items": [{"product_id": dish, "quantity": 1}], "contact": {"name": "Ana Hoxha", "phone": "+355691112227"},
                    "fulfilment": {"kind": "pickup"}, "payment": "cash"}),
        )
        .on("alpha"),
        &[("slug", "alpha")],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let list = site.run(crate::services::customers::handlers::customers, get(&at("/api/owner/customers")).bearer(&t).on("alpha"), &[]);
    let key = list.body_value()["customers"][0]["key"].as_str().expect("a customer").to_string();
    let r = site.run(
        crate::services::customers::consent_routes::owner_act,
        post(&at(&format!("/api/owner/customers/{key}/consent")), &json!({"state": "given", "evidence": "said yes at the counter", "lang": "sq"})).bearer(&t).on("alpha"),
        &[("key", &key)],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let r = site.run(
        crate::services::campaigns::handlers::define,
        post(
            &at("/api/owner/campaigns"),
            &json!({"name": "Welcome back", "text": "We miss you", "segment": {"kind": "everyone_consented"}, "template": {"name": "welcome_back", "lang": "sq"}}),
        )
        .bearer(&t)
        .on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let id = r.body_value()["campaign"]["id"].as_str().expect("id").to_string();
    let r = site.run(
        crate::services::campaigns::handlers::send_now,
        Call::new(&at(&format!("/api/owner/campaigns/{id}/send")), worker::Method::Post).unwrap().with_json(&json!({"confirm": true})).bearer(&t).on("alpha"),
        &[("id", &id)],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let kind = crate::services::campaigns::send::OUTBOX_KIND;
    let queued: Vec<_> = waiting(&site).into_iter().filter(|e| e.kind == kind).collect();
    assert_eq!(queued.len(), 1, "one consented customer, one message: {queued:?}");

    // No WhatsApp number yet: it waits.
    answer_outbound(|_| Reply::from_json(&json!({"messages": [{"id": "wamid.camp"}]})));
    block_on(crate::cron::run(&site.env(), "alpha", site.now_ms + 60_000));
    assert!(bodies("/messages").is_empty());
    assert_eq!(waiting(&site).iter().filter(|e| e.kind == kind).count(), 1, "dropped instead of waiting");

    setting(&site, &t, "notify.whatsapp.token", "wa-token");
    setting(&site, &t, "notify.whatsapp.phone_id", "1234");
    block_on(crate::cron::run(&site.env(), "alpha", site.now_ms + 120_000));
    let got = bodies("/1234/messages");
    assert_eq!(got.len(), 1, "{got:?}");
    assert_eq!(got[0]["type"].as_str(), Some("template"), "{}", got[0]);
    assert_eq!(got[0]["template"]["name"].as_str(), Some("welcome_back"), "{}", got[0]);
    assert!(got[0]["to"].as_str().is_some_and(|to| to.contains("691112227")), "{}", got[0]);
    assert!(waiting(&site).iter().all(|e| e.kind != kind));
}
