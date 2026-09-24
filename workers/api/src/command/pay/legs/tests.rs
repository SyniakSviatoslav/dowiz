//! Law 12 and its repair: each finding beside the twin that holds.

use super::super::wallet::pay;
use super::super::Room;
use super::*;
use crate::hubdo::OrderView;
use dowiz_kernel::ledger_account::top_up;
use dowiz_kernel::money::{Currency, Money};
use serde_json::json;

const NOW: i64 = 1_790_000_000_000;
const OPEN: Room<'static> = Room { open_till: Some("main"), venue_currency: "ALL" };

fn round(id: &str, total: i64) -> Value {
    json!({ "id": id, "status": "READY", "location_id": "v1", "currency": "ALL",
            "items": [{"product_id": "m", "quantity": 1, "unit_price": total}],
            "subtotal": total, "discount": 0, "delivery_fee": 0, "tip": 0, "total": total })
}

fn topped(held: i64) -> String {
    let tx = top_up(crate::wallet::id64("tx_top"), crate::wallet::id64("u1"), Money::new(held, Currency::All), "card").unwrap();
    json!({ "id": "tx_top", "kind": crate::wallet::kind_str(tx.kind), "reverses": null, "memo": "card", "at_ms": 1,
            "postings": tx.postings.iter().map(|p| json!({"account": p.account.as_str(), "minor": p.amount.minor, "currency": p.amount.currency.code()})).collect::<Vec<_>>() })
    .to_string()
}

/// A wallet payment of `amount` on order `id` through the REAL pay path:
/// the paid order and the leg it wrote.
fn wallet_paid(id: &str, amount: i64, at: i64, ledger: &[String]) -> (Value, Debit) {
    let mut h = dowiz_hub::Hub::create_sized(64 * 1024).unwrap();
    let o = round(id, 1000);
    let v = OrderView { order_id: id.into(), kind: 3, seq: 1, order_json: o.to_string() };
    let input = PayIn {
        order_id: id.into(), location_id: "v1".into(), amount, method: "wallet".into(), by: "p1".into(),
        till_id: None, covers: None, currency: None, rate_ppm: None, tip: None, wallet: Some("u1".into()), now_ms: at,
    };
    let (order, _, _, d) = pay(&mut h, Some(&v), &input, &OPEN, ledger).expect("covered");
    (order, d.expect("a wallet leg"))
}

#[test]
fn a_wallet_paid_with_its_leg_holds_and_one_without_it_is_named_missing() {
    let ledger = vec![topped(1000)];
    let (order, leg) = wallet_paid("r1", 400, NOW, &ledger);
    let whole = [ledger.clone(), vec![leg.record.clone()]].concat();
    assert!(audit(&[order.clone()], &whole, "v1", "ALL").holds(), "the twin: leg present");
    let a = audit(&[order], &ledger, "v1", "ALL");
    assert_eq!(a.missing.len(), 1);
    assert_eq!((a.missing[0].tx_id.as_str(), a.missing[0].amount), (leg.tx_id.as_str(), 400), "the derived id names it");
}

/// THE REPAIR REBUILDS THE LOST LEG BYTE FOR BYTE, and a second repair
/// writes nothing: the id is derived, so the repair is idempotent.
#[test]
fn the_repair_rewrites_the_lost_leg_exactly_and_is_idempotent() {
    let ledger = vec![topped(1000)];
    let (order, lost) = wallet_paid("r1", 400, NOW, &ledger);
    let plan = repair(&[order.clone()], &ledger, "v1", "ALL").unwrap();
    assert_eq!(plan.write, vec![lost.clone()], "the same id, the same record");
    assert!(plan.refused.is_empty());
    let healed = [ledger, vec![plan.write[0].record.clone()]].concat();
    assert!(audit(&[order.clone()], &healed, "v1", "ALL").holds());
    assert!(repair(&[order], &healed, "v1", "ALL").unwrap().write.is_empty(), "nothing left to write");
}

/// NEVER NEGATIVE: two lost legs of 300 on a wallet holding 500 -- the first
/// is rewritten, the second REPORTED. Twin: 600 covers both.
#[test]
fn the_repair_refuses_a_leg_the_wallet_no_longer_covers() {
    let (a, _) = wallet_paid("r1", 300, NOW, &[topped(600)]);
    let (b, _) = wallet_paid("r2", 300, NOW + 1, &[topped(600)]);
    let plan = repair(&[a.clone(), b.clone()], &[topped(500)], "v1", "ALL").unwrap();
    assert_eq!(plan.write.len(), 1);
    assert_eq!(plan.refused.len(), 1);
    assert!(plan.refused[0].1.contains("never goes negative"), "{}", plan.refused[0].1);
    assert_eq!(repair(&[a, b], &[topped(600)], "v1", "ALL").unwrap().write.len(), 2, "the twin");
}

#[test]
fn a_spend_with_no_paid_is_an_orphan_and_a_wrong_amount_a_mismatch() {
    let ledger = vec![topped(1000)];
    let (order, leg) = wallet_paid("r1", 400, NOW, &ledger);
    // The leg with no order that paid it.
    let orphan = audit(&[round("r1", 1000)], &[ledger.clone(), vec![leg.record.clone()]].concat(), "v1", "ALL");
    assert_eq!(orphan.orphans.len(), 1, "{orphan:?}");
    // The leg for a different amount than the Paid.
    let wrong = leg.record.replace("-400", "-399");
    let mm = audit(&[order.clone()], &[ledger.clone(), vec![wrong]].concat(), "v1", "ALL");
    assert_eq!((mm.mismatched.len(), mm.missing.len()), (1, 0), "{mm:?}");
    // Twin: the right leg, the right order.
    assert!(audit(&[order], &[ledger, vec![leg.record]].concat(), "v1", "ALL").holds());
}

#[test]
fn only_wallet_payments_are_owed_a_leg() {
    let mut o = round("r1", 1000);
    o["payments"] = json!([{"method": "card", "amount": 1000, "at": NOW, "currency": "ALL"}]);
    assert!(expected(&[o.clone()], "v1", "ALL").is_empty());
    o["payments"][0]["method"] = json!("wallet");
    o["payments"][0]["wallet"] = json!("u1");
    assert_eq!(expected(&[o], "v1", "ALL").len(), 1);
}

/// THE ID THE GATE RECOMPUTES. `e2e/gates/conservation.mjs` law 12 derives
/// the same id in JavaScript and `conservation.prove.mjs` carries these two
/// numbers (computed a third time, in Python, when they were written): if
/// either side's hash moves, one of the three disagrees.
#[test]
fn the_derived_ids_are_the_ones_the_gate_recomputes() {
    assert_eq!(tx_id_of("stub-venue", "r1", 1_790_000_000_000), "tx_2e8dcd2f92ca8acb");
    assert_eq!(dowiz_kernel::ledger_account::Account::Wallet(crate::wallet::id64("u1")).as_str(), "wallet:631765120777144307");
}

/// THE OBJECT'S RETRY (`hubdo::room::retry_leg`): its put lost to a ledger
/// that moved. (a) A top-up landed in between: the retry, deciding against
/// the NEW ledger, writes exactly the leg the payment would have. (b) An
/// owner's repair already wrote it: the retry finds it and writes nothing.
#[test]
fn the_objects_retry_after_a_lost_put_writes_the_leg_once() {
    let ledger = vec![topped(500)];
    let (order, lost) = wallet_paid("r1", 400, NOW, &ledger);
    let second_top = topped(700).replace("tx_top", "tx_top2");
    let moved = [ledger.clone(), vec![second_top]].concat();
    let plan = repair(std::slice::from_ref(&order), &moved, "v1", "ALL").unwrap();
    assert_eq!(plan.write, vec![lost.clone()], "(a) the same id, the same record");
    let already = [ledger, vec![lost.record]].concat();
    let again = repair(std::slice::from_ref(&order), &already, "v1", "ALL").unwrap();
    assert!(again.write.is_empty() && again.refused.is_empty(), "(b) nothing to write");
}
