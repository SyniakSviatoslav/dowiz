//! Wallet-leg findings as exception rows, and the first-sight alert, each
//! beside its twin. The orders and legs come from the REAL wallet pay path;
//! the findings from the REAL `legs::audit` / `legs::repair`.

use super::*;
use crate::command::pay::wallet::pay;
use crate::command::pay::{PayIn, Room};
use crate::exceptions::alert::Voice;
use crate::hubdo::OrderView;
use dowiz_kernel::ledger_account::top_up;
use dowiz_kernel::money::{Currency, Money};
use serde_json::json;
use std::collections::HashSet;

const NOW: i64 = 1_790_000_000_000;
const OPEN: Room<'static> = Room { open_till: Some("main"), venue_currency: "ALL" };

fn topped(held: i64) -> String {
    let tx = top_up(crate::wallet::id64("tx_top"), crate::wallet::id64("u1"), Money::new(held, Currency::All), "card").unwrap();
    json!({ "id": "tx_top", "kind": crate::wallet::kind_str(tx.kind), "reverses": null, "memo": "card", "at_ms": 1,
            "postings": tx.postings.iter().map(|p| json!({"account": p.account.as_str(), "minor": p.amount.minor, "currency": p.amount.currency.code()})).collect::<Vec<_>>() })
    .to_string()
}

/// A wallet payment by `p1` through the real pay path: the order and its leg record.
fn paid(id: &str, amount: i64, at: i64, ledger: &[String]) -> (Value, String, String) {
    let mut h = dowiz_hub::Hub::create_sized(64 * 1024).unwrap();
    let o = json!({ "id": id, "status": "READY", "location_id": "v1", "currency": "ALL",
        "items": [{"product_id": "m", "quantity": 1, "unit_price": 1000}],
        "subtotal": 1000, "discount": 0, "delivery_fee": 0, "tip": 0, "total": 1000 });
    let v = OrderView { order_id: id.into(), kind: 3, seq: 1, order_json: o.to_string() };
    let input = PayIn {
        order_id: id.into(), location_id: "v1".into(), amount, method: "wallet".into(), by: "p1".into(),
        till_id: None, covers: None, currency: None, rate_ppm: None, tip: None, wallet: Some("u1".into()), base_seq: None, now_ms: at,
    };
    let (order, _, _, d) = pay(&mut h, Some(&v), &input, &OPEN, ledger).expect("covered");
    let d = d.expect("a wallet leg");
    (order, d.tx_id, d.record)
}

fn kinds(rows: &[Row]) -> Vec<&'static str> {
    rows.iter().map(|r| r.kind).collect()
}

#[test]
fn a_lost_leg_is_a_missing_row_naming_the_paid_and_a_present_one_is_nothing() {
    let ledger = vec![topped(1000)];
    let (order, tx, leg) = paid("r1", 400, NOW, &ledger);
    let rows = leg_rows(&[order.clone()], &ledger, "v1", "ALL").unwrap();
    assert_eq!(kinds(&rows), vec![LEG_MISSING]);
    let r = &rows[0];
    assert_eq!((r.order_id.as_deref(), r.tx_id.as_deref(), r.amount, r.by.as_str(), r.at), (Some("r1"), Some(tx.as_str()), 400, "p1", NOW));
    // Twin: the leg is in the ledger.
    assert!(leg_rows(&[order], &[ledger, vec![leg]].concat(), "v1", "ALL").unwrap().is_empty());
}

#[test]
fn a_leg_the_wallet_no_longer_covers_is_refused_with_why_and_covered_ones_are_missing() {
    let (a, _, _) = paid("r1", 300, NOW, &[topped(600)]);
    let (b, _, _) = paid("r2", 300, NOW + 1, &[topped(600)]);
    let rows = leg_rows(&[a.clone(), b.clone()], &[topped(500)], "v1", "ALL").unwrap();
    assert_eq!(kinds(&rows), vec![LEG_MISSING, LEG_REFUSED]);
    assert!(rows[1].reason.as_deref().is_some_and(|w| w.contains("never goes negative")), "{rows:?}");
    // Twin: 600 covers both, so neither is refused.
    assert_eq!(kinds(&leg_rows(&[a, b], &[topped(600)], "v1", "ALL").unwrap()), vec![LEG_MISSING, LEG_MISSING]);
}

#[test]
fn a_spend_with_no_paid_is_an_orphan_row_and_a_wrong_amount_a_mismatch() {
    let ledger = vec![topped(1000)];
    let (order, tx, leg) = paid("r1", 400, NOW, &ledger);
    let mut unpaid = order.clone();
    unpaid["payments"] = json!([]);
    let rows = leg_rows(&[unpaid], &[ledger.clone(), vec![leg.clone()]].concat(), "v1", "ALL").unwrap();
    assert_eq!(kinds(&rows), vec![LEG_ORPHAN]);
    assert_eq!((rows[0].tx_id.as_deref(), rows[0].amount, rows[0].order_id.as_deref()), (Some(tx.as_str()), 400, None));
    let wrong = leg.replace("-400", "-399");
    let rows = leg_rows(&[order.clone()], &[ledger.clone(), vec![wrong]].concat(), "v1", "ALL").unwrap();
    assert_eq!(kinds(&rows), vec![LEG_MISMATCHED]);
    assert_eq!(rows[0].by, "p1");
}

#[test]
fn a_ledger_that_does_not_replay_is_an_error_not_no_findings() {
    let (order, _, _) = paid("r1", 400, NOW, &[topped(1000)]);
    // The top-up's postings no longer balance: the journal refuses to replay.
    let broken = topped(1000).replacen("1000", "999", 1);
    assert!(leg_rows(&[order], &[broken], "v1", "ALL").is_err());
}

fn voice() -> Voice<'static> {
    Voice { venue: "v1", zone: dowiz_hub::tz::zone("Europe/Tirane").unwrap(), lang: "en" }
}

/// FIRST SIGHT, ONCE: the first finding alerts with no threshold; once its
/// marker is kept it is quiet; a SECOND finding alerts alone.
#[test]
fn the_first_lost_leg_alerts_once_and_a_second_one_alerts_alone() {
    let ledger = vec![topped(1000)];
    let (a, tx_a, _) = paid("r1", 400, NOW, &ledger);
    let rows = leg_rows(&[a.clone()], &ledger, "v1", "ALL").unwrap();
    let none = HashSet::<String>::new();
    let (got, marks) = first(&rows, &|id| none.contains(id), NOW + 5, &voice(), "chat-1");
    assert_eq!(got.len(), 1, "one finding is enough: no threshold");
    assert_eq!((got[0].to.as_str(), got[0].id.clone()), ("chat-1", format!("exceptions/first/{LEG_MISSING}/{tx_a}")));
    assert!(got[0].text.contains("wallet leg missing") && got[0].text.contains("r1") && got[0].text.contains("by p1"), "{}", got[0].text);
    assert_eq!(marks, vec![got[0].id.clone()]);
    // Marked: quiet.
    let kept: HashSet<String> = marks.into_iter().collect();
    assert!(first(&rows, &|id| kept.contains(id), NOW + 9, &voice(), "chat-1").0.is_empty(), "told once");
    // A second lost leg: only it is new.
    let (b, tx_b, _) = paid("r2", 300, NOW + 20, &ledger);
    let rows = leg_rows(&[a, b], &ledger, "v1", "ALL").unwrap();
    let (got, marks) = first(&rows, &|id| kept.contains(id), NOW + 30, &voice(), "chat-1");
    assert_eq!(marks, vec![format!("exceptions/first/{LEG_MISSING}/{tx_b}")]);
    assert!(got[0].text.contains("r2") && !got[0].text.contains("r1"), "{}", got[0].text);
    // No chat: nobody to tell, and nothing is marked.
    assert_eq!(first(&rows, &|_| false, NOW, &voice(), " "), (vec![], vec![]));
}
