//! THE IMPORT'S RULES, against a real `Hub` and `StockLog` in memory and
//! sales mapped by the real mapper (`map::to_order` / `to_paid`).

use super::super::map::{to_order, to_paid};
use super::super::parse_detail;
use super::super::wire::Sale;
use super::*;
use dowiz_hub::stock::{StockLevel, StockLog};
use serde_json::json;

pub(super) const LOC: &str = "loc-dubin";
pub(super) const T0: i64 = 1_789_981_285_000; // 2026-09-21T09:01:25Z
pub(super) const ROLL: &str = r#"{"id":"p-roll","name":"Sake roll","price":250,"bom":[{"supply":"salmon","qty":40}]}"#;

pub(super) struct Line(pub(super) &'static str, pub(super) &'static str, pub(super) i64, pub(super) i64);

/// A detail answer in the measured shape (§1.5), identifiers synthetic.
#[allow(clippy::too_many_arguments)]
pub(super) fn sale(id: i64, table: Option<&str>, suo: Option<i64>, summary: bool, lines: &[Line], secs: i64, logs: usize) -> Sale {
    let total: i64 = lines.iter().map(|l| l.2 * l.3).sum();
    let recs: Vec<Value> = lines
        .iter()
        .map(|l| json!({"itemName": l.1, "amount": l.2 as f64, "price": l.3 as f64, "totalValue": (l.2 * l.3) as f64,
                         "vat": "VAT_20", "discount": 0.0, "itemInSale": {"itemCode": l.0}}))
        .collect();
    let log: Vec<Value> = (0..logs).map(|_| json!({"iic": "IIC", "fic": "FIC", "status": "SUCCESS"})).collect();
    let at = T0 + secs * 1000;
    let t = (at / 1000).rem_euclid(86_400);
    let stamp = format!("{}T{:02}:{:02}:{:02}Z", super::super::time::day_of(at), t / 3600, t % 3600 / 60, t % 60);
    let v = json!({
        "id": id, "invOrdNum": id - 4, "uuid": format!("00000000-0000-4000-8000-{id:012}"), "fic": "FIC",
        "timestamp": stamp, "status": "CLOSED", "fiscalSatus": "FINISHED", "draft": 0,
        "summaryInvoice": summary, "paymentMethod": "CASH", "totalValue": total as f64, "currencyRate": 1.0,
        "currency": {"currencyCode": "ALL"}, "pointOfSale": {"id": 1}, "extraUser": {"id": 1},
        "saleUnit": table.map(|t| json!({"identifier": t, "type": "TABLE"})),
        "saleUnitOrder": suo.map(|s| json!({"id": s, "status": "COMPLETED"})),
        "saleRecords": recs, "logCis": log,
    });
    parse_detail(&json!({ "sale": v }).to_string()).expect("fixture parses")
}

pub(super) fn course(id: i64, table: &str, suo: i64, lines: &[Line], secs: i64) -> Mapped {
    Mapped::Order { sale_id: id, envelope: to_order(&sale(id, Some(table), Some(suo), false, lines, secs, 1), LOC).unwrap() }
}

pub(super) fn bill(id: i64, table: &str, lines: &[Line], secs: i64) -> Mapped {
    Mapped::Bill { sale_id: id, paid: to_paid(&sale(id, Some(table), None, true, lines, secs, 1)).unwrap() }
}

pub(super) struct Venue {
    pub(super) hub: Hub,
    pub(super) stock: StockLog,
    pub(super) map: Vec<(String, String)>,
    pub(super) waiting: Waiting,
}

impl Venue {
    pub(super) fn new() -> Venue {
        Venue { hub: Hub::create_sized(64 * 1024).unwrap(), stock: StockLog::create_sized(64 * 1024).unwrap(), map: vec![], waiting: Waiting::default() }
    }
    pub(super) fn import(&mut self, sales: &[Mapped], now: i64) -> Outcome {
        let listed: Vec<OrderView> = crate::hubstore::orders_state(&self.hub).into_iter().map(OrderView::of_event).collect();
        let map = |c: &str| self.map.iter().find(|(k, _)| k == c).map(|(_, v)| v.clone());
        let product = |p: &str| (p == "p-roll").then(|| ROLL.to_string());
        let look = Lookups { map: &map, product: &product };
        let out = decide(&mut self.hub, &mut self.stock, &listed, &look, std::mem::take(&mut self.waiting), sales, now).unwrap();
        self.waiting = Waiting { bills: out.pending.clone(), leads: out.leads.clone() };
        out
    }
    pub(super) fn order(&self, id: &str) -> Value {
        let v = crate::hubstore::orders_state(&self.hub).into_iter().find(|e| e.order_id == id).expect("held");
        serde_json::from_str(&v.order_json).unwrap()
    }
    pub(super) fn orders(&self) -> usize {
        crate::hubstore::orders_state(&self.hub).len()
    }
}

pub(super) fn id_of(m: &Mapped) -> String {
    match m {
        Mapped::Order { envelope, .. } => envelope["id"].as_str().unwrap().to_string(),
        _ => unreachable!(),
    }
}

/// THE DUPLICATE CHECK: the same sale twice is ONE order, and the second
/// import writes nothing -- the image's bytes do not move.
#[test]
fn the_same_sale_twice_is_one_order_and_the_bytes_stand() {
    let mut v = Venue::new();
    let c = course(8689, "12", 5614, &[Line("56", "korca", 2, 130)], 0);
    let first = v.import(std::slice::from_ref(&c), T0 + 60_000);
    assert_eq!((first.placed, first.written.len()), (1, 1));
    let bytes = v.hub.to_bytes_trimmed();
    let again = v.import(std::slice::from_ref(&c), T0 + 360_000);
    assert_eq!((again.placed, again.noted, again.unchanged, again.written.len()), (0, 0, 1, 0));
    assert_eq!(v.hub.to_bytes_trimmed(), bytes, "a repeat writes nothing at all");
    assert_eq!(v.orders(), 1);
    let o = v.order(&id_of(&c));
    assert_eq!((o["status"].as_str(), o["channel"].as_str(), o["total"].as_i64()), (Some("PICKED_UP"), Some("ebills"), Some(260)));
}

/// AMENDED IN EBILLS: a `Noted` with the new figures beside the order, the
/// order's own money untouched; the same amendment again is silence.
#[test]
fn an_amended_sale_is_noted_once_and_its_money_does_not_move() {
    let mut v = Venue::new();
    let c = course(8689, "12", 5614, &[Line("56", "korca", 2, 130)], 0);
    v.import(std::slice::from_ref(&c), T0);
    let mut amended = to_order(&sale(8689, Some("12"), Some(5614), false, &[Line("56", "korca", 3, 130)], 0, 2), LOC).unwrap();
    amended["id"] = json!(id_of(&c));
    let m = Mapped::Order { sale_id: 8689, envelope: amended };
    let out = v.import(std::slice::from_ref(&m), T0 + 600_000);
    assert_eq!((out.placed, out.noted), (0, 1));
    assert_eq!(out.written[0].0, EventKind::Noted);
    let o = v.order(&id_of(&c));
    assert_eq!(o["total"], json!(260), "Noted promises no money moved");
    assert_eq!(o["ebills_revision"]["total"], json!(390));
    assert_eq!(o["ebills_revision"]["log_cis_len"], json!(2));
    assert_eq!(o["ebills_changed"], json!(true));
    let again = v.import(std::slice::from_ref(&m), T0 + 900_000);
    assert_eq!((again.noted, again.unchanged), (0, 1), "the same amendment is not news twice");
    // The daily re-read's fingerprint path says the same.
    let seen = Mapped::Seen { sale_id: 8689, order_id: id_of(&c), total: 390, log_cis_len: 3 };
    assert_eq!(v.import(&[seen], T0 + 999_000).noted, 1);
}

/// A BILL IS NEVER AN ORDER: it is `Paid` on the courses of its sitting
/// (260 + 4600 = 4860, §1.6), and it never enters `payments[]` (law 10).
#[test]
fn a_bill_pays_its_courses_and_is_never_an_order() {
    let mut v = Venue::new();
    let a = course(8689, "12", 5614, &[Line("56", "korca", 2, 130)], 0);
    let b = course(8690, "12", 5614, &[Line("7", "sushi set", 2, 2300)], 14);
    let other = course(8688, "3", 5600, &[Line("56", "korca", 1, 130)], 0);
    let out = v.import(&[a.clone(), b.clone(), other.clone(), bill(8691, "12", &[Line("56", "korca", 2, 130), Line("7", "sushi set", 2, 2300)], 5331)], T0 + 5_400_000);
    assert_eq!((out.placed, out.paid), (3, 2));
    assert_eq!(v.orders(), 3, "the bill added no order");
    for c in [&a, &b] {
        let o = v.order(&id_of(c));
        assert_eq!(o["payment_status"], json!("paid"));
        assert_eq!(o["bill"]["total"], json!(4860));
        assert_eq!(o["bill"]["courses"], json!(2));
        assert!(o.get("payments").is_none(), "the till's cash is not dowiz's drawer");
    }
    assert!(v.order(&id_of(&other)).get("bill").is_none(), "another table's course is not on this bill");
    // The same bill again pays nothing twice.
    let again = v.import(&[bill(8691, "12", &[Line("56", "korca", 2, 130), Line("7", "sushi set", 2, 2300)], 5331)], T0 + 5_700_000);
    assert_eq!((again.paid, again.written.len()), (0, 0));
}

/// A bill whose courses have not arrived waits; when they do, it joins. One
/// that never finds them is given up on BY NAME after two days.
#[test]
fn a_bill_waits_for_its_courses_and_gives_up_loudly() {
    let mut v = Venue::new();
    let lines = [Line("56", "korca", 1, 160)];
    let out = v.import(&[bill(8692, "18", &lines, 7000)], T0 + 7_200_000);
    assert_eq!((out.paid, out.pending.len()), (0, 1));
    let out = v.import(&[course(8687, "18", 5620, &lines, 6000)], T0 + 7_500_000);
    assert_eq!((out.placed, out.paid, out.pending.len()), (1, 1, 0));

    let mut lonely = Venue::new();
    lonely.import(&[bill(8700, "4", &lines, 7000)], T0);
    let out = lonely.import(&[], T0 + PENDING_FOR_MS + 1);
    assert!(out.pending.is_empty());
    assert_eq!(out.refused.len(), 1);
    assert!(out.refused[0].why.contains("never arrived"));
}

/// THE CROSSWALK: unmatched lines keep `ebills:<code>` and draw nothing;
/// once the owner maps the code, the next course draws its BOM as `Served`
/// -- past an empty shelf, recorded, never refused (§6.4).
#[test]
fn a_mapped_code_draws_the_shelf_and_an_unmatched_one_does_not() {
    let mut v = Venue::new();
    let before = v.stock.len();
    let c1 = course(8689, "1", 1, &[Line("56", "Sake roll", 2, 250)], 0);
    let out = v.import(std::slice::from_ref(&c1), T0);
    assert_eq!(v.stock.len(), before, "unmatched: no stock event at all");
    assert_eq!(v.order(&id_of(&c1))["items"][0]["product_id"], json!("ebills:56"));
    assert_eq!(out.seen, vec![("56".to_string(), "Sake roll".to_string(), 250)]);

    v.map.push(("56".into(), "p-roll".into()));
    v.stock.append(&StockEvent::Received { item: "salmon".into(), qty: 100 }).unwrap();
    let c2 = course(8690, "1", 2, &[Line("56", "Sake roll", 3, 250)], 60);
    v.import(std::slice::from_ref(&c2), T0 + 60_000);
    let o = v.order(&id_of(&c2));
    assert_eq!((o["items"][0]["product_id"].as_str(), o["items"][0]["ebills_code"].as_str()), (Some("p-roll"), Some("56")));
    let led = v.stock.ledger().unwrap();
    assert_eq!(led.level("salmon"), StockLevel { on_hand: -20, reserved: 0 }, "3 x 40 g served from 100 g");
    assert_eq!(dowiz_hub::stock::short(&led), vec![("salmon".to_string(), -20)]);
    assert!(led.stranded().is_empty(), "a served dish holds nothing");
}

/// LAW 3 on what the importer writes: total = Σ unit_price × quantity + fee
/// + tip − discount, read the way `e2e/gates/conservation.mjs` reads it --
/// here on the measured free line (`price 0, discount 100%`).
#[test]
fn every_imported_order_satisfies_law_3() {
    let mut v = Venue::new();
    let mut s = sale(8801, Some("9"), Some(7), false, &[Line("1", "a", 2, 500), Line("164", "free", 1, 0)], 0, 1);
    if let Some(r) = s.sale_records.as_mut() {
        r[1].discount = Some(100.0);
    }
    let env = to_order(&s, LOC).unwrap();
    v.import(&[Mapped::Order { sale_id: 8801, envelope: env.clone() }], T0);
    let o = v.order(env["id"].as_str().unwrap());
    let lines: i64 = o["items"].as_array().unwrap().iter().map(|i| i["unit_price"].as_i64().unwrap() * i["quantity"].as_i64().unwrap()).sum();
    let expect = lines + o["delivery_fee"].as_i64().unwrap() + o["tip"].as_i64().unwrap() - o["discount"].as_i64().unwrap();
    assert_eq!((o["total"].as_i64(), expect), (Some(1000), 1000));
}

/// THE VOID, AS MEASURED ON TABLE 10: a course (+4600), its cancellation
/// (-4600, naming the course), and a bill of 0. Both are orders, they net to
/// zero, the bill pays both, and the voided course says who voided it.
#[test]
fn a_void_nets_its_course_to_zero_and_the_bill_of_zero_pays_both() {
    let mut v = Venue::new();
    let course = sale(8717, Some("10"), Some(5685), false, &[Line("117", "set", 1, 4600)], 0, 1);
    let mut void = sale(8718, Some("10"), Some(5686), false, &[Line("117", "set", 1, 4600)], 60, 1);
    void.changed_status = Some("CANCELLED".into());
    void.modified = Some(super::super::wire::Modified { id: 8717, uuid: course.uuid.clone() });
    if let Some(r) = void.sale_records.as_mut() {
        (r[0].amount, r[0].total_value) = (-1.0, -4600.0);
    }
    void.total_value = -4600.0;
    let (c, x) = (to_order(&course, LOC).unwrap(), to_order(&void, LOC).unwrap());
    let out = v.import(
        &[Mapped::Order { sale_id: 8717, envelope: c.clone() }, Mapped::Order { sale_id: 8718, envelope: x.clone() }, bill(8719, "10", &[], 120)],
        T0 + 200_000,
    );
    assert_eq!((out.placed, out.noted, out.paid, out.pending.len()), (2, 1, 2, 0));
    let (co, xo) = (v.order(c["id"].as_str().unwrap()), v.order(x["id"].as_str().unwrap()));
    assert_eq!(co["total"].as_i64().unwrap() + xo["total"].as_i64().unwrap(), 0);
    assert_eq!(co["ebills_voided_by"]["sale_id"], json!(8718));
    assert_eq!((co["bill"]["total"].as_i64(), xo["payment_status"].as_str()), (Some(0), Some("paid")));
}

/// THE WEEK'S RE-READ OF BILLS: a bill older than the import is checked and
/// dropped, NEVER queued (a week of them would flood the waiting list); a
/// bill already applied and since changed in ebills is `Noted` on its courses.
#[test]
fn a_rechecked_bill_is_never_queued_and_an_amended_one_is_noted() {
    let mut v = Venue::new();
    let old = match bill(8001, "5", &[Line("1", "x", 1, 300)], 0) {
        Mapped::Bill { sale_id, paid } => Mapped::Recheck { sale_id, paid },
        _ => unreachable!(),
    };
    let out = v.import(std::slice::from_ref(&old), T0);
    assert_eq!((out.pending.len(), out.written.len(), out.refused.len()), (0, 0, 0));

    let c = course(8689, "12", 5614, &[Line("56", "korca", 2, 130)], 0);
    v.import(&[c.clone(), bill(8691, "12", &[Line("56", "korca", 2, 130)], 600)], T0 + 700_000);
    assert_eq!(v.order(&id_of(&c))["bill"]["total"], json!(260));
    let Mapped::Bill { paid, .. } = bill(8691, "12", &[Line("56", "korca", 2, 130)], 600) else { unreachable!() };
    let mut amended = paid.clone();
    amended["external"]["log_cis_len"] = json!(2);
    let out = v.import(&[Mapped::Recheck { sale_id: 8691, paid: amended.clone() }], T0 + 900_000);
    assert_eq!((out.noted, out.paid), (1, 0));
    assert_eq!(v.order(&id_of(&c))["ebills_bill_revision"]["log_cis_len"], json!(2));
    let again = v.import(&[Mapped::Recheck { sale_id: 8691, paid: amended }], T0 + 950_000);
    assert_eq!((again.noted, again.unchanged), (0, 1));
}
