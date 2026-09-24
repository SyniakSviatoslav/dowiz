//! P1 (audit §3c, G4 + G5's CHECK): MONEY IS CONSERVED ON ONE ROUND over
//! random sequences of pay (cash, card, wallet) / void / confirm / reject /
//! refund, each through the real decider over a real Hub and ledger.
//!
//! After EVERY step, accepted or refused:
//!   * Σ paid ≤ total, and `payment_status == "paid"` ⇔ Σ paid == total > 0 (D8);
//!   * a REJECTED or CANCELLED round, or a guest's round nobody confirmed,
//!     holds no money (D4, D9);
//!   * the drawer holds Σ cash taken − Σ cash handed back (D12, the till's
//!     `cash_payments`);
//!   * the wallet holds what it was topped up with − Σ spent + Σ handed back
//!     (D12, `refund::wallet::reversals` applied as the object applies them).
//!
//! 300 seeds. Following `gates-that-count-skips-as-passes`, the run also
//! asserts how many steps were EXECUTED (accepted) per kind, so a generator
//! that only ever produced refusals cannot pass.

use crate::command::advance::{self, AdvanceIn};
use crate::command::amend::{AmendIn, Op};
use crate::command::pay::{wallet, PayIn, Room};
use crate::command::refund::{self, RefundIn};
use crate::command::till::cash_payments;
use dowiz_hub::stock::StockLog;
use dowiz_hub::{EventKind, Hub};
use dowiz_kernel::ledger_account::{balance_of, top_up, Account};
use dowiz_kernel::money::{Currency, Money};
use serde_json::{json, Value};

const NOW: i64 = 1_790_000_000_000;
const OPEN: Room<'static> = Room { open_till: Some("main"), venue_currency: "ALL" };
const HELD: i64 = 5_000;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn upto(&mut self, n: i64) -> i64 {
        (self.next() % n.max(1) as u64) as i64
    }
}

fn ledger() -> Vec<String> {
    let tx = top_up(crate::wallet::id64("tx_top"), crate::wallet::id64("u1"), Money::new(HELD, Currency::All), "card").unwrap();
    vec![json!({
        "id": "tx_top", "kind": crate::wallet::kind_str(tx.kind), "reverses": null, "memo": "card", "at_ms": 1,
        "postings": tx.postings.iter().map(|p| json!({
            "account": p.account.as_str(), "minor": p.amount.minor, "currency": p.amount.currency.code(),
        })).collect::<Vec<_>>(),
    })
    .to_string()]
}

fn wallet_balance(rows: &[String]) -> i64 {
    let j = crate::wallet::journal_from(rows).unwrap();
    balance_of(&j, Account::Wallet(crate::wallet::id64("u1"))).unwrap().map_or(0, |m| m.minor)
}

fn order_of(h: &Hub) -> (crate::hubdo::OrderView, Value) {
    let c = dowiz_hub::room::view::current(h, "r1").expect("the round");
    let v = crate::hubdo::OrderView { order_id: c.order_id, kind: c.kind, seq: c.seq, order_json: c.order_json };
    let o = serde_json::from_str(&v.order_json).unwrap();
    (v, o)
}

fn sum_of(o: &Value, method: &str) -> i64 {
    o.get("payments").and_then(Value::as_array).into_iter().flatten()
        .filter(|p| p["method"] == method).filter_map(|p| p["amount"].as_i64()).sum()
}

/// Accepted steps per kind: pay cash, pay card, pay wallet, void, confirm,
/// reject, refund start, refund complete.
type Done = [u32; 8];

fn one_seed(seed: u64, done: &mut Done) {
    let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
    let (a, b) = (500 + r.upto(1500), 100 + r.upto(800));
    let guest = r.upto(2) == 0;
    let status = if guest || r.upto(2) == 0 { "PENDING" } else { "CONFIRMED" };
    let mut h = Hub::create_sized(128 * 1024).unwrap();
    let placed = json!({"id": "r1", "status": status, "location_id": "v1", "currency": "ALL", "created_at_ms": 1,
        "placed_by": if guest { "guest" } else { "p1" }, "tip": 0, "delivery_fee": 0, "discount": 0,
        "items": [{"product_id": "a", "quantity": 1, "unit_price": a, "name": "A"},
                  {"product_id": "b", "quantity": 1, "unit_price": b, "name": "B"}],
        "subtotal": a + b, "total": a + b});
    h.append(EventKind::Placed, "r1", &placed.to_string(), 100, [0; 32]).unwrap();
    let mut stock = StockLog::create_sized(64 * 1024).unwrap();
    let mut rows = ledger();
    let mut at = NOW;

    for _ in 0..10 {
        at += 1_000;
        let (v, o) = order_of(&h);
        let owed = o["total"].as_i64().unwrap() - crate::command::sitting::paid_of(&o);
        let pay_in = |method: &str, amount: i64| PayIn {
            order_id: "r1".into(), location_id: "v1".into(), amount, method: method.into(), by: "p1".into(),
            till_id: None, covers: None, currency: None, rate_ppm: None, tip: None,
            wallet: (method == "wallet").then(|| "u1".to_string()),
            base_seq: Some(v.seq), now_ms: at,
        };
        // The amounts that reach the edges: the whole of what is owed, what is
        // owed but the last line (a void then lands on exactly paid, D8), and
        // anything up to 100 past it (refused: Σ would pass the total).
        let last_line = o["items"].as_array().filter(|l| l.len() > 1).and_then(|l| l[1]["unit_price"].as_i64()).unwrap_or(0);
        let amount = match r.upto(3) {
            0 => owed,
            1 if owed > last_line => owed - last_line,
            _ => 1 + r.upto(owed.max(1) + 100),
        };
        let step = r.upto(8);
        let ok = match step {
            0 | 1 => {
                let method = if step == 0 { "cash" } else { "card" };
                crate::command::pay::decide(&mut h, Some(&v), &pay_in(method, amount), &OPEN).is_ok()
            }
            2 => match wallet::pay(&mut h, Some(&v), &pay_in("wallet", amount), &OPEN, &rows) {
                Ok((_, _, _, d)) => {
                    rows.extend(d.map(|d| d.record));
                    true
                }
                Err(_) => false,
            },
            3 => {
                let void = AmendIn { order_id: "r1".into(), location_id: "v1".into(), base_seq: v.seq,
                    ops: vec![Op::Remove { line: 1 }], by: "p1".into(), reason: Some("mistake".into()),
                    may_void: true, boms: vec![], now_ms: at };
                crate::command::amend::decide(&mut h, &mut stock, Some(&v), &void).is_ok()
            }
            4 | 5 => {
                let next = if step == 4 { "CONFIRMED" } else { "REJECTED" };
                let i = AdvanceIn { order_id: "r1".into(), location_id: "v1".into(), next: next.into(), reason: None, now_ms: at };
                advance::decide(&mut h, &mut stock, Some(&v.order_json), &i).is_ok()
            }
            _ => {
                let complete = o["status"] == "REFUNDING";
                let i = RefundIn { order_id: "r1".into(), location_id: "v1".into(), by: "p1".into(),
                    reason: "venue_cancelled".into(), complete, now_ms: at, at_door: false, note: None };
                let landed = refund::decide(&mut h, &mut stock, Some(&v), &i, "ALL").is_ok();
                if landed && complete {
                    // What the object does after the log (`hubdo::refund`).
                    let (_, after) = order_of(&h);
                    let back = refund::wallet::reversals(&after, "v1", &rows, at).expect("reversible");
                    rows.extend(back.into_iter().map(|d| d.record));
                }
                landed
            }
        };
        if ok {
            let k = match step { 6 | 7 if o["status"] == "REFUNDING" => 7, 6 | 7 => 6, 5 => 5, 4 => 4, s => s as usize };
            done[k] += 1;
        }
        check(seed, &h, &rows);
    }
}

fn check(seed: u64, h: &Hub, rows: &[String]) {
    let (_, o) = order_of(h);
    let (total, paid) = (o["total"].as_i64().unwrap(), crate::command::sitting::paid_of(&o));
    let st = o["status"].as_str().unwrap_or("");
    let say = || format!("seed {seed}: {o}");
    assert!(paid <= total, "Σ paid past the total: {}", say());
    assert_eq!(o["payment_status"] == "paid", paid > 0 && paid == total, "paid stamp disagrees with Σ: {}", say());
    if matches!(st, "REJECTED" | "CANCELLED") || (st == "PENDING" && o["placed_by"] == "guest") {
        assert_eq!(paid, 0, "money on a round that is not the venue's: {}", say());
    }
    let handed_back = o.pointer("/refund/returned/at").is_some();
    let drawer: i64 = cash_payments(&[o.clone()], "v1", "ALL").iter().map(|c| c.amount).sum();
    let cash = sum_of(&o, "cash");
    assert_eq!(drawer, if handed_back { 0 } else { cash }, "drawer: {}", say());
    let spent = sum_of(&o, "wallet");
    assert_eq!(wallet_balance(rows), HELD - if handed_back { 0 } else { spent }, "wallet: {}", say());
}

#[test]
fn money_is_conserved_on_a_round_over_300_random_histories() {
    let mut done: Done = [0; 8];
    for seed in 1..=300 {
        one_seed(seed, &mut done);
    }
    eprintln!("P1 accepted steps [cash, card, wallet, void, confirm, reject, refund start, refund complete]: {done:?}");
    // THE RUN MEASURED SOMETHING: every kind of step was executed, not only refused.
    let names = ["cash", "card", "wallet", "void", "confirm", "reject", "refund start", "refund complete"];
    for (k, n) in done.iter().enumerate() {
        assert!(*n >= 5, "only {n} accepted `{}` steps in 300 seeds: {done:?}", names[k]);
    }
    assert!(done.iter().sum::<u32>() >= 600, "{done:?}");
}
