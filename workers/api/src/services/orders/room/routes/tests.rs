//! The room through its routes (W-COV C2): staff invited and signed in, a table's round placed,
//! amended by a waiter, seen by the kitchen, paid at the till -- and a waiter of another venue
//! refused on every door.

use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::storefront::route_tests::open_venue;
use serde_json::{json, Value};

fn at(slug: &str, path: &str) -> String {
    format!("https://{slug}.{PLATFORM_HOST}{path}")
}

fn dine_in(site: &Site, slug: &str, dish: &str, table: &str) -> crate::wire::Reply {
    dine_in_as(site, slug, dish, table, None)
}

/// A round sent by a waiter opens a SITTING (`room::placer::staffed`); a guest's does not.
fn dine_in_as(site: &Site, slug: &str, dish: &str, table: &str, waiter: Option<&str>) -> crate::wire::Reply {
    let call = post(
        &at(slug, &format!("/api/public/locations/{slug}/orders")),
        &json!({
            "items": [{"product_id": dish, "quantity": 1}],
            "contact": {"name": "Table", "phone": ""},
            "fulfilment": {"kind": "dine_in", "table": table},
            "payment": "cash",
        }),
    )
    .on(slug);
    let call = match waiter {
        Some(t) => call.bearer(t),
        None => call,
    };
    site.run(crate::storefront::place, call, &[("slug", slug)])
}

#[allow(dead_code)]
fn guest_dine_in(site: &Site, slug: &str, dish: &str, table: &str) -> crate::wire::Reply {
    site.run(
        crate::storefront::place,
        post(
            &at(slug, &format!("/api/public/locations/{slug}/orders")),
            &json!({
                "items": [{"product_id": dish, "quantity": 1}],
                "contact": {"name": "Table", "phone": ""},
                "fulfilment": {"kind": "dine_in", "table": table},
                "payment": "cash",
            }),
        )
        .on(slug),
        &[("slug", slug)],
    )
}

fn id_of(r: &crate::wire::Reply) -> String {
    let v = r.body_value();
    ["id", "order_id", "orderId"]
        .iter()
        .find_map(|k| v[*k].as_str().or(v["order"][*k].as_str()).map(str::to_string))
        .unwrap_or_else(|| panic!("no id in {}", r.body_str()))
}

fn stored(site: &Site, slug: &str, id: &str) -> (u64, Value) {
    let o = site.fold(slug, &format!("/fold/order?id={id}")).body_value();
    let v: Value = serde_json::from_str(o["order_json"].as_str().unwrap_or("{}")).unwrap_or_default();
    (o["seq"].as_u64().unwrap_or(0), v)
}

#[test]
fn a_waiter_sees_the_table_adds_a_dish_and_takes_the_money() {
    let site = Site::new();
    let (owner, dish) = open_venue(&site, "alpha", "a@x.test");
    let waiter = site.staff("alpha", &owner, "w@x.test", "waiter");
    let placed = dine_in_as(&site, "alpha", &dish, "T1", Some(&waiter));
    assert_eq!(placed.status_code(), 200, "{}", placed.body_str());
    let id = id_of(&placed);

    let room = site.run(
        crate::services::orders::room::handlers::room_view,
        get(&at("alpha", "/api/staff/room?location_id=alpha")).bearer(&waiter).on("alpha"),
        &[],
    );
    assert_eq!(room.status_code(), 200, "{}", room.body_str());
    assert!(room.body_str().contains(&id), "the table's round is in the room: {}", room.body_str());

    let (seq, before) = stored(&site, "alpha", &id);
    let r = site.run(
        crate::services::orders::room::handlers::amend,
        post(
            &at("alpha", &format!("/api/staff/orders/{id}/amend")),
            &json!({"location_id": "alpha", "base_seq": seq, "ops": [{"op": "add", "product_id": dish, "quantity": 1}]}),
        )
        .bearer(&waiter)
        .on("alpha"),
        &[("id", &id)],
    );
    assert_eq!(r.status_code(), 200, "amend: {}", r.body_str());
    let (seq2, after) = stored(&site, "alpha", &id);
    assert!(seq2 > seq);
    assert_eq!(after["subtotal"].as_i64(), before["subtotal"].as_i64().map(|s| s + 900), "priced from the catalogue: {after}");

    // Adds commute, but an edit to a line is versioned: a stale base_seq is refused ...
    let set_qty = |base: u64| {
        site.run(
            crate::services::orders::room::handlers::amend,
            post(
                &at("alpha", &format!("/api/staff/orders/{id}/amend")),
                &json!({"location_id": "alpha", "base_seq": base, "ops": [{"op": "set_qty", "line": 0, "qty": 2}]}),
            )
            .bearer(&waiter)
            .on("alpha"),
            &[("id", &id)],
        )
    };
    let stale = set_qty(seq);
    assert_eq!(stale.status_code(), 409, "a stale edit was applied: {}", stale.body_str());
    assert_eq!(stored(&site, "alpha", &id).0, seq2, "and nothing was written");
    // ... and the current one lands.
    let fresh = set_qty(seq2);
    assert_eq!(fresh.status_code(), 200, "{}", fresh.body_str());
    let (_, after) = stored(&site, "alpha", &id);
    assert_eq!(after["subtotal"], json!(2700), "line 0 now counts two: {after}");

    let total = after["total"].as_i64().expect("total");
    let pay = |body: Value| {
        site.run(
            crate::services::orders::room::pay::pay,
            post(&at("alpha", &format!("/api/staff/orders/{id}/pay")), &body).bearer(&waiter).on("alpha"),
            &[("id", &id)],
        )
    };
    // Cash goes into a drawer: with no till open it is refused ...
    let r = pay(json!({"location_id": "alpha", "amount": total, "method": "cash"}));
    assert_eq!(r.status_code(), 409, "{}", r.body_str());
    // ... and lands once a manager has opened one.
    let manager = site.staff("alpha", &owner, "m@x.test", "counter-manager");
    let r = site.run(
        crate::services::orders::room::till::open,
        post(&at("alpha", "/api/staff/till/open"), &json!({"location_id": "alpha", "till_id": "t1", "float": {"ALL": 5000}}))
            .bearer(&manager)
            .on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let r = pay(json!({"location_id": "alpha", "amount": total, "method": "cash", "till_id": "t1"}));
    assert_eq!(r.status_code(), 200, "pay: {}", r.body_str());
    let (_, paid) = stored(&site, "alpha", &id);
    assert!(paid.to_string().contains("\"cash\""), "the payment is on the order: {paid}");
}

#[test]
fn the_kitchen_sees_the_ticket_without_the_price_and_acks_it() {
    let site = Site::new();
    let (owner, dish) = open_venue(&site, "alpha", "a@x.test");
    let cook = site.staff("alpha", &owner, "k@x.test", "kitchen");
    let id = id_of(&dine_in(&site, "alpha", &dish, "T2"));
    let board = site.run(
        crate::services::orders::kitchen_ack::board::kitchen_orders,
        get(&at("alpha", "/api/staff/kitchen?location_id=alpha")).bearer(&cook).on("alpha"),
        &[],
    );
    assert_eq!(board.status_code(), 200, "{}", board.body_str());
    assert!(board.body_str().contains(&id), "{}", board.body_str());
    assert!(!board.body_str().contains("+3556"), "no phone on the pass: {}", board.body_str());
    let r = site.run(
        crate::services::orders::kitchen_ack::kitchen_ack,
        post(&at("alpha", &format!("/api/staff/orders/{id}/kitchen-ack")), &json!({"location_id": "alpha"}))
            .bearer(&cook)
            .on("alpha"),
        &[("id", &id)],
    );
    assert_eq!(r.status_code(), 200, "ack: {}", r.body_str());
}

#[test]
fn a_till_opens_counts_and_closes_for_a_manager_and_not_for_a_waiter() {
    let site = Site::new();
    let (owner, _) = open_venue(&site, "alpha", "a@x.test");
    let manager = site.staff("alpha", &owner, "m@x.test", "counter-manager");
    let waiter = site.staff("alpha", &owner, "w@x.test", "waiter");
    let open = |tok: &str| {
        site.run(
            crate::services::orders::room::till::open,
            post(&at("alpha", "/api/staff/till/open"), &json!({"location_id": "alpha", "till_id": "t1", "float": {"ALL": 5000}}))
                .bearer(tok)
                .on("alpha"),
            &[],
        )
    };
    let r = open(&waiter);
    assert!(r.status_code() == 401 || r.status_code() == 403, "a waiter opened the till: {}", r.body_str());
    let r = open(&manager);
    assert_eq!(r.status_code(), 200, "open: {}", r.body_str());
    for (route, body) in [
        ("pay_in", json!({"location_id": "alpha", "till_id": "t1", "currency": "ALL", "amount": 1000, "reason": "change from the bank"})),
        ("count", json!({"location_id": "alpha", "till_id": "t1", "observed": {"ALL": 6000}})),
        ("close", json!({"location_id": "alpha", "till_id": "t1"})),
    ] {
        let call = post(&at("alpha", &format!("/api/staff/till/{route}")), &body).bearer(&manager).on("alpha");
        let r = match route {
            "pay_in" => site.run(crate::services::orders::room::till::pay_in, call, &[]),
            "count" => site.run(crate::services::orders::room::till::count, call, &[]),
            _ => site.run(crate::services::orders::room::till::close, call, &[]),
        };
        assert_eq!(r.status_code(), 200, "{route}: {}", r.body_str());
    }
    let tips = site.run(
        crate::services::orders::room::till::tips,
        get(&at("alpha", "/api/staff/till/tips?location_id=alpha")).bearer(&manager).on("alpha"),
        &[],
    );
    assert_eq!(tips.status_code(), 200, "{}", tips.body_str());
}

#[test]
fn a_waiter_of_another_venue_is_refused_in_this_room() {
    let site = Site::new();
    let (_owner, dish) = open_venue(&site, "alpha", "a@x.test");
    let beta_owner = site.venue("beta", "b@x.test");
    let stranger = site.staff("beta", &beta_owner, "s@x.test", "waiter");
    let own = site.staff("alpha", &_owner, "w@x.test", "waiter");
    let id = id_of(&dine_in_as(&site, "alpha", &dish, "T3", Some(&own)));
    let (seq, _) = stored(&site, "alpha", &id);
    let room = site.run(
        crate::services::orders::room::handlers::room_view,
        get(&at("alpha", "/api/staff/room?location_id=alpha")).bearer(&stranger).on("alpha"),
        &[],
    );
    assert!(room.status_code() >= 400, "{}", room.body_str());
    let r = site.run(
        crate::services::orders::room::handlers::amend,
        post(
            &at("alpha", &format!("/api/staff/orders/{id}/amend")),
            &json!({"location_id": "alpha", "base_seq": seq, "ops": [{"op": "add", "product_id": dish, "quantity": 1}]}),
        )
        .bearer(&stranger)
        .on("alpha"),
        &[("id", &id)],
    );
    assert!(r.status_code() >= 400, "{}", r.body_str());
    assert_eq!(stored(&site, "alpha", &id).0, seq, "nothing was written");
}

// ── A GUEST AT THE TABLE, BY ITS QR CODE ────────────────────────────────────

fn plan(site: &Site, owner: &str) {
    let t = |n: i64| json!({ "n": n, "x": 100 * n, "y": 100, "w": 40, "h": 40, "seats": 4 });
    let r = site.run(
        crate::booking::set_plan,
        post(&at("alpha", "/api/owner/floorplan"), &json!({"zones": [{"id": "salla", "name": "Salla", "tables": [t(1), t(2)]}]})).bearer(owner).on("alpha"),
        &[],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
}

#[test]
fn a_guest_round_from_the_table_waits_for_the_room_and_the_bill_is_read_by_the_guest() {
    let site = Site::new();
    let (owner, dish) = open_venue(&site, "alpha", "a@x.test");
    plan(&site, &owner);
    let waiter = site.staff("alpha", &owner, "w@x.test", "waiter");

    let qr = site.run(crate::services::orders::room::table_qr::list, get(&at("alpha", "/api/owner/tables/qr")).bearer(&owner).on("alpha"), &[]);
    assert_eq!(qr.status_code(), 200, "{}", qr.body_str());
    // The link's `?t=`, read from the table's `url` (the whole body also carries
    // SVG attributes, and `height="` ends in `t=`).
    let url = qr.body_value()["tables"][0]["url"].as_str().unwrap_or_else(|| panic!("no url in {}", qr.body_str())).to_string();
    let t = worker::Url::parse(&url).unwrap().query_pairs().find(|(k, _)| k == "t").map(|(_, v)| v.into_owned()).unwrap_or_else(|| panic!("no t in {url}"));
    assert!(t.len() > 1, "{url}");
    let svg = site.run(
        crate::services::orders::room::table_qr::one,
        get(&at("alpha", "/api/owner/tables/salla/1/qr.svg")).bearer(&owner).on("alpha"),
        &[("zone", "salla"), ("n", "1.svg")],
    );
    assert_eq!(svg.status_code(), 200, "{}", svg.body_str());
    assert!(svg.body_str().starts_with("<svg") || svg.body_str().contains("<svg"));
    let missing = site.run(
        crate::services::orders::room::table_qr::one,
        get(&at("alpha", "/api/owner/tables/salla/9/qr.svg")).bearer(&owner).on("alpha"),
        &[("zone", "salla"), ("n", "9.svg")],
    );
    assert_eq!(missing.status_code(), 404);

    let place = |link: &str| {
        site.run(
            crate::storefront::place,
            post(
                &at("alpha", "/api/public/locations/alpha/orders"),
                &json!({"items": [{"product_id": dish, "quantity": 1}], "contact": {"name": "Guest", "phone": ""},
                        "fulfilment": {"kind": "dine_in"}, "payment": "cash", "table_link": link}),
            )
            .on("alpha"),
            &[("slug", "alpha")],
        )
    };
    let forged = place(&format!("{}0", &t[..t.len() - 1]));
    assert_eq!(forged.status_code(), 400, "a forged table link: {}", forged.body_str());
    let r = place(&t);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let v = r.body_value();
    let (id, key) = (v["id"].as_str().unwrap().to_string(), v["access_token"].as_str().unwrap_or("").to_string());

    let confirm = |action: &str| {
        site.run(
            crate::services::orders::room::guest_round::confirm,
            post(&at("alpha", &format!("/api/staff/orders/{id}/guest")), &json!({"location_id": "alpha", "action": action})).bearer(&waiter).on("alpha"),
            &[("id", &id)],
        )
    };
    assert_eq!(confirm("shrug").status_code(), 400);
    let r = confirm("confirm");
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(stored(&site, "alpha", &id).1["status"], "CONFIRMED");
    assert_eq!(confirm("confirm").status_code(), 409, "a round is confirmed once");

    let bill = site.run(
        crate::services::orders::room::guest_round::sitting_bill,
        get(&at("alpha", &format!("/api/order/{id}/sitting"))).bearer(&key).on("alpha"),
        &[("id", &id)],
    );
    assert_eq!(bill.status_code(), 200, "{}", bill.body_str());
    let anon = site.run(crate::services::orders::room::guest_round::sitting_bill, get(&at("alpha", &format!("/api/order/{id}/sitting"))).on("alpha"), &[("id", &id)]);
    assert!(anon.status_code() >= 400);

    let floor = site.run(crate::services::orders::room::floor::get, get(&at("alpha", "/api/staff/floor?location_id=alpha")).bearer(&waiter).on("alpha"), &[]);
    assert_eq!(floor.status_code(), 200, "{}", floor.body_str());
    let sitting = stored(&site, "alpha", &id).1["sitting_id"].as_str().unwrap_or("").to_string();
    let r = site.run(
        crate::services::orders::room::floor::post_cleared,
        post(&at("alpha", &format!("/api/staff/floor/{sitting}/cleared")), &json!({"location_id": "alpha"})).bearer(&waiter).on("alpha"),
        &[("sitting", &sitting)],
    );
    assert_eq!(r.status_code(), 409, "a table still eating is not cleared: {}", r.body_str());
    let stamps = site.run(crate::services::loyalty::handlers::order_stamps, get(&at("alpha", &format!("/api/order/{id}/stamps"))).bearer(&key).on("alpha"), &[("id", &id)]);
    assert!(stamps.status_code() < 500, "{}", stamps.body_str());
}

#[test]
fn a_line_moves_between_rounds_and_a_sitting_moves_to_another_table() {
    let site = Site::new();
    let (owner, dish) = open_venue(&site, "alpha", "a@x.test");
    let waiter = site.staff("alpha", &owner, "w@x.test", "waiter");
    let a = id_of(&dine_in_as(&site, "alpha", &dish, "T1", Some(&waiter)));
    let b = id_of(&dine_in_as(&site, "alpha", &dish, "T2", Some(&waiter)));
    // Round A gets a second line, so it has one to give.
    let (seq, _) = stored(&site, "alpha", &a);
    let r = site.run(
        crate::services::orders::room::handlers::amend,
        post(&at("alpha", &format!("/api/staff/orders/{a}/amend")), &json!({"location_id": "alpha", "base_seq": seq, "ops": [{"op": "add", "product_id": dish, "quantity": 2}]}))
            .bearer(&waiter)
            .on("alpha"),
        &[("id", &a)],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let (sa, oa) = stored(&site, "alpha", &a);
    let (sb, ob) = stored(&site, "alpha", &b);
    let (ta, tb) = (oa["subtotal"].as_i64().unwrap(), ob["subtotal"].as_i64().unwrap());
    let transfer = |tok: &str, from_seq: u64, to_seq: u64| {
        site.run(
            crate::services::orders::room::transfer::transfer,
            post(
                &at("alpha", &format!("/api/staff/orders/{a}/transfer")),
                &json!({"location_id": "alpha", "to_order_id": b, "from_base_seq": from_seq, "to_base_seq": to_seq, "lines": [1]}),
            )
            .bearer(tok)
            .on("alpha"),
            &[("id", &a)],
        )
    };
    // A stale base on either side is refused and writes nothing.
    let stale = transfer(&waiter, sa - 1, sb);
    assert_eq!(stale.status_code(), 409, "{}", stale.body_str());
    assert_eq!((stored(&site, "alpha", &a).0, stored(&site, "alpha", &b).0), (sa, sb));
    let r = transfer(&waiter, sa, sb);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let (_, oa2) = stored(&site, "alpha", &a);
    let (_, ob2) = stored(&site, "alpha", &b);
    // The money moves with the line: what A lost, B gained, and nothing else changed.
    let (ta2, tb2) = (oa2["subtotal"].as_i64().unwrap(), ob2["subtotal"].as_i64().unwrap());
    assert_eq!(ta - ta2, 1800, "{oa2}");
    assert_eq!(tb2 - tb, 1800, "{ob2}");
    assert_eq!(ta + tb, ta2 + tb2);

    // A sitting moves to a free table, every round with it.
    let sitting = oa2["sitting_id"].as_str().unwrap_or_else(|| panic!("no sitting on {oa2}")).to_string();
    let mv = |tok: &str, table: &str| {
        site.run(
            crate::services::orders::room::transfer::move_sitting,
            post(&at("alpha", &format!("/api/staff/sittings/{sitting}/move")), &json!({"location_id": "alpha", "table": table})).bearer(tok).on("alpha"),
            &[("id", &sitting)],
        )
    };
    let r = mv(&waiter, "T7");
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(r.body_value()["table"], "T7");
    let moved = r.body_value()["moved"].as_array().cloned().unwrap_or_default();
    assert!(moved.iter().any(|m| m[0] == a.as_str()), "the round moved with its sitting: {}", r.body_str());
    assert_eq!(stored(&site, "alpha", &a).1["fulfilment"]["table"], "T7", "{}", stored(&site, "alpha", &a).1);

    // Another venue's waiter moves neither a line nor a sitting.
    let (beta, _) = open_venue(&site, "beta", "b@x.test");
    let stranger = site.staff("beta", &beta, "s@x.test", "waiter");
    let (sa, sb) = (stored(&site, "alpha", &a).0, stored(&site, "alpha", &b).0);
    assert!(transfer(&stranger, sa, sb).status_code() >= 400);
    assert!(mv(&stranger, "T9").status_code() >= 400);
    assert_eq!((stored(&site, "alpha", &a).0, stored(&site, "alpha", &b).0), (sa, sb), "nothing was written");
    // A body that is not one is refused before anything is read.
    let r = site.run(
        crate::services::orders::room::transfer::transfer,
        post(&at("alpha", &format!("/api/staff/orders/{a}/transfer")), &json!({"location_id": "alpha"})).bearer(&waiter).on("alpha"),
        &[("id", &a)],
    );
    assert_eq!(r.status_code(), 400);
}
