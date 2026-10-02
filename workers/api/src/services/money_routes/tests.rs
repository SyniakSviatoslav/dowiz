//! Promotions, campaigns, API keys, refunds, the aggregator, feedback -- through the routes
//! (W-COV C2), each read back from what was stored and each refused to the other venue.

use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::{open_venue, place_pickup};
use serde_json::{json, Value};

fn at(slug: &str, path: &str) -> String {
    format!("https://{slug}.{PLATFORM_HOST}{path}")
}

fn promo(site: &Site, t: &str, body: Value) -> crate::wire::Reply {
    site.run(crate::services::ordering::promotions::set_promotion, post(&at("alpha", "/api/owner/promotions"), &body).bearer(t).on("alpha"), &[])
}

#[test]
fn a_promotion_is_made_checked_redeemed_at_the_storefront_and_deleted() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    let r = promo(&site, &t, json!({"code": "SAVE10", "kind": "percent", "value": 10, "active": true}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let bad = promo(&site, &t, json!({"code": "BAD", "kind": "percent", "value": 150}));
    assert_eq!(bad.status_code(), 400, "150% is refused: {}", bad.body_str());
    let list = site.run(crate::services::ordering::promotions::promotions, get(&at("alpha", "/api/owner/promotions")).bearer(&t).on("alpha"), &[]);
    assert!(list.body_str().contains("SAVE10"), "{}", list.body_str());

    let check = site.run(
        crate::services::ordering::preview::promo_check,
        post(&at("alpha", "/api/promo/check"), &json!({"code": "SAVE10", "items": [{"product_id": dish, "quantity": 2}]})).on("alpha"),
        &[],
    );
    assert_eq!(check.status_code(), 200, "{}", check.body_str());
    assert!(check.body_str().contains("180"), "10% of 1800: {}", check.body_str());
    let nope = site.run(
        crate::services::ordering::preview::promo_check,
        post(&at("alpha", "/api/promo/check"), &json!({"code": "NOPE", "items": [{"product_id": dish, "quantity": 1}]})).on("alpha"),
        &[],
    );
    assert!(nope.status_code() >= 400 || nope.body_str().contains("false"), "{}", nope.body_str());

    let placed = site.run(
        crate::storefront::place,
        post(
            &at("alpha", "/api/public/locations/alpha/orders"),
            &json!({"items": [{"product_id": dish, "quantity": 2}], "contact": {"name": "G", "phone": "+355690000003"},
                    "fulfilment": {"kind": "pickup"}, "payment": "cash", "promo": "SAVE10"}),
        )
        .on("alpha"),
        &[("slug", "alpha")],
    );
    assert_eq!(placed.status_code(), 200, "{}", placed.body_str());
    assert_eq!(placed.body_value()["total"], 1620, "the stored total is after the discount: {}", placed.body_str());

    let b = site.venue("beta", "b@x.test");
    let r = site.run(
        crate::services::ordering::promotions::delete_promotion,
        post(&at("alpha", "/api/owner/promotions/SAVE10/delete?location_id=alpha"), &json!({})).bearer(&b).on("alpha"),
        &[("code", "SAVE10")],
    );
    assert!(r.status_code() >= 400, "beta deleted alpha's code: {}", r.body_str());
    let r = site.run(
        crate::services::ordering::promotions::delete_promotion,
        post(&at("alpha", "/api/owner/promotions/SAVE10/delete"), &json!({})).bearer(&t).on("alpha"),
        &[("code", "SAVE10")],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let list = site.run(crate::services::ordering::promotions::promotions, get(&at("alpha", "/api/owner/promotions")).bearer(&t).on("alpha"), &[]);
    assert!(!list.body_str().contains("\"SAVE10\"") || list.body_str().contains("false"), "{}", list.body_str());
}

#[test]
fn a_campaign_is_defined_previewed_reported_and_a_send_needs_confirmation() {
    let site = Site::new();
    let (t, _) = open_venue(&site, "alpha", "a@x.test");
    let r = site.run(
        crate::services::campaigns::handlers::define,
        post(&at("alpha", "/api/owner/campaigns"), &json!({"name": "Welcome back", "text": "We miss you", "segment": {"kind": "not_seen_since", "days": 30}}))
            .bearer(&t)
            .on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let id = r.body_value()["campaign"]["id"].as_str().expect("id").to_string();
    let bad = site.run(
        crate::services::campaigns::handlers::define,
        post(&at("alpha", "/api/owner/campaigns"), &json!({"name": "x", "text": "y", "segment": {"kind": "everyone_ever"}})).bearer(&t).on("alpha"),
        &[],
    );
    assert_eq!(bad.status_code(), 400, "an unknown segment: {}", bad.body_str());
    let preview = |site: &Site| {
        site.run(
            crate::services::campaigns::handlers::preview,
            post(&at("alpha", &format!("/api/owner/campaigns/{id}/preview")), &json!({})).bearer(&t).on("alpha"),
            &[("id", &id)],
        )
    };
    // Free text is not a WhatsApp campaign: nothing previews or sends until a template is named.
    assert_eq!(preview(&site).status_code(), 409);
    let r = site.run(
        crate::services::campaigns::handlers::define,
        post(
            &at("alpha", "/api/owner/campaigns"),
            &json!({"id": id, "name": "Welcome back", "text": "We miss you", "segment": {"kind": "everyone_consented"},
                    "template": {"name": "welcome_back", "lang": "sq"}}),
        )
        .bearer(&t)
        .on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let p = preview(&site);
    assert_eq!(p.status_code(), 200, "{}", p.body_str());
    let rep = site.run(crate::services::campaigns::handlers::report, get(&at("alpha", &format!("/api/owner/campaigns/{id}"))).bearer(&t).on("alpha"), &[("id", &id)]);
    assert_eq!(rep.status_code(), 200, "{}", rep.body_str());
    let unconfirmed = site.run(
        crate::services::campaigns::handlers::send_now,
        post(&at("alpha", &format!("/api/owner/campaigns/{id}/send")), &json!({"confirm": false})).bearer(&t).on("alpha"),
        &[("id", &id)],
    );
    assert_eq!(unconfirmed.status_code(), 400);
    let sent = site.run(
        crate::services::campaigns::handlers::send_now,
        post(&at("alpha", &format!("/api/owner/campaigns/{id}/send")), &json!({"confirm": true})).bearer(&t).on("alpha"),
        &[("id", &id)],
    );
    assert_eq!(sent.status_code(), 200, "{}", sent.body_str());
    let b = site.venue("beta", "b@x.test");
    let r = site.run(crate::services::campaigns::handlers::report, get(&at("alpha", &format!("/api/owner/campaigns/{id}?location_id=alpha"))).bearer(&b).on("alpha"), &[("id", &id)]);
    assert!(r.status_code() >= 400, "{}", r.body_str());
}

#[test]
fn an_api_key_authenticates_until_it_is_revoked() {
    let site = Site::new();
    let (t, _) = open_venue(&site, "alpha", "a@x.test");
    let r = site.run(
        crate::services::identity::keys::create_api_key,
        post(&at("alpha", "/api/owner/apikeys"), &json!({"label": "POS"})).bearer(&t).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let v = r.body_value();
    let key = ["key", "apiKey", "token", "secret"].iter().find_map(|k| v[*k].as_str()).unwrap_or_else(|| panic!("{v}")).to_string();
    let id = v["id"].as_str().map(str::to_string);
    assert!(key.starts_with("dowiz_"), "{key}");
    let read = site.run(crate::owner::orders, get(&at("alpha", "/api/owner/orders")).bearer(&key).on("alpha"), &[]);
    assert!(read.status_code() < 500, "{}", read.body_str());
    let list = site.run(crate::services::identity::keys::list_api_keys, get(&at("alpha", "/api/owner/apikeys")).bearer(&t).on("alpha"), &[]);
    assert!(list.body_str().contains("POS"), "{}", list.body_str());
    let id = id.or_else(|| list.body_value().to_string().split("\"id\":\"").nth(1).map(|s| s.split('"').next().unwrap_or("").to_string())).expect("key id");
    let r = site.run(
        crate::services::identity::keys::revoke_api_key,
        post(&at("alpha", "/api/owner/apikeys/revoke"), &json!({"id": id})).bearer(&t).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let after = crate::edge::mem::block_on(crate::auth::authenticate_token(&key, &site.env(), site.now_ms));
    assert!(after.is_err(), "a revoked key still authenticates");
}

#[test]
fn a_refund_needs_the_right_staff_and_moves_the_order() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    let manager = site.staff("alpha", &t, "m@x.test", "counter-manager");
    let waiter = site.staff("alpha", &t, "w@x.test", "waiter");
    let placed = place_pickup(&site, "alpha", &dish, 1);
    let id = placed.body_value()["id"].as_str().expect("id").to_string();
    // Refundable from PREPARING, READY or IN_DELIVERY (command/refund.rs); a collected
    // pickup is not.
    for a in ["confirm", "preparing", "ready"] {
        site.run(
            crate::owner::order_action,
            post(&at("alpha", &format!("/api/owner/orders/{id}/action")), &json!({"location_id": "alpha", "action": a})).bearer(&t).on("alpha"),
            &[("id", &id)],
        );
    }
    let refund_for = |tok: &str, reason: &str| {
        site.run(
            crate::services::orders::refund::refund,
            post(&at("alpha", &format!("/api/staff/orders/{id}/refund")), &json!({"location_id": "alpha", "reason": reason}))
                .bearer(tok)
                .on("alpha"),
            &[("id", &id)],
        )
    };
    let refund = |tok: &str| refund_for(tok, "customer_request");
    let by_waiter = refund(&waiter);
    assert!(by_waiter.status_code() == 401 || by_waiter.status_code() == 403, "a waiter refunded: {}", by_waiter.body_str());
    let before = site.fold("alpha", &format!("/fold/order?id={id}")).body_str();
    assert!(!before.contains("REFUND"), "the waiter's refusal wrote nothing: {before}");
    let bogus = refund_for(&manager, "cold");
    assert_eq!(bogus.status_code(), 400, "only a RefundReason word starts a refund: {}", bogus.body_str());
    let by_manager = refund(&manager);
    assert_eq!(by_manager.status_code(), 200, "{}", by_manager.body_str());
    let o = site.fold("alpha", &format!("/fold/order?id={id}")).body_str();
    assert!(o.contains("REFUND"), "{o}");
}

#[test]
fn an_aggregator_order_is_entered_once_and_feedback_needs_the_orders_key() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    let waiter = site.staff("alpha", &t, "w@x.test", "waiter");
    let entry = json!({"location_id": "alpha", "channel": "wolt", "external_id": "W-1", "lines": [{"product_id": dish, "quantity": 1, "unit_price": 1000}], "total": 1000});
    let first = site.run(crate::services::orders::aggregator::enter, post(&at("alpha", "/api/staff/orders/aggregator"), &entry).bearer(&waiter).on("alpha"), &[]);
    assert_eq!(first.status_code(), 200, "{}", first.body_str());
    let again = site.run(crate::services::orders::aggregator::enter, post(&at("alpha", "/api/staff/orders/aggregator"), &entry).bearer(&waiter).on("alpha"), &[]);
    assert_eq!(again.status_code(), 200, "{}", again.body_str());
    assert!(again.body_str().contains("true"), "the second entry is the existing one: {}", again.body_str());
    let gen = site.fold("alpha", "/fold/generation").body_value();

    let placed = place_pickup(&site, "alpha", &dish, 1);
    let v = placed.body_value();
    let (id, key) = (v["id"].as_str().unwrap().to_string(), v["access_token"].as_str().unwrap_or("").to_string());
    let anon = site.run(crate::services::orders::feedback::feedback, post(&at("alpha", &format!("/api/order/{id}/feedback")), &json!({"text": "tasty"})).on("alpha"), &[("id", &id)]);
    assert!(anon.status_code() >= 400, "{}", anon.body_str());
    let own = site.run(
        crate::services::orders::feedback::feedback,
        post(&at("alpha", &format!("/api/order/{id}/feedback")), &json!({"text": "tasty"})).bearer(&key).on("alpha"),
        &[("id", &id)],
    );
    assert!(own.status_code() < 500, "{}", own.body_str());
    let _ = gen;
}

#[test]
fn the_wallet_legs_audit_holds_on_a_clean_venue_and_a_repair_writes_nothing_twice() {
    // The rules (`command::pay::legs`, `refund::wallet::hand_back`) are tested natively; this is
    // their transport: the owner's read, a dry run by default, the apply, and the doors.
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    let (b, _) = open_venue(&site, "beta", "b@x.test");
    assert_eq!(place_pickup(&site, "alpha", &dish, 2).status_code(), 200);
    let audit = |tok: &str| site.run(crate::services::orders::legs::audit, get(&at("alpha", "/api/owner/wallet/legs?location_id=alpha")).bearer(tok).on("alpha"), &[]);
    let r = audit(&t);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let v = r.body_value();
    assert_eq!((v["venue"].as_str(), v["holds"].as_bool()), (Some("alpha"), Some(true)), "{v}");
    assert_eq!(r.headers().get("cache-control").unwrap().as_deref(), Some("private, no-store"));
    let repair = |tok: &str, body: &str| {
        site.run(
            crate::services::orders::legs::repair,
            crate::wire::Call::new(&at("alpha", "/api/owner/wallet/legs/repair?location_id=alpha"), worker::Method::Post).unwrap().with_body(body.as_bytes().to_vec()).bearer(tok).on("alpha"),
            &[],
        )
    };
    let dry = repair(&t, "");
    assert_eq!(dry.status_code(), 200, "{}", dry.body_str());
    assert_eq!((dry.body_value()["applied"].as_bool(), dry.body_value()["wouldWrite"].as_array().map(Vec::len)), (Some(false), Some(0)));
    let applied = repair(&t, r#"{"apply": true}"#);
    assert_eq!(applied.status_code(), 200, "{}", applied.body_str());
    assert_eq!((applied.body_value()["applied"].as_bool(), applied.body_value()["written"].as_array().map(Vec::len)), (Some(true), Some(0)));
    assert_eq!(repair(&t, r#"{"apply": "yes"}"#).status_code(), 400);
    assert_eq!(repair(&t, r#"{"force": true}"#).status_code(), 400, "an unknown field is refused");
    // Beta's owner neither reads nor repairs alpha's ledger.
    assert!(audit(&b).status_code() >= 400);
    assert!(repair(&b, r#"{"apply": true}"#).status_code() >= 400);
}

#[test]
fn a_refund_past_the_venues_threshold_queues_one_exception_alert_per_level() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    let manager = site.staff("alpha", &t, "m@x.test", "counter-manager");
    let set = |k: &str, v: &str| {
        let r = site.run(crate::services::venue::settings::set_setting, post(&at("alpha", "/api/owner/settings"), &json!({"key": k, "value": v})).bearer(&t).on("alpha"), &[]);
        assert_eq!(r.status_code(), 200, "{k}: {}", r.body_str());
    };
    let refund_one = || {
        let id = place_pickup(&site, "alpha", &dish, 1).body_value()["id"].as_str().unwrap().to_string();
        for a in ["confirm", "preparing"] {
            site.run(
                crate::owner::order_action,
                post(&at("alpha", &format!("/api/owner/orders/{id}/action")), &json!({"location_id": "alpha", "action": a})).bearer(&t).on("alpha"),
                &[("id", &id)],
            );
        }
        let r = site.run(
            crate::services::orders::refund::refund,
            post(&at("alpha", &format!("/api/staff/orders/{id}/refund")), &json!({"location_id": "alpha", "reason": "customer_request"})).bearer(&manager).on("alpha"),
            &[("id", &id)],
        );
        assert_eq!(r.status_code(), 200, "{}", r.body_str());
    };
    let alerts = || -> Vec<crate::outbox::Entry> {
        let place = crate::hubstore::Place::of_authorised(&site.ctx(&[]), "alpha").unwrap();
        crate::edge::mem::block_on(crate::outbox::waiting(&place)).unwrap().into_iter().filter(|e| e.id.starts_with("exceptions/")).collect()
    };
    // No chat configured: nobody to tell, nothing queued.
    refund_one();
    assert!(alerts().is_empty());
    set(crate::notify::route::groups::LEGACY_CHAT, "-5005");
    set(crate::exceptions::alert::THRESHOLD_KEY, "2");
    // The count crosses 2 on this refund (the first one counts too): one alert, to the chat.
    refund_one();
    let a = alerts();
    assert_eq!(a.len(), 1, "{a:?}");
    assert_eq!(a[0].to, "-5005");
    assert!(a[0].id.ends_with("/2"), "{}", a[0].id);
    // The third stays under the next level; the fourth reaches it.
    refund_one();
    assert_eq!(alerts().len(), 1);
    refund_one();
    let a = alerts();
    assert_eq!(a.len(), 2, "{a:?}");
    assert!(a.iter().any(|e| e.id.ends_with("/4")), "{a:?}");
}

#[test]
fn resetting_the_ingredients_needs_the_venues_id_repeated_and_clears_allergens_and_supplies_only() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    let (b, _) = open_venue(&site, "beta", "b@x.test");
    let reset = |tok: &str, confirm: &str| {
        site.run(
            crate::services::operations::ingredients_reset::reset_ingredients,
            post(&at("alpha", "/api/owner/ingredients/reset"), &json!({"confirm": confirm, "location_id": "alpha"})).bearer(tok).on("alpha"),
            &[],
        )
    };
    // The open_venue dish declared its allergens (an empty list).
    assert_eq!(reset(&t, "beta").status_code(), 400, "the confirmation must name this venue");
    assert!(reset(&b, "alpha").status_code() >= 400, "another venue's owner resets nothing");
    let r = reset(&t, "alpha");
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let v = r.body_value();
    assert_eq!((v["allergens"].as_u64(), v["supplies"].as_u64()), (Some(1), Some(0)), "{v}");
    // The dish stays; only what was declared about it is gone.
    let menu = site.run(crate::storefront::menu, get(&at("alpha", "/api/public/locations/alpha/menu?fresh=1")).on("alpha"), &[("slug", "alpha")]).body_str();
    assert!(menu.contains(&dish) || menu.contains("Futomaki"), "{menu}");
    // A second reset finds nothing to clear.
    assert_eq!(reset(&t, "alpha").body_value()["allergens"], 0);
}

#[test]
fn a_stamp_card_counts_a_customers_orders_and_shows_only_to_the_orders_own_key() {
    let site = Site::new();
    let (t, dish) = open_venue(&site, "alpha", "a@x.test");
    let set = |k: &str, v: &str| {
        let r = site.run(crate::services::venue::settings::set_setting, post(&at("alpha", "/api/owner/settings"), &json!({"key": k, "value": v})).bearer(&t).on("alpha"), &[]);
        assert_eq!(r.status_code(), 200, "{k}: {}", r.body_str());
    };
    let card = |id: &str, key: &str| {
        site.run(crate::services::loyalty::handlers::order_stamps, get(&at("alpha", &format!("/api/order/{id}/stamps"))).bearer(key).on("alpha"), &[("id", id)])
    };
    // No card: `on:false`.
    let v = place_pickup(&site, "alpha", &dish, 1).body_value();
    let (id0, key0) = (v["id"].as_str().unwrap().to_string(), v["access_token"].as_str().unwrap().to_string());
    assert_eq!(card(&id0, &key0).body_value()["on"], false);
    set(crate::services::loyalty::stamps::STAMPS_N, "3");
    set(crate::services::loyalty::stamps::REWARD, "500");
    set(crate::services::loyalty::stamps::ENABLED, "1");
    // Two more orders from the same phone: each one is a stamp.
    let mut last = (String::new(), String::new());
    for _ in 0..2 {
        let v = place_pickup(&site, "alpha", &dish, 1).body_value();
        last = (v["id"].as_str().unwrap().to_string(), v["access_token"].as_str().unwrap().to_string());
    }
    let c = card(&last.0, &last.1);
    assert_eq!(c.status_code(), 200, "{}", c.body_str());
    let v = c.body_value();
    assert_eq!((v["on"].as_bool(), v["n"].as_i64(), v["reward"].as_i64()), (Some(true), Some(3), Some(500)), "{v}");
    assert!(v["have"].as_i64().unwrap() >= 1, "{v}");
    // Another order's key, or none, does not open this card.
    assert_eq!(card(&last.0, &key0).status_code(), 401);
    assert_eq!(card(&last.0, &t).status_code(), 401, "even the owner's token is not this order's key");
    assert_eq!(card("ghost", &last.1).status_code(), 401);
}
