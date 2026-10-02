//! Reservations through the routes (W-COV C2): the owner draws the floor, a guest books a table
//! for tomorrow evening, reads it back with its own key, the venue sees and confirms it -- and
//! the other venue's owner sees none of it.

use crate::edge::site::{get, post, As, Site, PLATFORM_HOST, T0};
use serde_json::{json, Value};

fn at(slug: &str, path: &str) -> String {
    format!("https://{slug}.{PLATFORM_HOST}{path}")
}

/// Tomorrow 19:00 UTC, in minutes, as the booking API counts.
fn tomorrow_evening_min() -> i64 {
    let day = 86_400_000;
    ((T0 / day + 1) * day + 19 * 3_600_000) / 60_000
}

fn plan() -> Value {
    let t = |n: i64, seats: i64| json!({ "n": n, "x": 100 * n, "y": 100, "w": 40, "h": 40, "seats": seats });
    json!({"zones": [{ "id": "salla", "name": "Salla", "tables": [t(1, 4), t(2, 2)] }]})
}

fn floor(site: &Site, owner: &str) {
    let r = site.run(crate::booking::set_plan, post(&at("alpha", "/api/owner/floorplan"), &plan()).bearer(owner).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "plan: {}", r.body_str());
    assert_eq!(r.body_value()["tables"], json!(2));
}

fn book(site: &Site, party: u16, request: &str) -> crate::wire::Reply {
    site.run(
        crate::booking::create,
        post(
            &at("alpha", "/api/public/locations/alpha/reservations"),
            &json!({"party": party, "slotMin": tomorrow_evening_min(), "contactName": "Ana", "contactPhone": "+355690000009", "requestId": request}),
        )
        .on("alpha"),
        &[("slug", "alpha")],
    )
}

#[test]
fn the_floor_is_drawn_read_back_and_a_bad_plan_is_refused() {
    let site = Site::new();
    let a = site.venue("alpha", "a@x.test");
    let bad = site.run(
        crate::booking::set_plan,
        post(&at("alpha", "/api/owner/floorplan"), &json!({"zones": [{"id": "z", "name": "Z", "tables": [{"n": 1, "x": 1, "y": 1, "w": 1, "h": 1, "seats": 0}]}]}))
            .bearer(&a)
            .on("alpha"),
        &[],
    );
    assert_eq!(bad.status_code(), 400, "a table with no seats: {}", bad.body_str());
    floor(&site, &a);
    let r = site.run(crate::booking::get_plan, get(&at("alpha", "/api/owner/floorplan")).bearer(&a).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(r.body_value()["zones"][0]["id"], "salla");
    let b = site.venue("beta", "b@x.test");
    let r = site.run(crate::booking::get_plan, get(&at("beta", "/api/owner/floorplan")).bearer(&b).on("beta"), &[]);
    assert!(!r.body_str().contains("salla"), "beta's floor is not alpha's: {}", r.body_str());
}

#[test]
fn a_guest_books_reads_it_back_and_the_venue_confirms() {
    let site = Site::new();
    let a = site.venue("alpha", "a@x.test");
    floor(&site, &a);
    let free = site.run(
        crate::booking::availability,
        get(&at("alpha", &format!("/api/public/locations/alpha/tables?slotMin={}", tomorrow_evening_min()))).on("alpha"),
        &[("slug", "alpha")],
    );
    assert_eq!(free.status_code(), 200, "{}", free.body_str());
    let no_slot = site.run(crate::booking::availability, get(&at("alpha", "/api/public/locations/alpha/tables")).on("alpha"), &[("slug", "alpha")]);
    assert_eq!(no_slot.status_code(), 400, "a plan without a slot is refused: {}", no_slot.body_str());

    let r = book(&site, 2, "req-1");
    assert_eq!(r.status_code(), 200, "book: {}", r.body_str());
    let v = r.body_value();
    let id = ["id", "reservationId"].iter().find_map(|k| v[*k].as_str().or(v["reservation"][*k].as_str())).unwrap_or_else(|| panic!("{v}")).to_string();
    let token = ["token", "guestToken", "key"].iter().find_map(|k| v[*k].as_str()).map(str::to_string);
    // The same requestId is the same booking (the retry of a form), not a second one.
    let again = book(&site, 2, "req-1");
    assert!(again.body_str().contains(&id), "a retried request made a second booking: {}", again.body_str());

    if let Some(t) = token {
        let d = site.run(
            crate::booking::detail,
            get(&at("alpha", &format!("/api/public/locations/alpha/reservations/{id}"))).bearer(&t).on("alpha"),
            &[("slug", "alpha"), ("id", &id)],
        );
        assert_eq!(d.status_code(), 200, "{}", d.body_str());
    }
    let anon = site.run(
        crate::booking::detail,
        get(&at("alpha", &format!("/api/public/locations/alpha/reservations/{id}"))).on("alpha"),
        &[("slug", "alpha"), ("id", &id)],
    );
    assert!(anon.status_code() >= 400, "a booking is not readable by whoever knows its id: {}", anon.body_str());

    let range = format!("/api/owner/reservations?from={}&to={}", tomorrow_evening_min() - 600, tomorrow_evening_min() + 600);
    let bare = site.run(crate::booking::venue_day, get(&at("alpha", "/api/owner/reservations")).bearer(&a).on("alpha"), &[]);
    assert_eq!(bare.status_code(), 400, "a day needs its bounds: {}", bare.body_str());
    let day = site.run(crate::booking::venue_day, get(&at("alpha", &range)).bearer(&a).on("alpha"), &[]);
    assert_eq!(day.status_code(), 200, "{}", day.body_str());
    assert!(day.body_str().contains(&id), "the venue sees the booking: {}", day.body_str());
    let r = site.run(
        crate::booking::venue_action,
        post(&at("alpha", &format!("/api/owner/reservations/{id}/action")), &json!({"to": "CONFIRMED"})).bearer(&a).on("alpha"),
        &[("id", &id)],
    );
    assert_eq!(r.status_code(), 200, "confirm: {}", r.body_str());
    assert_eq!(r.body_value()["status"], "CONFIRMED");
    let bogus = site.run(
        crate::booking::venue_action,
        post(&at("alpha", &format!("/api/owner/reservations/{id}/action")), &json!({"to": "EATEN"})).bearer(&a).on("alpha"),
        &[("id", &id)],
    );
    assert_eq!(bogus.status_code(), 400);

    let b = site.venue("beta", "b@x.test");
    let r = site.run(
        crate::booking::venue_action,
        post(&at("alpha", &format!("/api/owner/reservations/{id}/action?location_id=alpha")), &json!({"to": "CANCELLED"})).bearer(&b).on("alpha"),
        &[("id", &id)],
    );
    assert!(r.status_code() >= 400, "beta's owner cancelled alpha's booking: {}", r.body_str());
}

#[test]
fn a_booking_without_a_request_id_or_a_name_is_refused() {
    let site = Site::new();
    let a = site.venue("alpha", "a@x.test");
    floor(&site, &a);
    let r = site.run(
        crate::booking::create,
        post(&at("alpha", "/api/public/locations/alpha/reservations"), &json!({"party": 2, "slotMin": tomorrow_evening_min(), "requestId": " "})).on("alpha"),
        &[("slug", "alpha")],
    );
    assert_eq!(r.status_code(), 400);
    let r = site.run(
        crate::booking::create,
        post(&at("alpha", "/api/public/locations/alpha/reservations"), &json!({"party": 2, "slotMin": tomorrow_evening_min(), "requestId": "x"})).on("alpha"),
        &[("slug", "alpha")],
    );
    assert_eq!(r.status_code(), 400, "a guest must say who: {}", r.body_str());
}

fn book_at(site: &Site, slot: i64, request: &str, phone: &str) -> crate::wire::Reply {
    site.run(
        crate::booking::create,
        post(
            &at("alpha", "/api/public/locations/alpha/reservations"),
            &json!({"party": 2, "slotMin": slot, "contactName": "Ana", "contactPhone": phone, "requestId": request}),
        )
        .on("alpha"),
        &[("slug", "alpha")],
    )
}

fn pass_of(site: &Site, id: &str, tok: &str) -> crate::wire::Reply {
    site.run(
        crate::booking::issue_pass,
        get(&at("alpha", &format!("/api/public/locations/alpha/reservations/{id}/pass"))).bearer(tok).on("alpha"),
        &[("slug", "alpha"), ("id", id)],
    )
}

fn scan(site: &Site, code: &str) -> Value {
    let r = site.run(
        crate::booking::verify_pass,
        post(&at("alpha", "/api/public/locations/alpha/pass/verify"), &json!({"code": code})).on("alpha"),
        &[("slug", "alpha")],
    );
    assert_eq!(r.status_code(), 200, "a scanner's question always succeeds: {}", r.body_str());
    r.body_value()
}

fn guest_act(site: &Site, id: &str, tok: &str, to: &str) -> crate::wire::Reply {
    site.run(
        crate::booking::action,
        post(&at("alpha", &format!("/api/public/locations/alpha/reservations/{id}/action")), &json!({"to": to, "reason": "plans changed"})).bearer(tok).on("alpha"),
        &[("slug", "alpha"), ("id", id)],
    )
}

#[test]
fn a_confirmed_booking_gets_a_pass_the_door_reads_and_the_guest_holds_only_their_own() {
    let mut site = Site::new();
    let a = site.venue("alpha", "a@x.test");
    floor(&site, &a);
    // A bookable slot (on the grid, inside the hours); the door is reached by moving the clock.
    let soon = tomorrow_evening_min();
    let r = book_at(&site, soon, "req-soon", "+355690000011");
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let v = r.body_value();
    let (id, tok) = (v["id"].as_str().unwrap().to_string(), v["access_token"].as_str().expect("a guest token, once").to_string());
    // An identical retry from the same phone is answered with the recorded first answer, verbatim
    // (the idempotency key is the requestId, scoped to the phone): same booking, same token.
    let again = book_at(&site, soon, "req-soon", "+355690000011").body_value();
    assert_eq!(again["id"].as_str(), Some(id.as_str()), "{again}");
    assert!(again["access_token"].is_string(), "the same phone gets its token back: {again}");
    // Another phone with the same requestId is a new claim that finds the booking: the same id,
    // said to be a replay, and no token -- a token goes only to the phone the booking is under.
    let other_phone = book_at(&site, soon, "req-soon", "+355690000099").body_value();
    assert_eq!((other_phone["id"].as_str(), other_phone["replayed"].as_bool()), (Some(id.as_str()), Some(true)), "{other_phone}");
    assert!(other_phone["access_token"].is_null(), "another phone does not: {other_phone}");

    // The guest reads their booking; it is a request until the venue answers.
    let d = site.run(
        crate::booking::detail,
        get(&at("alpha", &format!("/api/public/locations/alpha/reservations/{id}"))).bearer(&tok).on("alpha"),
        &[("slug", "alpha"), ("id", &id)],
    );
    assert_eq!(d.status_code(), 200, "{}", d.body_str());
    let dv = d.body_value();
    assert_eq!((dv["status"].as_str(), dv["statusCacheDrifted"].as_bool()), (Some("REQUESTED"), Some(false)), "{dv}");
    // No pass for an unanswered request.
    let r = pass_of(&site, &id, &tok);
    assert_eq!(r.status_code(), 409, "{}", r.body_str());
    // A guest cannot confirm their own booking.
    assert_eq!(guest_act(&site, &id, &tok, "CONFIRMED").status_code(), 403);

    let r = site.run(
        crate::booking::venue_action,
        post(&at("alpha", &format!("/api/owner/reservations/{id}/action")), &json!({"to": "CONFIRMED"})).bearer(&a).on("alpha"),
        &[("id", &id)],
    );
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let p = pass_of(&site, &id, &tok);
    assert_eq!(p.status_code(), 200, "{}", p.body_str());
    let code = p.body_value()["code"].as_str().unwrap().to_string();
    // A day early the door names the reason and lets nobody in.
    let early = scan(&site, &code);
    assert_eq!(early["ok"], false, "{early}");
    assert!(early["why"].as_str().is_some_and(|w| !w.is_empty()), "{early}");
    // Ten minutes before the slot: inside the door's window (30 early, 15 grace).
    site.now_ms = (soon - 10) * 60_000;
    let ok = scan(&site, &code);
    assert_eq!((ok["ok"].as_bool(), ok["party"].as_u64(), ok["slotMin"].as_i64()), (Some(true), Some(2), Some(soon)), "{ok}");
    // The venue's key is stable: the owner's reissue verifies too.
    let by_owner = pass_of(&site, &id, &a);
    assert_eq!(scan(&site, by_owner.body_value()["code"].as_str().unwrap())["ok"], true);
    // A tampered code and garbage are named refusals, not errors.
    let mut bad = code.clone().into_bytes();
    let last = bad.len() - 2;
    bad[last] = if bad[last] == b'A' { b'B' } else { b'A' };
    assert_eq!(scan(&site, &String::from_utf8(bad).unwrap())["ok"], false);
    let junk = scan(&site, "not-a-pass");
    assert_eq!(junk["ok"], false);
    assert!(junk["why"].as_str().is_some_and(|w| !w.is_empty()), "{junk}");

    // Another booking's guest sees nothing of this one.
    let other = book_at(&site, soon + 60 * 24, "req-other", "+355690000012").body_value();
    let other_tok = other["access_token"].as_str().expect("token").to_string();
    assert_eq!(pass_of(&site, &id, &other_tok).status_code(), 404);
    assert_eq!(guest_act(&site, &id, &other_tok, "CANCELLED_BY_GUEST").status_code(), 404);
    // The venue reads one user's bookings; a guest's token does not open that list.
    let list = |tok: &str| {
        site.run(
            crate::booking::list,
            get(&at("alpha", "/api/public/locations/alpha/reservations?user=u1")).bearer(tok).on("alpha"),
            &[("slug", "alpha")],
        )
    };
    assert_eq!(list(&a).status_code(), 200);
    assert_eq!(list(&tok).status_code(), 403);
    let nouser = site.run(crate::booking::list, get(&at("alpha", "/api/public/locations/alpha/reservations")).bearer(&a).on("alpha"), &[("slug", "alpha")]);
    assert_eq!(nouser.status_code(), 400);

    // The guest cancels their own; the pass stops being issued.
    // The venue's word is not the guest's: a guest cannot cancel in the venue's name.
    assert!(guest_act(&site, &id, &tok, "CANCELLED_BY_VENUE").status_code() >= 400);
    let r = guest_act(&site, &id, &tok, "CANCELLED_BY_GUEST");
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    assert_eq!(r.body_value()["status"], "CANCELLED_BY_GUEST", "{}", r.body_str());
    assert_eq!(pass_of(&site, &id, &tok).status_code(), 409);
    assert_eq!(guest_act(&site, &id, &tok, "NOPE").status_code(), 400);

    // Beta's owner neither reads nor mints alpha's pass.
    let b = site.venue("beta", "b@x.test");
    let r = pass_of(&site, &id, &b);
    assert!([401, 403, 404].contains(&r.status_code()), "{}", r.body_str());
    assert!(!r.body_str().contains("code"), "no pass in a refusal: {}", r.body_str());
}
