//! The card path through the routes (W-COV C2): a card order at a venue with no Stripe key is
//! placed and told so; Stripe's signed webhook marks it paid ONCE; a bad signature, a stale
//! timestamp and an order nobody has are each answered without touching the log.

use crate::edge::site::{post, As, Site, PLATFORM_HOST, STRIPE_WEBHOOK_SECRET};
use crate::storefront::route_tests::open_venue;
use hmac::{Hmac, Mac};
use serde_json::json;

fn signed(payload: &str, t: i64) -> String {
    let mut mac = Hmac::<sha2::Sha256>::new_from_slice(STRIPE_WEBHOOK_SECRET.as_bytes()).unwrap();
    mac.update(format!("{t}.{payload}").as_bytes());
    let sig: String = mac.finalize().into_bytes().iter().map(|b| format!("{b:02x}")).collect();
    format!("t={t},v1={sig}")
}

fn hook(site: &Site, payload: &str, sig: &str) -> crate::wire::Reply {
    site.run(
        crate::stripe::webhook,
        crate::wire::Call::new(&format!("https://alpha.{PLATFORM_HOST}/api/webhooks/stripe"), worker::Method::Post)
            .unwrap()
            .with_body(payload.as_bytes().to_vec())
            .with_header("stripe-signature", sig)
            .on("alpha"),
        &[],
    )
}

#[test]
fn a_card_order_is_paid_once_by_a_signed_webhook() {
    // No publishable key: a card order is refused before anything is written.
    let bare = Site::new();
    let (_t, dish) = open_venue(&bare, "alpha", "a@x.test");
    let card = json!({"items": [{"product_id": dish, "quantity": 1}], "contact": {"name": "G", "phone": "+355690000004"},
                      "fulfilment": {"kind": "pickup"}, "payment": "card"});
    let r = bare.run(
        crate::storefront::place,
        post(&format!("https://alpha.{PLATFORM_HOST}/api/public/locations/alpha/orders"), &card).on("alpha"),
        &[("slug", "alpha")],
    );
    assert_eq!(r.status_code(), 409, "{}", r.body_str());
    assert!(r.body_str().contains("card payments are not configured"), "{}", r.body_str());

    // A publishable key but no secret key: the order is placed and told the card
    // provider is not configured; the webhook then settles it.
    let site = Site::with_secrets(&[("STRIPE_PUBLISHABLE_KEY", "pk_test_1")]);
    let (_t, dish) = open_venue(&site, "alpha", "a@x.test");
    let placed = site.run(
        crate::storefront::place,
        post(
            &format!("https://alpha.{PLATFORM_HOST}/api/public/locations/alpha/orders"),
            &json!({"items": [{"product_id": dish, "quantity": 1}], "contact": {"name": "G", "phone": "+355690000004"},
                    "fulfilment": {"kind": "pickup"}, "payment": "card"}),
        )
        .on("alpha"),
        &[("slug", "alpha")],
    );
    assert_eq!(placed.status_code(), 200, "{}", placed.body_str());
    assert_eq!(placed.body_value()["payment_error"], "card payments are not configured");
    let id = placed.body_value()["id"].as_str().unwrap().to_string();

    let now_s = site.now_ms / 1000;
    let ev = json!({"id": "evt_1", "type": "payment_intent.succeeded",
                    "data": {"object": {"id": "pi_1", "amount_received": 900, "metadata": {"order_id": id}}}})
        .to_string();
    let gen = site.fold("alpha", "/fold/generation").body_value();
    assert_eq!(hook(&site, &ev, "t=1,v1=00").status_code(), 400, "a bad signature");
    assert_eq!(hook(&site, &ev, &signed(&ev, now_s - 3600)).status_code(), 400, "a stale timestamp");
    assert_eq!(site.fold("alpha", "/fold/generation").body_value(), gen, "neither touched the log");

    let r = hook(&site, &ev, &signed(&ev, now_s));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(r.body_value()["applied"], id.as_str());
    let o = site.fold("alpha", &format!("/fold/order?id={id}")).body_str();
    assert!(o.contains("\\\"payment_status\\\":\\\"paid\\\"") || o.contains("paid"), "{o}");
    let again = hook(&site, &ev, &signed(&ev, now_s));
    assert_eq!(again.body_value()["duplicate"], id.as_str(), "{}", again.body_str());

    let ghost = json!({"id": "evt_2", "type": "payment_intent.succeeded", "data": {"object": {"id": "pi_2", "metadata": {"order_id": "ghost"}}}}).to_string();
    let r = hook(&site, &ghost, &signed(&ghost, now_s));
    assert_eq!(r.status_code(), 200);
    assert!(r.body_value()["unapplied"].is_string(), "{}", r.body_str());
    let other = json!({"id": "evt_3", "type": "charge.refunded", "data": {"object": {}}}).to_string();
    assert_eq!(hook(&site, &other, &signed(&other, now_s)).body_value()["ignored"], "charge.refunded");
}
