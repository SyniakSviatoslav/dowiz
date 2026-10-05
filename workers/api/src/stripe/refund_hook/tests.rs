//! W-REFUND, END TO END IN MEMORY (tested in-memory, NOT live — the live
//! probe is `tools/live-proof/probes/feature-card-refund.mjs`): a card order
//! paid through the signed webhook, refunded half and then the rest through
//! the staff route, each card refund sent by the venue's drain to a fake
//! Stripe at the edge with its Idempotency-Key, and Stripe's signed
//! `refund.updated` closing the order only when the whole card part is back.

use crate::edge::mem::{answer_outbound, block_on, sent};
use crate::edge::site::{post, As, Site, PLATFORM_HOST, STRIPE_WEBHOOK_SECRET};
use crate::storefront::route_tests::open_venue;
use crate::wire::{Call, Reply};
use hmac::{Hmac, Mac};
use serde_json::{json, Value};

fn at(path: &str) -> String {
    format!("https://alpha.{PLATFORM_HOST}{path}")
}

fn signed(payload: &str, t: i64) -> String {
    let mut mac = Hmac::<sha2::Sha256>::new_from_slice(STRIPE_WEBHOOK_SECRET.as_bytes()).unwrap();
    mac.update(format!("{t}.{payload}").as_bytes());
    let sig: String = mac.finalize().into_bytes().iter().map(|b| format!("{b:02x}")).collect();
    format!("t={t},v1={sig}")
}

fn hook(site: &Site, ev: &Value) -> Reply {
    let payload = ev.to_string();
    hook_signed(site, &payload, &signed(&payload, site.now_ms / 1000))
}

fn hook_signed(site: &Site, payload: &str, sig: &str) -> Reply {
    let call = Call::new(&at("/api/webhooks/stripe"), worker::Method::Post).unwrap().with_body(payload.as_bytes().to_vec());
    site.run(crate::stripe::webhook, call.with_header("stripe-signature", sig).on("alpha"), &[])
}

fn order(site: &Site, id: &str) -> Value {
    let v = site.fold("alpha", &format!("/fold/order?id={id}")).body_value();
    serde_json::from_str(v["order_json"].as_str().unwrap_or("null")).unwrap_or(Value::Null)
}

fn refund(site: &Site, t: &str, id: &str, body: Value) -> Reply {
    let call = post(&at(&format!("/api/staff/orders/{id}/refund")), &body).bearer(t).on("alpha");
    site.run(crate::services::orders::refund::refund, call, &[("id", id)])
}

/// Stripe as the edge sees it: an intent, and every refund `pending`.
fn fake_stripe() {
    answer_outbound(|c| {
        let path = c.url().unwrap().path().to_string();
        if path.ends_with("/v1/payment_intents") {
            return Reply::from_json(&json!({"id": "pi_1", "client_secret": "cs_1"}));
        }
        let n = sent().iter().filter(|s| s.url().unwrap().path().ends_with("/v1/refunds")).count();
        Reply::from_json(&json!({"id": format!("re_{n}"), "object": "refund", "status": "pending"}))
    });
}

fn refunds_sent() -> Vec<Call> {
    sent().into_iter().filter(|c| c.url().unwrap().path().ends_with("/v1/refunds")).collect()
}

fn refund_event(evt: &str, id: &str, re: &str, status: &str, amount: i64, venue: &str, key: &str) -> Value {
    json!({"id": evt, "type": "refund.updated", "data": {"object": {
        "id": re, "object": "refund", "status": status, "amount": amount, "currency": "all", "payment_intent": "pi_1",
        "metadata": {"order_id": id, "venue": venue, "key": key}}}})
}

/// A card order of 2 x 900 lek, paid: the intent was asked for 180000 of
/// Stripe's units, and the webhook's 180000 was written back as 1800 lek.
fn paid_card_order(site: &Site, t: &str, dish: &str) -> String {
    let body = json!({"items": [{"product_id": dish, "quantity": 2}], "contact": {"name": "G", "phone": "+355690000004"},
                      "fulfilment": {"kind": "pickup"}, "payment": "card"});
    let r = site.run(crate::storefront::place, post(&at("/api/public/locations/alpha/orders"), &body).on("alpha"), &[("slug", "alpha")]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let id = r.body_value()["id"].as_str().unwrap().to_string();
    let intent = sent().into_iter().find(|c| c.url().unwrap().path().ends_with("/v1/payment_intents")).expect("an intent");
    let form = String::from_utf8(intent.body_bytes()).unwrap();
    assert!(form.starts_with("amount=180000&currency=all"), "1800 lek is 180000 to Stripe: {form}");
    let paid = json!({"id": "evt_paid", "type": "payment_intent.succeeded",
        "data": {"object": {"id": "pi_1", "amount_received": 180000, "currency": "all", "metadata": {"order_id": id}}}});
    assert_eq!(hook(site, &paid).status_code(), 200);
    assert_eq!(order(site, &id)["amount_received"], json!(1800), "Stripe's units back into lek");
    let c = post(&at(&format!("/api/owner/orders/{id}/action")), &json!({"location_id": "alpha", "action": "confirm"})).bearer(t).on("alpha");
    assert_eq!(site.run(crate::owner::order_action, c, &[("id", &id)]).status_code(), 200);
    id
}

#[test]
fn half_then_the_rest_goes_back_to_the_card_and_only_then_the_order_ends() {
    let site = Site::with_secrets(&[("STRIPE_PUBLISHABLE_KEY", "pk_test_1"), ("STRIPE_SECRET_KEY", "sk_test_1")]);
    fake_stripe();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    let id = paid_card_order(&site, &t, &dish);

    // HALF: queued beside the event, sent by the drain with the attempt's key.
    let r = refund(&site, &t, &id, json!({"location_id": "alpha", "reason": "customer_request", "card_amount": 900}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let o = order(&site, &id);
    assert_eq!(o["status"], json!("REFUNDING"));
    let key1 = o["refund"]["card"]["attempts"][0]["key"].as_str().unwrap().to_string();
    assert_eq!(key1, crate::command::refund::card::idem_key("alpha", &id, 1));
    block_on(crate::cron::run(&site.env(), "alpha", site.now_ms + 60_000));
    let out = refunds_sent();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].headers().get("idempotency-key").unwrap().as_deref(), Some(key1.as_str()));
    assert_eq!(out[0].headers().get("stripe-version").unwrap().as_deref(), Some(crate::stripe::refund::STRIPE_VERSION));
    assert!(String::from_utf8(out[0].body_bytes()).unwrap().starts_with("payment_intent=pi_1&amount=90000&"));
    let o = order(&site, &id);
    assert_eq!((o["refund"]["card"]["attempts"][0]["status"].as_str(), o["refund"]["card"]["attempts"][0]["id"].as_str()), (Some("pending"), Some("re_1")));
    // The drain ran again: the sent entry is gone, nothing is sent twice.
    block_on(crate::cron::run(&site.env(), "alpha", site.now_ms + 180_000));
    assert_eq!(refunds_sent().len(), 1, "a sent refund was sent again");

    // A FOREIGN VENUE's event and a BAD SIGNATURE change nothing.
    let before = order(&site, &id);
    let foreign = hook(&site, &refund_event("evt_x", &id, "re_1", "succeeded", 90000, "beta", &key1));
    assert_eq!(foreign.status_code(), 200, "final, so Stripe stops: {}", foreign.body_str());
    assert!(foreign.body_str().contains("refused"), "{}", foreign.body_str());
    let ev = refund_event("evt_1", &id, "re_1", "succeeded", 90000, "alpha", &key1).to_string();
    assert_eq!(hook_signed(&site, &ev, "t=1,v1=00").status_code(), 400, "a stale timestamp");
    // A FRESH timestamp with a wrong signature: the HMAC itself is what refuses it.
    let forged = format!("t={},v1={}", site.now_ms / 1000, "0".repeat(64));
    assert_eq!(hook_signed(&site, &ev, &forged).status_code(), 400, "a wrong signature");
    assert_eq!(order(&site, &id), before, "neither touched the order");

    // Stripe says succeeded for the half: recorded, the order still REFUNDING.
    let ok = hook(&site, &serde_json::from_str(&ev).unwrap());
    assert_eq!(ok.status_code(), 200, "{}", ok.body_str());
    assert_eq!(order(&site, &id)["status"], json!("REFUNDING"), "half the card is not the card part");
    let again = hook(&site, &serde_json::from_str(&ev).unwrap());
    assert!(again.body_str().contains("duplicate"), "{}", again.body_str());

    // THE REST, from the sheet's "refund to card" on a REFUNDING order.
    let r = refund(&site, &t, &id, json!({"location_id": "alpha", "card_amount": 901}));
    assert_eq!(r.status_code(), 400, "one lek over: {}", r.body_str());
    let r = refund(&site, &t, &id, json!({"location_id": "alpha", "card_amount": 900}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    block_on(crate::cron::run(&site.env(), "alpha", site.now_ms + 300_000));
    let out = refunds_sent();
    assert_eq!(out.len(), 2);
    let key2 = crate::command::refund::card::idem_key("alpha", &id, 2);
    assert_eq!(out[1].headers().get("idempotency-key").unwrap().as_deref(), Some(key2.as_str()));
    let done = hook(&site, &refund_event("evt_2", &id, "re_2", "succeeded", 90000, "alpha", &key2));
    assert_eq!(done.status_code(), 200, "{}", done.body_str());
    let o = order(&site, &id);
    assert_eq!(o["status"], json!("COMPENSATED_REFUND"), "{o}");
    assert_eq!(o["refund"]["returned"]["by"], json!("stripe"));
}

/// Stripe refuses: the attempt says why, the entry is not retried, and the
/// venue's audit (the health pane) names it.
#[test]
fn a_refund_stripe_refuses_is_failed_on_the_order_and_said_once() {
    let site = Site::with_secrets(&[("STRIPE_PUBLISHABLE_KEY", "pk_test_1"), ("STRIPE_SECRET_KEY", "sk_test_1")]);
    fake_stripe();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    let id = paid_card_order(&site, &t, &dish);
    answer_outbound(|_| Ok(Reply::from_json(&json!({"error": {"message": "charge_disputed"}}))?.with_status(400)));
    assert_eq!(refund(&site, &t, &id, json!({"location_id": "alpha", "reason": "customer_request"})).status_code(), 200);
    block_on(crate::cron::run(&site.env(), "alpha", site.now_ms + 60_000));
    let a = order(&site, &id)["refund"]["card"]["attempts"][0].clone();
    assert_eq!(a["status"], json!("failed"));
    assert!(a["failure"].as_str().unwrap().contains("charge_disputed"), "{a}");
    let place = crate::hubstore::Place::of_authorised(&site.ctx(&[]), "alpha").unwrap();
    assert!(block_on(crate::outbox::waiting(&place)).unwrap().iter().all(|e| e.kind != crate::stripe::refund::KIND), "a final refusal is not retried");
    block_on(crate::cron::run(&site.env(), "alpha", site.now_ms + 600_000));
    assert_eq!(refunds_sent().len(), 1, "sent once");
}

/// No secret key on this Worker: the card part is by hand, the record says
/// so, and a card amount is refused with the console's sentence.
#[test]
fn without_a_stripe_key_the_card_refund_is_by_hand() {
    let site = Site::with_secrets(&[("STRIPE_PUBLISHABLE_KEY", "pk_test_1")]);
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    let body = json!({"items": [{"product_id": dish, "quantity": 1}], "contact": {"name": "G", "phone": "+355690000004"},
                      "fulfilment": {"kind": "pickup"}, "payment": "card"});
    let r = site.run(crate::storefront::place, post(&at("/api/public/locations/alpha/orders"), &body).on("alpha"), &[("slug", "alpha")]);
    let id = r.body_value()["id"].as_str().unwrap().to_string();
    let paid = json!({"id": "evt_p", "type": "payment_intent.succeeded", "data": {"object": {"id": "pi_1", "amount_received": 90000, "currency": "all", "metadata": {"order_id": id}}}});
    assert_eq!(hook(&site, &paid).status_code(), 200);
    let c = post(&at(&format!("/api/owner/orders/{id}/action")), &json!({"location_id": "alpha", "action": "confirm"})).bearer(&t).on("alpha");
    site.run(crate::owner::order_action, c, &[("id", &id)]);
    let r = refund(&site, &t, &id, json!({"location_id": "alpha", "reason": "customer_request", "card_amount": 900}));
    assert_eq!(r.status_code(), 409, "{}", r.body_str());
    assert!(r.body_str().contains("refund the card by hand"), "{}", r.body_str());
    let r = refund(&site, &t, &id, json!({"location_id": "alpha", "reason": "customer_request"}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let o = order(&site, &id);
    assert_eq!((o["status"].as_str(), o["refund"]["card"]["manual"].as_bool()), (Some("REFUNDING"), Some(true)));
    let done = refund(&site, &t, &id, json!({"location_id": "alpha", "complete": true}));
    assert_eq!(done.status_code(), 200, "the existing manual path: {}", done.body_str());
    assert_eq!(order(&site, &id)["status"], json!("COMPENSATED_REFUND"));
}
