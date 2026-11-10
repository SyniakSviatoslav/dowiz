//! The payroll routes through the real seam: owner-only, venue-scoped, every write read back
//! through the month view, money exact.

use super::*;
use crate::edge::site::{get, post, As, Site, PLATFORM_HOST, T0};
use serde_json::{json, Value};

fn at(slug: &str, path: &str) -> String {
    format!("https://{slug}.{PLATFORM_HOST}{path}")
}

struct Two { site: Site, a: String, b: String, ira: String, leo: String, beta_staff: String }

fn two() -> Two {
    let site = Site::new();
    let a = site.venue("alpha", "a@x.test");
    let b = site.venue("beta", "b@x.test");
    let (_, ira) = site.staff_full("alpha", &a, "ira@x.test", "waiter");
    let (_, leo) = site.staff_full("alpha", &a, "leo@x.test", "kitchen");
    let (_, beta_staff) = site.staff_full("beta", &b, "bo@x.test", "waiter");
    Two { site, a, b, ira, leo, beta_staff }
}

fn rate(t: &Two, who: &str, minor: i64) -> crate::wire::Reply {
    t.site.run(set_rate, post(&at("alpha", "/api/owner/payroll/rate"), &json!({"staff_id": who, "rate_minor": minor})).bearer(&t.a).on("alpha"), &[])
}

fn enter(t: &Two, who: &str, date: &str, minutes: i64) -> crate::wire::Reply {
    t.site.run(set_entry, post(&at("alpha", "/api/owner/payroll/entry"), &json!({"staff_id": who, "date": date, "minutes": minutes})).bearer(&t.a).on("alpha"), &[])
}

fn view(t: &Two, month: &str) -> Value {
    let r = t.site.run(month_route, get(&at("alpha", &format!("/api/owner/payroll?month={month}"))).bearer(&t.a).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    r.body_value()
}

fn ok(r: crate::wire::Reply) -> crate::wire::Reply {
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    r
}

/// The month's pay as the money sheet subtracts it: the payroll row of the venue's own
/// `GET /api/owner/finance`, asked by that venue's owner.
fn total(t: &Two, venue: &str, month: &str) -> i64 {
    let owner = if venue == "alpha" { &t.a } else { &t.b };
    let r = t.site.run(crate::services::finance::routes::finance, get(&at(venue, &format!("/api/owner/finance?location_id={venue}&month={month}"))).bearer(owner).on(venue), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let v = r.body_value();
    let row = v["profit"]["minus"]["rows"].as_array().unwrap().iter().find(|r| r["key"] == "payroll").expect("the payroll row").clone();
    row["amount"].as_i64().expect("an integer")
}

#[test]
fn hours_become_pay_in_the_month_view_and_the_total_function_agrees() {
    let t = two();
    ok(rate(&t, &t.ira, 250));
    ok(rate(&t, &t.leo, 300));
    ok(enter(&t, &t.ira, "2023-11-13", 450)); // 7.5 h x 250 = 1875
    ok(enter(&t, &t.ira, "2023-11-14", 15)); // 0.25 h: row rounded once below
    ok(enter(&t, &t.leo, "2023-11-13", 480)); // 8 h x 300 = 2400
    let v = view(&t, "2023-11");
    let row = |id: &str| v["rows"].as_array().unwrap().iter().find(|r| r["staffId"] == id).unwrap().clone();
    assert_eq!(row(&t.ira)["minutes"], 465);
    assert_eq!(row(&t.ira)["payMinor"], 1938); // 465 x 250 / 60 = 1937.5, half up
    assert_eq!(row(&t.leo)["payMinor"], 2400);
    assert_eq!(v["totalMinor"], 4338);
    assert_eq!(v["dates"], json!(["2023-11-13", "2023-11-14"]));
    assert_eq!(row(&t.ira)["name"], "Staff", "names come from the existing staff entity");
    assert_eq!(total(&t, "alpha", "2023-11"), 4338, "the money sheet subtracts the table's own total");
    assert_eq!(total(&t, "alpha", "2023-10"), 0);
    assert_eq!(total(&t, "beta", "2023-11"), 0, "beta's total is beta's");
}

#[test]
fn without_a_month_the_view_is_the_venues_current_month() {
    let t = two();
    let r = ok(t.site.run(month_route, get(&at("alpha", "/api/owner/payroll")).bearer(&t.a).on("alpha"), &[]));
    let v = r.body_value();
    assert_eq!(v["month"], "2023-11");
    assert_eq!(v["today"], "2023-11-14");
    assert_eq!(r.headers().get("cache-control").unwrap().as_deref(), Some("private, no-store"), "pay is not cached anywhere");
    let bad = t.site.run(month_route, get(&at("alpha", "/api/owner/payroll?month=2023-13")).bearer(&t.a).on("alpha"), &[]);
    assert_eq!(bad.status_code(), 400, "{}", bad.body_str());
}

#[test]
fn an_entry_needs_a_rate_first_and_then_is_accepted() {
    let t = two();
    let r = enter(&t, &t.ira, "2023-11-13", 60);
    assert_eq!(r.status_code(), 409, "{}", r.body_str());
    ok(rate(&t, &t.ira, 100));
    let r = ok(enter(&t, &t.ira, "2023-11-13", 60));
    assert_eq!(r.body_value()["entry"]["rateMinor"], 100);
}

#[test]
fn a_bad_entry_is_refused_and_a_good_twin_accepted() {
    let t = two();
    ok(rate(&t, &t.ira, 100));
    for (minutes, date) in [(0, "2023-11-13"), (7, "2023-11-13"), (1455, "2023-11-13"), (60, "2023-02-30"), (60, "nope")] {
        let r = enter(&t, &t.ira, date, minutes);
        assert_eq!(r.status_code(), 400, "{minutes} {date}: {}", r.body_str());
    }
    let unknown = t.site.run(set_entry, post(&at("alpha", "/api/owner/payroll/entry"), &json!({"staff_id": t.ira, "date": "2023-11-13", "minutes": 60, "hours": 1})).bearer(&t.a).on("alpha"), &[]);
    assert_eq!(unknown.status_code(), 400, "{}", unknown.body_str());
    ok(enter(&t, &t.ira, "2023-11-13", 60));
}

#[test]
fn the_future_is_refused_in_the_venues_own_day() {
    let mut t = two();
    ok(rate(&t, &t.ira, 100));
    // 23:30 UTC on the 14th is already the 15th in the venue's zone (Tirana, UTC+1 in November).
    t.site.now_ms = T0 + 77 * 60_000;
    ok(enter(&t, &t.ira, "2023-11-15", 60));
    let r = enter(&t, &t.ira, "2023-11-16", 60);
    assert_eq!(r.status_code(), 400, "{}", r.body_str());
}

#[test]
fn a_rate_change_never_rewrites_what_was_already_entered() {
    let t = two();
    ok(rate(&t, &t.ira, 100));
    ok(enter(&t, &t.ira, "2023-10-30", 60));
    ok(rate(&t, &t.ira, 500));
    assert_eq!(total(&t, "alpha", "2023-10"), 100, "last month keeps its rate");
    // An edit of hours keeps that entry's rate.
    ok(enter(&t, &t.ira, "2023-10-30", 120));
    assert_eq!(total(&t, "alpha", "2023-10"), 200);
    // A new entry takes the current rate.
    ok(enter(&t, &t.ira, "2023-11-13", 60));
    assert_eq!(total(&t, "alpha", "2023-11"), 500);
    assert_eq!(view(&t, "2023-10")["rows"].as_array().unwrap().iter().find(|r| r["staffId"] == json!(t.ira)).unwrap()["minutes"], 120, "one entry per person and date: the edit replaced it, not added");
}

#[test]
fn a_mistake_is_deleted_and_a_second_delete_is_404() {
    let t = two();
    ok(rate(&t, &t.ira, 100));
    ok(enter(&t, &t.ira, "2023-11-13", 60));
    let del = |t: &Two| t.site.run(delete_entry, post(&at("alpha", "/api/owner/payroll/entry/delete"), &json!({"staff_id": t.ira, "date": "2023-11-13"})).bearer(&t.a).on("alpha"), &[]);
    ok(del(&t));
    assert_eq!(total(&t, "alpha", "2023-11"), 0);
    assert_eq!(del(&t).status_code(), 404);
}

#[test]
fn one_employees_history_is_pay_by_month() {
    let t = two();
    ok(rate(&t, &t.ira, 100));
    ok(enter(&t, &t.ira, "2023-09-20", 60));
    ok(enter(&t, &t.ira, "2023-11-13", 90));
    let r = ok(t.site.run(history_route, get(&at("alpha", &format!("/api/owner/payroll/history?staff_id={}", t.ira))).bearer(&t.a).on("alpha"), &[]));
    let v = r.body_value();
    assert_eq!(r.headers().get("cache-control").unwrap().as_deref(), Some("private, no-store"));
    assert_eq!(v["months"], json!([{"month":"2023-11","minutes":90,"payMinor":150},{"month":"2023-09","minutes":60,"payMinor":100}]));
    let missing = t.site.run(history_route, get(&at("alpha", "/api/owner/payroll/history")).bearer(&t.a).on("alpha"), &[]);
    assert_eq!(missing.status_code(), 400);
}

#[test]
fn another_venues_owner_gets_nothing_and_changes_nothing() {
    let t = two();
    ok(rate(&t, &t.ira, 100));
    ok(enter(&t, &t.ira, "2023-11-13", 60));
    // Beta's owner naming alpha: every route refuses.
    let read = t.site.run(month_route, get(&at("alpha", "/api/owner/payroll?month=2023-11&location_id=alpha")).bearer(&t.b).on("alpha"), &[]);
    assert!(read.status_code() >= 400, "{} {}", read.status_code(), read.body_str());
    let hist = t.site.run(history_route, get(&at("alpha", &format!("/api/owner/payroll/history?staff_id={}&location_id=alpha", t.ira))).bearer(&t.b).on("alpha"), &[]);
    assert!(hist.status_code() >= 400, "{}", hist.body_str());
    let r = t.site.run(set_entry, post(&at("alpha", "/api/owner/payroll/entry?location_id=alpha"), &json!({"staff_id": t.ira, "date": "2023-11-13", "minutes": 120})).bearer(&t.b).on("alpha"), &[]);
    assert!(r.status_code() >= 400, "{}", r.body_str());
    let r = t.site.run(set_rate, post(&at("alpha", "/api/owner/payroll/rate?location_id=alpha"), &json!({"staff_id": t.ira, "rate_minor": 1})).bearer(&t.b).on("alpha"), &[]);
    assert!(r.status_code() >= 400, "{}", r.body_str());
    let r = t.site.run(delete_entry, post(&at("alpha", "/api/owner/payroll/entry/delete?location_id=alpha"), &json!({"staff_id": t.ira, "date": "2023-11-13"})).bearer(&t.b).on("alpha"), &[]);
    assert!(r.status_code() >= 400, "{}", r.body_str());
    assert_eq!(total(&t, "alpha", "2023-11"), 100, "alpha's money did not move");
    // Beta's own view does not carry alpha's rows.
    let own = ok(t.site.run(month_route, get(&at("beta", "/api/owner/payroll?month=2023-11")).bearer(&t.b).on("beta"), &[]));
    assert!(!own.body_str().contains(&t.ira), "{}", own.body_str());
    assert_eq!(own.body_value()["totalMinor"], 0);
}

#[test]
fn an_employee_of_another_venue_cannot_be_paid_here() {
    let t = two();
    // Alpha's own owner naming beta's employee id.
    let r = rate(&t, &t.beta_staff, 100);
    assert_eq!(r.status_code(), 404, "{}", r.body_str());
    let r = enter(&t, &t.beta_staff, "2023-11-13", 60);
    assert_eq!(r.status_code(), 404, "{}", r.body_str());
    // And the twin: alpha's own employee is accepted.
    ok(rate(&t, &t.ira, 100));
}

#[test]
fn a_member_of_staff_cannot_open_the_payroll() {
    let t = two();
    let staff = t.site.staff("alpha", &t.a, "sara@x.test", "kitchen");
    let r = t.site.run(month_route, get(&at("alpha", "/api/owner/payroll?month=2023-11")).bearer(&staff).on("alpha"), &[]);
    assert!(r.status_code() >= 400, "{} {}", r.status_code(), r.body_str());
}

/// AN OWNER OF TWO VENUES reads and writes the one the query names: the token was issued for
/// alpha, the console shows beta. Without the parameter the same token means alpha.
#[test]
fn an_owner_of_two_venues_reads_and_writes_the_venue_the_query_names() {
    let site = Site::new();
    let a = site.venue("alpha", "a@x.test");
    site.venue("beta", "a@x.test");
    let (_, ira) = site.staff_full("alpha", &a, "ira@x.test", "waiter");
    let b = site.login("beta", "a@x.test");
    let (_, bo) = site.staff_full("beta", &b, "bo@x.test", "waiter");
    let on_beta = |path: &str| at("alpha", &format!("/api/owner/payroll{path}{}location_id=beta", if path.contains('?') { '&' } else { '?' }));
    // Alpha's token, beta named: every route acts on beta.
    ok(site.run(set_rate, post(&on_beta("/rate"), &json!({"staff_id": bo, "rate_minor": 400})).bearer(&a).on("alpha"), &[]));
    ok(site.run(set_entry, post(&on_beta("/entry"), &json!({"staff_id": bo, "date": "2023-11-13", "minutes": 60})).bearer(&a).on("alpha"), &[]));
    let month = |url: &str| ok(site.run(month_route, get(url).bearer(&a).on("alpha"), &[])).body_value();
    let beta = month(&on_beta("?month=2023-11"));
    assert_eq!(beta["totalMinor"], 400, "{beta}");
    let ids = |v: &Value| v["staff"].as_array().unwrap().iter().map(|s| s["id"].as_str().unwrap().to_string()).collect::<Vec<_>>();
    assert_eq!(ids(&beta), vec![bo.clone()], "beta's staff, and nobody of alpha's");
    let hist = ok(site.run(history_route, get(&on_beta(&format!("/history?staff_id={bo}"))).bearer(&a).on("alpha"), &[])).body_value();
    assert_eq!(hist["months"], json!([{"month": "2023-11", "minutes": 60, "payMinor": 400}]));
    // The same token with no venue named is alpha: beta's hour is not there, alpha's person is.
    let alpha = month(&at("alpha", "/api/owner/payroll?month=2023-11"));
    assert_eq!(alpha["totalMinor"], 0, "{alpha}");
    assert_eq!(ids(&alpha), vec![ira.clone()]);
    // Beta's person is not alpha's staff, so alpha (no venue named) refuses her; naming beta deletes the day.
    let wrong = site.run(set_entry, post(&at("alpha", "/api/owner/payroll/entry"), &json!({"staff_id": bo, "date": "2023-11-13", "minutes": 60})).bearer(&a).on("alpha"), &[]);
    assert_eq!(wrong.status_code(), 404, "{}", wrong.body_str());
    ok(site.run(delete_entry, post(&on_beta("/entry/delete"), &json!({"staff_id": bo, "date": "2023-11-13"})).bearer(&a).on("alpha"), &[]));
    assert_eq!(month(&on_beta("?month=2023-11"))["totalMinor"], 0);
}

/// A body names no venue: the query is the one place the venue is read, so a body that tries
/// is refused as any unknown field is, on all three writes.
#[test]
fn a_venue_in_the_body_is_an_unknown_field_and_its_twin_without_one_is_accepted() {
    let t = two();
    let run = |r: crate::wire::Reply| r.status_code();
    assert_eq!(run(t.site.run(set_rate, post(&at("alpha", "/api/owner/payroll/rate"), &json!({"staff_id": t.ira, "rate_minor": 100, "location_id": "beta"})).bearer(&t.a).on("alpha"), &[])), 400);
    ok(rate(&t, &t.ira, 100));
    assert_eq!(run(t.site.run(set_entry, post(&at("alpha", "/api/owner/payroll/entry"), &json!({"staff_id": t.ira, "date": "2023-11-13", "minutes": 60, "location_id": "beta"})).bearer(&t.a).on("alpha"), &[])), 400);
    ok(enter(&t, &t.ira, "2023-11-13", 60));
    assert_eq!(run(t.site.run(delete_entry, post(&at("alpha", "/api/owner/payroll/entry/delete"), &json!({"staff_id": t.ira, "date": "2023-11-13", "location_id": "beta"})).bearer(&t.a).on("alpha"), &[])), 400);
    assert_eq!(total(&t, "alpha", "2023-11"), 100, "the refused delete removed nothing");
}

/// A PERSON WHO LEFT THE STAFF keeps the days already entered: they stay in the month and in
/// its total, cannot be changed (the person is not staff), and can be deleted.
#[test]
fn a_departed_persons_hours_stay_in_the_month_and_can_only_be_deleted() {
    let t = two();
    ok(rate(&t, &t.ira, 100));
    ok(enter(&t, &t.ira, "2023-11-13", 60));
    let place = crate::hubstore::Place::of_authorised(&t.site.ctx(&[]), "alpha").expect("place");
    let gone = "left-the-venue".to_string();
    let g = gone.clone();
    crate::edge::mem::block_on(crate::hubstore::with_table(&place, IMAGE, BYTES, move |tb| {
        let e = Entry { staff_id: g.clone(), date: "2023-11-12".into(), minutes: 120, rate_minor: 300, note: String::new() };
        tb.put(K_ENTRY, &rules::entry_id(&e.date, &e.staff_id), &serde_json::to_string(&e).unwrap(), &[], &[]).map_err(|e| Error::RustError(format!("{e}")))
    }))
    .expect("the stored day");
    let v = view(&t, "2023-11");
    let row = v["rows"].as_array().unwrap().iter().find(|r| r["staffId"] == json!(gone)).expect("the row").clone();
    assert_eq!((row["payMinor"].clone(), row["active"].clone()), (json!(600), Value::Null), "{row}");
    assert_eq!(v["totalMinor"], 700);
    assert_eq!(enter(&t, &gone, "2023-11-12", 60).status_code(), 404, "not staff: the day is not changed");
    ok(t.site.run(delete_entry, post(&at("alpha", "/api/owner/payroll/entry/delete"), &json!({"staff_id": gone, "date": "2023-11-12"})).bearer(&t.a).on("alpha"), &[]));
    assert_eq!(view(&t, "2023-11")["totalMinor"], 100);
}
