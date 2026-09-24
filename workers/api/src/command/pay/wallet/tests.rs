//! §2.3's CHECK, natively: a mixed tender on one bill, a wallet that cannot
//! go negative, and the one document's means.

use super::super::tender::{document_means, Means};
use super::super::{settles, PayIn, Room};
use super::*;
use crate::command::sitting::Round;
use dowiz_kernel::ledger_account::top_up;

const NOW: i64 = 1_790_000_000_000;
const OPEN: Room<'static> = Room { open_till: Some("main"), venue_currency: "ALL" };

fn round(total: i64) -> Value {
    json!({
        "id": "r1", "status": "READY", "location_id": "v1", "currency": "ALL",
        "items": [{"product_id": "maki", "quantity": 1, "unit_price": total, "name": "Maki"}],
        "subtotal": total, "discount": 0, "delivery_fee": 0, "tip": 0, "total": total,
        "fulfilment": {"kind": "dine_in", "table": "7"}, "sitting_id": "sit-000001"
    })
}

fn view(o: &Value, seq: u64) -> OrderView {
    OrderView { order_id: "r1".into(), kind: 3, seq, order_json: o.to_string() }
}

fn input(amount: i64, method: &str, at: i64) -> PayIn {
    PayIn {
        order_id: "r1".into(), location_id: "v1".into(), amount, method: method.into(), by: "p1".into(),
        till_id: None, covers: None, currency: None, rate_ppm: None, tip: None,
        wallet: (method == "wallet").then(|| "u1".to_string()), base_seq: None, now_ms: at,
    }
}

/// The ledger's records, as `wallet::top_up` writes them: `u1` holds `held`.
fn ledger(held: i64) -> Vec<String> {
    let tx = top_up(crate::wallet::id64("tx_top"), crate::wallet::id64("u1"), Money::new(held, Currency::All), "card").unwrap();
    vec![json!({
        "id": "tx_top", "kind": crate::wallet::kind_str(tx.kind), "reverses": null, "memo": "card", "at_ms": 1,
        "postings": tx.postings.iter().map(|p| json!({
            "account": p.account.as_str(), "minor": p.amount.minor, "currency": p.amount.currency.code(),
        })).collect::<Vec<_>>(),
    })
    .to_string()]
}

/// cash 2000 + card 1500 + wallet 500 on a 4000 bill: paid, three `Paid` in
/// order, ONE wallet leg, a fourth payment refused, and the document lists
/// three means summing to the bill.
#[test]
fn a_mixed_tender_settles_one_bill_with_one_wallet_leg_and_one_document() {
    let mut h = dowiz_hub::Hub::create_sized(64 * 1024).unwrap();
    let (mut o, mut seq) = (round(4000), 1u64);
    let mut legs = Vec::new();
    for (i, (amount, method)) in [(2000, "cash"), (1500, "card"), (500, "wallet")].into_iter().enumerate() {
        let (next, _, s, debit) = pay(&mut h, Some(&view(&o, seq)), &input(amount, method, NOW + i as i64), &OPEN, &ledger(500))
            .unwrap_or_else(|r| panic!("{method}: {r:?}"));
        legs.extend(debit);
        (o, seq) = (next, s);
    }
    assert_eq!(o["payment_status"], json!("paid"));
    assert_eq!(h.events().len(), 3, "three Paid");
    let methods: Vec<&str> = o["payments"].as_array().unwrap().iter().map(|p| p["method"].as_str().unwrap()).collect();
    assert_eq!(methods, ["cash", "card", "wallet"], "history in order");
    assert_eq!(legs.len(), 1, "one wallet leg");
    assert!(legs[0].record.contains("\"minor\":-500") && legs[0].record.contains("\"memo\":\"r1\""), "{}", legs[0].record);

    let fourth = pay(&mut h, Some(&view(&o, seq)), &input(1, "card", NOW + 9), &OPEN, &[]);
    assert!(matches!(fourth, Err(Refused::Conflict(_))), "a paid bill takes no fourth payment");

    let v = view(&o, seq);
    let means = document_means(&[Round { view: &v, order: o.clone() }]).expect("the sitting is closed");
    assert_eq!(means, vec![
        Means { method: "cash".into(), amount: 2000 },
        Means { method: "card".into(), amount: 1500 },
        Means { method: "wallet".into(), amount: 500 },
    ]);
    assert_eq!(means.iter().map(|m| m.amount).sum::<i64>(), 4000, "the means sum to the LEK total");
}

/// A wallet short by ONE is refused: no leg, no `Paid`, not even in memory.
/// Twin: the exact balance lands.
#[test]
fn a_wallet_short_by_one_is_refused_with_no_leg_and_no_paid() {
    let mut h = dowiz_hub::Hub::create_sized(64 * 1024).unwrap();
    let before = h.to_bytes();
    let r = pay(&mut h, Some(&view(&round(4000), 1)), &input(500, "wallet", NOW), &OPEN, &ledger(499));
    assert!(matches!(r, Err(Refused::Conflict(ref m)) if m.contains("never goes negative")), "{r:?}");
    assert_eq!(h.to_bytes(), before, "no Paid");
    let (_, _, _, debit) = pay(&mut h, Some(&view(&round(4000), 1)), &input(499, "wallet", NOW), &OPEN, &ledger(499)).expect("covered");
    assert!(debit.is_some());
}

/// An empty wallet, and a wallet nobody topped up, cover nothing.
#[test]
fn an_untouched_wallet_covers_nothing() {
    let mut h = dowiz_hub::Hub::create_sized(64 * 1024).unwrap();
    assert!(matches!(pay(&mut h, Some(&view(&round(4000), 1)), &input(1, "wallet", NOW), &OPEN, &[]), Err(Refused::Conflict(_))));
    // Other methods never read the ledger: a card lands with an empty one.
    assert!(pay(&mut h, Some(&view(&round(4000), 1)), &input(1, "card", NOW), &OPEN, &[]).unwrap().3.is_none());
}

/// A sitting still owing has no document yet; a tip is on it, in its means.
#[test]
fn no_document_until_the_bill_is_paid_and_a_tip_is_in_the_means() {
    let mut h = dowiz_hub::Hub::create_sized(64 * 1024).unwrap();
    let (half, _, s, _) = pay(&mut h, Some(&view(&round(1000), 1)), &input(400, "card", NOW), &OPEN, &[]).unwrap();
    let v = view(&half, s);
    assert_eq!(document_means(&[Round { view: &v, order: half.clone() }]), None, "still owed");
    let tipped = PayIn { tip: Some(100), ..input(600, "cash", NOW + 1) };
    let (done, _, s2, _) = pay(&mut h, Some(&view(&half, s)), &tipped, &OPEN, &[]).unwrap();
    let v2 = view(&done, s2);
    let means = document_means(&[Round { view: &v2, order: done.clone() }]).expect("closed");
    assert_eq!(means.iter().map(|m| m.amount).sum::<i64>(), 1100, "bill 1000 + tip 100");
    assert_eq!(settles(&done["payments"][1]), 700);
}

/// A TIP ON A WALLET PAYMENT IS REFUSED: the leg debits `amount`, so the tip
/// would settle money nobody paid. Nothing is written. Twin: the same wallet
/// payment with no tip lands, and a tip on the card beside it lands too.
#[test]
fn a_tip_on_a_wallet_payment_is_refused_and_writes_nothing() {
    let mut h = dowiz_hub::Hub::create_sized(64 * 1024).unwrap();
    let before = h.to_bytes();
    let tipped = PayIn { tip: Some(100), ..input(500, "wallet", NOW) };
    let r = pay(&mut h, Some(&view(&round(4000), 1)), &tipped, &OPEN, &ledger(1000));
    assert!(matches!(r, Err(Refused::Invalid(ref m)) if m.contains("a wallet pays the bill only")), "{r:?}");
    assert_eq!(h.to_bytes(), before, "no Paid");
    let (_, _, _, d) = pay(&mut h, Some(&view(&round(4000), 1)), &input(500, "wallet", NOW), &OPEN, &ledger(1000)).expect("no tip: lands");
    assert!(d.is_some());
    let card = PayIn { tip: Some(100), ..input(500, "card", NOW + 1) };
    assert!(pay(&mut h, Some(&view(&round(4000), 1)), &card, &OPEN, &[]).is_ok(), "a card carries a tip");
}
