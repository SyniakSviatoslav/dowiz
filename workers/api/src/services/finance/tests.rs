//! W-FINANCE. Every rule beside its positive twin; the money exact; the routes through the
//! real handlers, and through the venue's object for the month's numbers.

use super::profit::{build, months_ending, receipts_of, Receipts};
use super::routes;
use super::*;
use crate::edge::site::{get, post, As, Site, PLATFORM_HOST};
use crate::services::analytics::cube::{self, DayCube};
use crate::storefront::route_tests::{open_venue, place_pickup};
use dowiz_hub::stock::meta::day_number;
use serde_json::{json, Value};
use std::collections::BTreeMap;

const TODAY: i64 = 20261115;

fn input(date: &str, name: &str, amount: i64, kind: Kind) -> ExpenseIn {
    ExpenseIn { date: date.into(), name: name.into(), amount, kind, client_id: None }
}

#[test]
fn an_expense_needs_a_real_past_date_a_name_and_a_positive_whole_amount() {
    let ok = validate(input("2026-11-03", "  Бензин ", 500, Kind::Other), TODAY).unwrap();
    assert_eq!((ok.date.as_str(), ok.day, ok.name.as_str(), ok.amount, ok.kind), ("2026-11-03", 20261103, "Бензин", 500, Kind::Other));
    assert!(validate(input("2026-11-15", "сьогодні", 1, Kind::Tax), TODAY).is_ok(), "today is allowed");
    for (date, name, amount, why) in [
        ("2026-02-30", "x", 1, "not a date"),
        ("zzz", "x", 1, "not a date"),
        ("2026-11-16", "x", 1, "future"),
        ("1999-12-31", "x", 1, "not a date"),
        ("2026-11-03", "   ", 1, "name"),
        ("2026-11-03", "x", 0, "amount"),
        ("2026-11-03", "x", -500, "amount"),
        ("2026-11-03", "x", AMOUNT_MAX + 1, "amount"),
    ] {
        let e = validate(input(date, name, amount, Kind::Other), TODAY).unwrap_err();
        assert!(e.contains(why), "{date} {name} {amount}: {e}");
    }
    let long = "ж".repeat(NAME_MAX + 1);
    assert!(validate(input("2026-11-03", &long, 1, Kind::Other), TODAY).is_err());
    assert!(validate(input("2026-11-03", &"ж".repeat(NAME_MAX), 1, Kind::Other), TODAY).is_ok());
}

#[test]
fn a_kind_is_tax_or_other_and_an_unknown_field_is_refused() {
    let k: Kind = serde_json::from_str("\"tax\"").unwrap();
    assert_eq!(k, Kind::Tax);
    assert!(serde_json::from_str::<Kind>("\"bribe\"").is_err());
    assert!(serde_json::from_str::<ExpenseIn>(r#"{"date":"2026-11-03","name":"x","amount":1,"kind":"other"}"#).is_ok());
    assert!(serde_json::from_str::<ExpenseIn>(r#"{"date":"2026-11-03","name":"x","amount":1,"kind":"other","venue":"beta"}"#).is_err());
    assert!(serde_json::from_str::<ExpenseIn>(r#"{"date":"2026-11-03","name":"x","amount":1.5,"kind":"other"}"#).is_err(), "money is whole minor units");
}

#[test]
fn a_month_is_yyyy_mm_and_the_history_crosses_a_year() {
    assert_eq!(parse_month("2026-11"), Some(202611));
    for bad in ["2026-13", "2026-00", "2026", "2026-1", "26-11", "2026-11-01", ""] {
        assert_eq!(parse_month(bad), None, "{bad:?}");
    }
    assert_eq!(show_month(202601), "2026-01");
    assert_eq!(months_ending(202602, 4), vec![202511, 202512, 202601, 202602]);
}

fn order(id: &str, at: i64, status: &str, total: i64, tip: i64, channel: Option<&str>) -> Value {
    let mut o = json!({"id": id, "created_at_ms": at, "status": status, "total": total, "tip": tip});
    if let Some(c) = channel {
        o["channel"] = json!(c);
    }
    o
}

fn expense(day: i64, amount: i64, kind: Kind) -> Expense {
    Expense { date: dowiz_hub::stock::meta::show_day(day), day, name: "x".into(), amount, kind, at: 0 }
}

/// THE CARD'S EXACT MONTH: sales from both doors, refunds and refused out, a tip not the
/// venue's, purchases, taxes, other, payroll -> one net.
#[test]
fn a_month_with_everything_comes_to_the_exact_net() {
    let zone = dowiz_hub::tz::DEFAULT;
    let nov = crate::services::analytics::history::midnight(zone, day_number(20261101));
    let orders = vec![
        order("web", nov + 3_600_000, "DELIVERED", 3_000, 200, None), // 2,800: the tip is the courier's
        order("till", nov + 7_200_000, "PICKED_UP", 1_500, 0, Some("ebills")),
        order("till2", nov + 9_000_000, "PICKED_UP", 700, 0, Some("ebills")),
        order("no", nov + 7_300_000, "REJECTED", 999, 0, None),
        order("wait", nov + 7_400_000, "PENDING", 888, 0, None),
        order("back", nov + 7_500_000, "COMPENSATED_REFUND", 777, 0, Some("ebills")),
        // One millisecond before the venue's November: October's money.
        order("oct", nov - 1, "DELIVERED", 111, 0, None),
    ];
    let hot = cube::fold_orders(&orders, zone, nov + 86_400_000);
    let mut cold: BTreeMap<i64, DayCube> = BTreeMap::new();
    let mut row = DayCube::new(20261102);
    row.t = 2_000; // an archived day: the rotated orders' takings
    row.c.insert("ebills".into(), [3, 500]);
    row.c.insert("storefront".into(), [2, 1_500]);
    cold.insert(row.d, row);
    let receipts: Receipts = BTreeMap::from([(202611, (2, 2_300, 1)), (202610, (1, 10, 0))]);
    let expenses = vec![expense(20261103, 500, Kind::Tax), expense(20261130, 700, Kind::Tax), expense(20261103, 3_200, Kind::Other), expense(20261001, 40, Kind::Other)];
    let months = months_ending(202611, 2);
    let got = build(&months, &[&hot, &cold], &receipts, &expenses, &|m| Ok(if m == "2026-11" { 1_100 } else { 0 })).unwrap();
    let n = got.iter().find(|m| m.month == 202611).unwrap();
    assert_eq!((n.orders, n.till), (2_800 + 1_500, 1_500 + 700 + 500), "{n:?}");
    assert_eq!(n.plus(), 2_800 + 1_500 + 1_500 + 700 + 500, "web + archived web + till + archived till, each once");
    assert_eq!((n.purchases, n.tax, n.other, n.payroll), (2_300, 1_200, 3_200, 1_100));
    assert_eq!(n.minus(), 2_300 + 1_200 + 3_200 + 1_100);
    assert_eq!(n.net(), 7_000 - 7_800);
    assert_eq!((n.receipts, n.uncosted), (2, 1));
    let o = got.iter().find(|m| m.month == 202610).unwrap();
    assert_eq!((o.plus(), o.purchases, o.other, o.tax, o.payroll, o.net()), (111, 10, 40, 0, 0, 111 - 10 - 40), "{o:?}");
}

#[test]
fn a_month_with_nothing_is_zero_not_absent() {
    let got = build(&[202603], &[], &Receipts::new(), &[], &|_| Ok(0)).unwrap();
    assert_eq!((got[0].plus(), got[0].minus(), got[0].net()), (0, 0, 0));
}

#[test]
fn a_payroll_that_cannot_be_added_up_fails_the_month_and_never_counts_as_zero() {
    let got = build(&[202603], &[], &Receipts::new(), &[], &|_| Err("too big".to_string()));
    assert_eq!(got.err().as_deref(), Some("too big"));
}

// ── the routes ──────────────────────────────────────────────────────────────

fn at(slug: &str, p: &str) -> String {
    format!("https://{slug}.{PLATFORM_HOST}{p}")
}

fn add(site: &Site, owner: &str, slug: &str, body: Value) -> crate::wire::Reply {
    site.run(routes::add, post(&at(slug, &format!("/api/owner/finance/expenses?location_id={slug}")), &body).bearer(owner).on(slug), &[])
}

fn edit(site: &Site, owner: &str, slug: &str, id: &str, body: Value) -> crate::wire::Reply {
    site.run(routes::edit, post(&at(slug, &format!("/api/owner/finance/expenses/{id}?location_id={slug}")), &body).bearer(owner).on(slug), &[("id", id)])
}

fn del(site: &Site, owner: &str, slug: &str, id: &str) -> crate::wire::Reply {
    site.run(routes::remove, post(&at(slug, &format!("/api/owner/finance/expenses/{id}?location_id={slug}")), &json!({})).bearer(owner).on(slug), &[("id", id)])
}

fn read(site: &Site, owner: &str, month: &str) -> Value {
    let r = site.run(routes::finance, get(&at("alpha", &format!("/api/owner/finance?location_id=alpha&month={month}"))).bearer(owner).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    r.body_value()
}

/// `(site, owner of alpha, dish)`.
fn venue() -> (Site, String, String) {
    let site = Site::new();
    let (owner, dish) = open_venue(&site, "alpha", "a@x.test");
    (site, owner, dish)
}

#[test]
fn expenses_are_added_listed_by_month_totalled_edited_and_deleted() {
    let (site, a, _) = venue();
    let r = add(&site, &a, "alpha", json!({"date": "2023-11-03", "name": "ПДВ", "amount": 500, "kind": "tax"}));
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let id = r.body_value()["id"].as_str().unwrap().to_string();
    let r2 = add(&site, &a, "alpha", json!({"date": "2023-11-05", "name": "Прикраси на Хеловін", "amount": 3200, "kind": "other"}));
    assert_eq!(r2.status_code(), 200, "{}", r2.body_str());
    assert_ne!(r2.body_value()["id"], json!(id), "two entries in one millisecond are two entries");
    add(&site, &a, "alpha", json!({"date": "2023-10-30", "name": "Жовтень", "amount": 40, "kind": "other"}));
    let v = read(&site, &a, "2023-11");
    let names: Vec<&str> = v["expenses"].as_array().unwrap().iter().map(|e| e["name"].as_str().unwrap()).collect();
    assert_eq!(names, vec!["Прикраси на Хеловін", "ПДВ"], "this month's, newest first");
    assert_eq!((v["tax"]["total"].clone(), v["other"]["total"].clone()), (json!(500), json!(3200)), "{v}");
    assert_eq!(v["month"], "2023-11");
    assert_eq!(read(&site, &a, "2023-10")["other"]["total"], 40);

    assert_eq!(edit(&site, &a, "alpha", &id, json!({"date": "2023-11-03", "name": "ПДВ листопад", "amount": 600, "kind": "tax"})).status_code(), 200);
    let v = read(&site, &a, "2023-11");
    assert_eq!(v["tax"]["total"], 600);
    assert_eq!(v["expenses"][1]["name"], "ПДВ листопад");

    let gone = edit(&site, &a, "alpha", "nope", json!({"date": "2023-11-03", "name": "x", "amount": 1, "kind": "tax"}));
    assert_eq!(gone.status_code(), 404, "{}", gone.body_str());
    assert_eq!(del(&site, &a, "alpha", "nope").status_code(), 404);
    assert_eq!(del(&site, &a, "alpha", &id).status_code(), 200);
    assert_eq!(read(&site, &a, "2023-11")["tax"]["total"], 0);
}

#[test]
fn a_refused_entry_writes_nothing_and_its_twin_does() {
    let (site, a, _) = venue();
    for body in [
        json!({"date": "2023-11-03", "name": "", "amount": 500, "kind": "tax"}),
        json!({"date": "2023-11-03", "name": "x", "amount": 0, "kind": "tax"}),
        json!({"date": "2030-01-01", "name": "x", "amount": 5, "kind": "tax"}),
        json!({"date": "2023-11-03", "name": "x", "amount": 5, "kind": "bribe"}),
        json!({"date": "2023-11-03", "name": "x", "amount": 5, "kind": "tax", "extra": 1}),
    ] {
        let r = add(&site, &a, "alpha", body.clone());
        assert_eq!(r.status_code(), 400, "{body}: {}", r.body_str());
    }
    assert_eq!(read(&site, &a, "2023-11")["tax"]["total"], 0, "nothing was written");
    assert_eq!(add(&site, &a, "alpha", json!({"date": "2023-11-03", "name": "x", "amount": 5, "kind": "tax"})).status_code(), 200);
    assert_eq!(read(&site, &a, "2023-11")["tax"]["total"], 5);
}

#[test]
fn another_owner_cannot_read_write_or_edit_this_venues_money_and_its_own_owner_can() {
    let (site, a, _) = venue();
    let b = site.venue("beta", "b@x.test");
    let own = add(&site, &a, "alpha", json!({"date": "2023-11-03", "name": "ПДВ", "amount": 500, "kind": "tax"}));
    assert_eq!(own.status_code(), 200);
    let id = own.body_value()["id"].as_str().unwrap().to_string();
    let foreign = |s: u16| assert!(s == 401 || s == 403 || s == 404, "foreign owner got {s}");
    foreign(add(&site, &b, "alpha", json!({"date": "2023-11-03", "name": "x", "amount": 1, "kind": "tax"})).status_code());
    foreign(edit(&site, &b, "alpha", &id, json!({"date": "2023-11-03", "name": "x", "amount": 1, "kind": "tax"})).status_code());
    foreign(del(&site, &b, "alpha", &id).status_code());
    let r = site.run(routes::finance, get(&at("alpha", "/api/owner/finance?location_id=alpha&month=2023-11")).bearer(&b).on("alpha"), &[]);
    foreign(r.status_code());
    let anon = site.run(routes::finance, get(&at("alpha", "/api/owner/finance?location_id=alpha&month=2023-11")).on("alpha"), &[]);
    foreign(anon.status_code());
    // Beta's own id space does not hold alpha's entry: asking on its own venue finds nothing.
    assert_eq!(del(&site, &b, "beta", &id).status_code(), 404);
    assert_eq!(read(&site, &a, "2023-11")["tax"]["total"], 500, "alpha's entry stands");
    let (bad, good) = (site.run(routes::finance, get(&at("alpha", "/api/owner/finance?location_id=alpha&month=2023-13")).bearer(&a).on("alpha"), &[]), read(&site, &a, "2023-11"));
    assert_eq!(bad.status_code(), 400, "{}", bad.body_str());
    assert_eq!(good["month"], "2023-11");
}

fn supply(site: &Site, a: &str, name: &str) {
    let r = site.run(crate::services::operations::supplies::set_supply, post(&at("alpha", "/api/owner/supplies"), &json!({"id": name, "name": name, "unit": "g"})).bearer(a).on("alpha"), &[]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
}

fn stock(site: &Site, a: &str, body: Value) {
    let r = site.run(crate::services::operations::stock::stock_move, post(&at("alpha", "/api/owner/stock/received"), &body).bearer(a).on("alpha"), &[("kind", "received")]);
    assert_eq!(r.status_code(), 200, "{}", r.body_str());
}

/// EVERY LINE SHOWS WHERE ITS NUMBER COMES FROM, through the real routes: an accepted
/// order is plus; a delivery with a price is minus, one without says so; the expenses
/// subtract; nobody was paid, so payroll is zero; and the net is exact.
#[test]
fn the_months_plus_minus_and_net_come_from_the_real_stores() {
    let (site, a, dish) = venue();
    let p = place_pickup(&site, "alpha", &dish, 2);
    assert_eq!(p.status_code(), 200, "{}", p.body_str());
    let order = p.body_value()["id"].as_str().unwrap().to_string();
    let ok = site.run(crate::owner::order_action, post(&at("alpha", &format!("/api/owner/orders/{order}/action")), &json!({"location_id": "alpha", "action": "confirm"})).bearer(&a).on("alpha"), &[("id", &order)]);
    assert_eq!(ok.status_code(), 200, "{}", ok.body_str());
    supply(&site, &a, "salmon");
    stock(&site, &a, json!({"item": "salmon", "qty": 1000, "total": 1200}));
    stock(&site, &a, json!({"item": "salmon", "qty": 10}));
    add(&site, &a, "alpha", json!({"date": "2023-11-03", "name": "ПДВ", "amount": 500, "kind": "tax"}));
    add(&site, &a, "alpha", json!({"date": "2023-11-04", "name": "Бензин", "amount": 3200, "kind": "other"}));
    let v = read(&site, &a, "2023-11");
    let p = &v["profit"];
    let row = |side: &str, key: &str| p[side]["rows"].as_array().unwrap().iter().find(|r| r["key"] == key).unwrap_or_else(|| panic!("{key} in {p}")).clone();
    assert_eq!((row("plus", "orders")["amount"].clone(), row("plus", "till")["amount"].clone(), p["plus"]["total"].clone()), (json!(1800), json!(0), json!(1800)), "{p}");
    assert_eq!(row("minus", "purchases")["amount"], 1200, "{p}");
    assert_eq!((row("minus", "purchases")["receipts"].clone(), row("minus", "purchases")["uncosted"].clone()), (json!(2), json!(1)), "{p}");
    assert_eq!((row("minus", "taxes")["amount"].clone(), row("minus", "other")["amount"].clone(), row("minus", "payroll")["amount"].clone()), (json!(500), json!(3200), json!(0)), "{p}");
    assert_eq!((p["minus"]["total"].clone(), p["net"].clone()), (json!(4900), json!(1800 - 4900)), "{p}");
    let h = v["history"].as_array().unwrap();
    assert_eq!(h.len(), HISTORY, "{v}");
    assert_eq!((h[HISTORY - 1]["month"].clone(), h[HISTORY - 1]["net"].clone()), (json!("2023-11"), json!(-3100)), "the selected month is last: {v}");
    assert_eq!(v["currency"], "ALL");
}

/// THE PAYROLL IS THE REAL ONE: hours the owner enters come out of the month's net, and a
/// later entry moves it (the read depends on the payroll image, not a stale fold).
#[test]
fn hours_entered_in_the_payroll_tab_come_out_of_the_months_net() {
    use crate::services::operations::payroll::{set_entry, set_rate};
    let (site, a, _) = venue();
    let (_, ira) = site.staff_full("alpha", &a, "ira@x.test", "waiter");
    let ok = |r: crate::wire::Reply| assert_eq!(r.status_code(), 200, "{}", r.body_str());
    let entry = |date: &str, minutes: i64| site.run(set_entry, post(&at("alpha", "/api/owner/payroll/entry"), &json!({"staff_id": ira, "date": date, "minutes": minutes})).bearer(&a).on("alpha"), &[]);
    ok(site.run(set_rate, post(&at("alpha", "/api/owner/payroll/rate"), &json!({"staff_id": ira, "rate_minor": 250})).bearer(&a).on("alpha"), &[]));
    ok(entry("2023-11-13", 450)); // 7.5 h x 250
    let payroll = |v: &Value| v["profit"]["minus"]["rows"].as_array().unwrap().iter().find(|r| r["key"] == "payroll").unwrap()["amount"].clone();
    let v = read(&site, &a, "2023-11");
    assert_eq!((payroll(&v), v["profit"]["net"].clone()), (json!(1875), json!(-1875)), "{v}");
    ok(entry("2023-11-14", 60));
    let v = read(&site, &a, "2023-11");
    assert_eq!((payroll(&v), v["profit"]["net"].clone()), (json!(2125), json!(-2125)), "a new entry moves the month: {v}");
}

#[test]
fn no_costed_receipts_is_zero_with_a_reason_not_a_silent_blank() {
    let (site, a, _) = venue();
    let v = read(&site, &a, "2023-11");
    let row = v["profit"]["minus"]["rows"].as_array().unwrap().iter().find(|r| r["key"] == "purchases").unwrap().clone();
    assert_eq!((row["amount"].clone(), row["receipts"].clone(), row["uncosted"].clone()), (json!(0), json!(0), json!(0)), "{row}");
    assert_eq!(v["archive"]["error"], Value::Null);
    let _: Receipts = receipts_of(&dowiz_hub::stock::StockLog::create_sized(64 * 1024).unwrap().journal().unwrap(), dowiz_hub::tz::DEFAULT, 0).0;
}
