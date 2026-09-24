//! D12 (G5), natively: a wallet-paid round refunded gives the wallet its
//! money back, once, and only the wallet's share.

use super::*;
use crate::command::pay::wallet::pay;
use crate::command::pay::{PayIn, Room};
use crate::hubdo::OrderView;
use dowiz_kernel::ledger_account::{balance_of, top_up, Account};
use dowiz_kernel::money::{Currency, Money};

const NOW: i64 = 1_790_000_000_000;
const OPEN: Room<'static> = Room { open_till: Some("main"), venue_currency: "ALL" };

fn record(tx: &dowiz_kernel::ledger_account::Transaction, id: &str) -> String {
    json!({
        "id": id, "kind": crate::wallet::kind_str(tx.kind), "reverses": null, "memo": "card", "at_ms": 1,
        "postings": tx.postings.iter().map(|p| json!({
            "account": p.account.as_str(), "minor": p.amount.minor, "currency": p.amount.currency.code(),
        })).collect::<Vec<_>>(),
    })
    .to_string()
}

fn balance(rows: &[String]) -> i64 {
    let j = crate::wallet::journal_from(rows).unwrap();
    balance_of(&j, Account::Wallet(crate::wallet::id64("u1"))).unwrap().map_or(0, |m| m.minor)
}

/// A 1000 round paid 600 from u1's wallet and 400 in cash; the ledger after.
fn paid_round() -> (Value, Vec<String>) {
    let tx = top_up(crate::wallet::id64("tx_top"), crate::wallet::id64("u1"), Money::new(1000, Currency::All), "card").unwrap();
    let mut rows = vec![record(&tx, "tx_top")];
    let mut h = dowiz_hub::Hub::create_sized(64 * 1024).unwrap();
    let mut o = json!({"id": "r1", "status": "READY", "location_id": "v1", "currency": "ALL", "tip": 0, "total": 1000});
    let mut seq = 1;
    for (amount, method, at) in [(600, "wallet", NOW), (400, "cash", NOW + 1)] {
        let i = PayIn {
            order_id: "r1".into(), location_id: "v1".into(), amount, method: method.into(), by: "p1".into(),
            till_id: None, covers: None, currency: None, rate_ppm: None, tip: None,
            wallet: (method == "wallet").then(|| "u1".to_string()), base_seq: None, now_ms: at,
        };
        let v = OrderView { order_id: "r1".into(), kind: 3, seq, order_json: o.to_string() };
        let (next, _, s, d) = pay(&mut h, Some(&v), &i, &OPEN, &rows).expect("paid");
        rows.extend(d.map(|d| d.record));
        (o, seq) = (next, s);
    }
    (o, rows)
}

#[test]
fn a_refunded_wallet_payment_goes_back_to_the_wallet() {
    let (o, mut rows) = paid_round();
    assert_eq!(balance(&rows), 400, "600 spent of 1000");
    let back = reversals(&o, "v1", &rows, NOW + 10).expect("reversible");
    assert_eq!(back.len(), 1, "one wallet payment, one reversal (the cash is the drawer's)");
    assert!(back[0].record.contains("\"kind\":\"REFUND\""), "{}", back[0].record);
    rows.extend(back.into_iter().map(|d| d.record));
    assert_eq!(balance(&rows), 1000, "the wallet holds what it held before the round");
    // THE REPLAY: a completion seen twice reverses nothing twice.
    assert!(reversals(&o, "v1", &rows, NOW + 11).unwrap().is_empty());
}

/// The twin: a round with no wallet payment, or whose wallet leg was never
/// written, gives nothing back from any wallet.
#[test]
fn no_wallet_leg_no_reversal() {
    let (o, rows) = paid_round();
    let cash_only = json!({"id": "r2", "payments": [{"method": "cash", "amount": 700, "at": NOW}]});
    assert!(reversals(&cash_only, "v1", &rows, NOW).unwrap().is_empty());
    assert!(reversals(&o, "v1", &rows[..1], NOW).unwrap().is_empty(), "the SPEND never reached the ledger");
}
