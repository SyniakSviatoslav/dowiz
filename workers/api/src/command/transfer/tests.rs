//! G4 (BLUEPRINT-POS-THE-ROOM §6): a transfer conserves the lines and the
//! money over `{from, to}`, each round still obeys law 3, the shelf follows
//! the lines before the kitchen, and every refusal writes NOTHING to either
//! image. Every refusal has the positive twin beside it.

use super::*;
use dowiz_hub::stock::{StockEvent, StockLog};

const NOW: i64 = 1_790_000_000_000;
const SEQ: u64 = 1_789_999_000_000;

fn dish(pid: &str, supply: &str, qty: i64) -> (String, String) {
    (pid.into(), json!({ "id": pid, "bom": [{ "supply": supply, "qty": qty }] }).to_string())
}

fn boms() -> Vec<(String, String)> {
    vec![dish("maki", "rice", 100), dish("beer", "keg", 1)]
}

fn round(id: &str, status: &str) -> Value {
    json!({
        "id": id, "status": status, "location_id": "v1",
        "items": [
            {"product_id": "maki", "quantity": 2, "unit_price": 600, "name": "Maki"},
            {"product_id": "beer", "quantity": 1, "unit_price": 300, "name": "Beer"}
        ],
        "subtotal": 1500, "discount": 0, "delivery_fee": 0, "tip": 0, "total": 1500,
        "fulfilment": {"kind": "dine_in", "table": "7"}, "sitting_id": "sit-000001"
    })
}

fn view(order: &Value) -> OrderView {
    let id = order["id"].as_str().unwrap_or("r1");
    OrderView { order_id: id.into(), kind: 1, seq: SEQ, order_json: order.to_string() }
}

fn input(lines: Vec<usize>) -> TransferIn {
    TransferIn {
        from_order_id: "r1".into(), to_order_id: "r2".into(), location_id: "v1".into(),
        from_base_seq: SEQ, to_base_seq: SEQ, lines, by: "p1".into(), boms: boms(), now_ms: NOW,
    }
}

fn hub() -> dowiz_hub::Hub {
    dowiz_hub::Hub::create_sized(64 * 1024).unwrap()
}

/// A shelf with both rounds' reservations on it, as placement left them.
fn shelf() -> StockLog {
    let mut s = StockLog::create_sized(64 * 1024).unwrap();
    s.append_all(&[
        StockEvent::Received { item: "rice".into(), qty: 2000 },
        StockEvent::Received { item: "keg".into(), qty: 20 },
    ]).unwrap();
    let lines: Vec<(String, i64)> = vec![(dish("maki", "rice", 100).1, 2), (dish("beer", "keg", 1).1, 1)];
    for id in ["r1", "r2"] {
        s.append_all(&dowiz_hub::stock::reservations_for(id, &lines)).unwrap();
    }
    s
}

/// What one order holds of one item on the shelf.
fn held(s: &StockLog, order: &str, item: &str) -> i64 {
    s.ledger().unwrap().stranded().into_iter()
        .filter(|(o, i, _)| o == order && i == item).map(|(_, _, q)| q).sum()
}

/// LAW 3, per round: total = Σ lines + fee + tip − discount.
fn law3(o: &Value) -> bool {
    let lines: i64 = o["items"].as_array().unwrap().iter()
        .map(|l| l["unit_price"].as_i64().unwrap() * l["quantity"].as_i64().unwrap()).sum();
    let int = |k: &str| o[k].as_i64().unwrap_or(0);
    int("total") == lines + int("delivery_fee") + int("tip") - int("discount")
}

/// Run a transfer that must be refused, and prove neither image moved.
fn refused(from: &Value, to: &Value, i: &TransferIn) -> Refused {
    let (mut h, mut s) = (hub(), shelf());
    let (hb, sb) = (h.to_bytes(), s.to_bytes());
    let Err(r) = decide(&mut h, &mut s, Some(&view(from)), Some(&view(to)), i) else { panic!("it landed") };
    assert_eq!(h.to_bytes(), hb, "the log was written on a refusal");
    assert_eq!(s.to_bytes(), sb, "the shelf was written on a refusal");
    r
}

/// THE POSITIVE TWIN: the maki moves, Σ lines and Σ totals are conserved, law
/// 3 holds on both, the shelf moves the rice with the dish, and each round
/// carries its half of one signed transfer record.
#[test]
fn a_legal_transfer_conserves_lines_money_and_shelf() {
    let (mut h, mut s) = (hub(), shelf());
    let (from, to) = (round("r1", "PENDING"), round("r2", "CONFIRMED"));
    let d = decide(&mut h, &mut s, Some(&view(&from)), Some(&view(&to)), &input(vec![0])).expect("lands");
    let (a, b) = (&d.moved.from, &d.moved.to);
    assert_eq!((a["subtotal"].as_i64(), b["subtotal"].as_i64()), (Some(300), Some(2700)));
    assert_eq!(a["total"].as_i64().unwrap() + b["total"].as_i64().unwrap(), 3000, "Σ totals conserved");
    assert!(law3(a) && law3(b), "law 3 on both rounds");
    assert_eq!(d.moved.amount, 1200);
    assert_eq!(held(&s, "r1", "rice"), 0, "the source no longer holds the maki's rice");
    assert_eq!(held(&s, "r2", "rice"), 400, "the destination holds both maki lines' rice");
    assert_eq!(held(&s, "r1", "keg") + held(&s, "r2", "keg"), 2, "the beer did not move");
    let (ta, tb) = (&a["amended"][0]["transfer"], &b["amended"][0]["transfer"]);
    assert_eq!((ta["dir"].as_str(), tb["dir"].as_str()), (Some("out"), Some("in")));
    assert_eq!(ta["id"], tb["id"]);
    assert_eq!(ta["amount"], tb["amount"]);
    assert_eq!(a["amended"][0]["by"], json!("p1"));
    let ev = h.events();
    assert_eq!(ev.len(), 2);
    assert!(ev.iter().all(|e| e.kind == dowiz_hub::EventKind::Amended));
    assert!(d.from_seq > SEQ && d.to_seq > SEQ, "both versions move");
    assert!(d.from_body.contains("\"_d\":true") && d.to_body.contains("\"_d\":true"), "deltas");
}

/// A COMPED LINE STAYS COMPED AND TAKES ITS COMP WITH IT: the source's
/// discount falls by the comp, the destination's rises by it, law 3 holds on
/// both, and each side's comp adjustments sum to the move.
#[test]
fn a_comped_line_moves_its_discount_with_it() {
    let mut from = round("r1", "CONFIRMED");
    from["items"][0]["comped"] = json!(true);
    from["discount"] = json!(1200);
    from["total"] = json!(300);
    let (mut h, mut s) = (hub(), shelf());
    let d = decide(&mut h, &mut s, Some(&view(&from)), Some(&view(&round("r2", "PENDING"))), &input(vec![0])).expect("lands");
    let (a, b) = (&d.moved.from, &d.moved.to);
    assert_eq!((a["discount"].as_i64(), a["total"].as_i64()), (Some(0), Some(300)));
    assert_eq!((b["discount"].as_i64(), b["total"].as_i64()), (Some(1200), Some(1500)));
    assert_eq!(b["items"][2]["comped"], json!(true), "still comped where it landed");
    assert!(law3(a) && law3(b));
    assert_eq!(d.moved.comp, 1200);
    assert_eq!(a["adjustments"][0]["amount"], json!(-1200));
    assert_eq!(b["adjustments"][0]["amount"], json!(1200));
}

/// §2.9: nothing moves between rounds after the kitchen — on either side, and
/// the shelf is not touched. The positive twin is the legal transfer above.
#[test]
fn after_the_kitchen_nothing_moves_on_either_side() {
    for (f, t) in [("PREPARING", "PENDING"), ("PENDING", "READY"), ("READY", "PREPARING")] {
        let r = refused(&round("r1", f), &round("r2", t), &input(vec![0]));
        assert!(matches!(&r, Refused::Conflict(m) if m.contains("kitchen")), "{f}->{t}: {r:?}");
    }
    let r = refused(&round("r1", "PENDING"), &round("r2", "DELIVERED"), &input(vec![0]));
    assert!(matches!(r, Refused::Conflict(_)));
}

/// THE CONFIRMED FRAME: a paid round is neither source nor destination.
#[test]
fn a_paid_round_is_neither_source_nor_destination() {
    let mut paid = round("r1", "CONFIRMED");
    paid["payment_status"] = json!("paid");
    let r = refused(&paid, &round("r2", "PENDING"), &input(vec![0]));
    assert!(matches!(&r, Refused::Conflict(m) if m.contains("refund")), "{r:?}");
    let mut paid = round("r2", "CONFIRMED");
    paid["payment_status"] = json!("paid");
    let r = refused(&round("r1", "PENDING"), &paid, &input(vec![0]));
    assert!(matches!(&r, Refused::Conflict(m) if m.contains("refund")), "{r:?}");
}

/// A round partly paid cannot give away more than would leave it below
/// what was paid: that difference is money owed back, a refund.
#[test]
fn a_partly_paid_source_keeps_at_least_what_was_paid() {
    let mut part = round("r1", "CONFIRMED");
    part["payments"] = json!([{"amount": 1000, "method": "card"}]);
    assert!(matches!(refused(&part, &round("r2", "PENDING"), &input(vec![0])), Refused::Conflict(_)));
    // Twin: giving the beer away leaves 1200 ≥ 1000 paid.
    let (mut h, mut s) = (hub(), shelf());
    let d = decide(&mut h, &mut s, Some(&view(&part)), Some(&view(&round("r2", "PENDING"))), &input(vec![1]));
    assert_eq!(d.expect("lands").moved.from["total"], json!(1200));
}

/// A stale version on EITHER side is a Conflict: somebody changed that round.
#[test]
fn a_stale_version_on_either_round_is_a_conflict() {
    let stale = Refused::Conflict("this order changed while you were editing it".into());
    let mut i = input(vec![0]);
    i.from_base_seq = SEQ - 1;
    assert_eq!(refused(&round("r1", "PENDING"), &round("r2", "PENDING"), &i), stale);
    let mut i = input(vec![0]);
    i.to_base_seq = SEQ - 1;
    assert_eq!(refused(&round("r1", "PENDING"), &round("r2", "PENDING"), &i), stale);
}

/// No signer, another venue's round, a round moving into itself: refused.
#[test]
fn the_signer_the_venue_and_the_pair_are_checked() {
    let (a, b) = (round("r1", "PENDING"), round("r2", "PENDING"));
    let mut i = input(vec![0]);
    i.by = " ".into();
    assert!(matches!(refused(&a, &b, &i), Refused::Invalid(_)));
    let mut other = round("r2", "PENDING");
    other["location_id"] = json!("v2");
    assert_eq!(refused(&a, &other, &input(vec![0])), Refused::NotFound);
    let mut i = input(vec![0]);
    i.to_order_id = "r1".into();
    assert!(matches!(refused(&a, &a, &i), Refused::Invalid(_)));
    let (mut h, mut s) = (hub(), shelf());
    assert_eq!(decide(&mut h, &mut s, Some(&view(&a)), None, &input(vec![0])).err(), Some(Refused::NotFound));
}

/// The lines named are lines of the source, each once, and not all of them:
/// a round left with nothing is a cancellation, not a transfer.
#[test]
fn the_lines_named_are_checked() {
    let (a, b) = (round("r1", "PENDING"), round("r2", "PENDING"));
    for lines in [vec![], vec![5], vec![0, 0]] {
        assert!(matches!(refused(&a, &b, &input(lines.clone())), Refused::Invalid(_)), "{lines:?}");
    }
    assert!(matches!(refused(&a, &b, &input(vec![0, 1])), Refused::Conflict(m) if m.contains("cancellation")));
}

/// LAW 8 ON A LOG WITH TRANSFERS (G4): each round's history — its `Placed`
/// snapshot, then the transfer's `Amended` delta — folds to exactly the round
/// the command decided, comp and all. A delta that dropped a field (the
/// comp's adjustment, the moved discount) would serve one bill and rebuild
/// another.
#[test]
fn each_rounds_log_folds_to_the_round_after_a_transfer() {
    let mut from = round("r1", "CONFIRMED");
    from["items"][0]["comped"] = json!(true);
    from["discount"] = json!(1200);
    from["total"] = json!(300);
    let to = round("r2", "PENDING");
    let (mut h, mut s) = (hub(), shelf());
    for o in [&from, &to] {
        let id = o["id"].as_str().unwrap();
        h.append(dowiz_hub::EventKind::Placed, id, &o.to_string(), SEQ, [0u8; 32]).unwrap();
    }
    let d = decide(&mut h, &mut s, Some(&view(&from)), Some(&view(&to)), &input(vec![0])).expect("lands");
    for (id, want) in [("r1", &d.moved.from), ("r2", &d.moved.to)] {
        let hist = h.history(id);
        assert_eq!(hist.len(), 2, "{id}: Placed + one Amended");
        let folded = crate::fold::fold(hist.iter().map(|e| e.order_json.as_str()));
        assert_eq!(&folded, want, "{id}: the log rebuilds the round being served");
    }
    let sub = |o: &Value| o["subtotal"].as_i64().unwrap();
    assert_eq!(sub(&d.moved.from) + sub(&d.moved.to), 3000, "Σ line totals conserved");
}

/// A PAYMENT ON THE DESTINATION DOES NOT STOP IT GROWING: what was paid is
/// below the new total. (The source's side of the same rule is above.)
#[test]
fn a_partly_paid_destination_may_take_lines() {
    let mut part = round("r2", "CONFIRMED");
    part["payments"] = json!([{"amount": 1500, "method": "cash"}]);
    let (mut h, mut s) = (hub(), shelf());
    let d = decide(&mut h, &mut s, Some(&view(&round("r1", "PENDING"))), Some(&view(&part)), &input(vec![1]));
    let d = d.expect("lands");
    assert_eq!(d.moved.to["total"], json!(1800));
    assert!(law3(&d.moved.from) && law3(&d.moved.to));
}
