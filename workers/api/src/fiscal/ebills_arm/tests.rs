//! A VENUE SENDS ONLY WHEN ALL HOLD (card L70): each missing condition named,
//! and the armed twin. Arming needs the phrase; disarming never does.

use super::*;

fn link() -> LinkState {
    LinkState { usable: true, halted: false, failures: 0, last_ok_ms: 1 }
}

fn armed() -> Arming {
    Arming { armed: true, sale_unit: "DELIVERY".into(), fee_item: String::new(), cancel_armed: false }
}

#[test]
fn every_condition_holds_and_the_venue_is_armed() {
    assert!(not_ready(&Config::From(1), &armed(), &link()).is_empty());
}

/// AN UNARMED VENUE SENDS NOTHING: each missing condition, alone, stops it.
#[test]
fn each_missing_condition_alone_stops_the_venue_by_name() {
    let say = |c: &Config, a: &Arming, l: &LinkState| not_ready(c, a, l).join(" / ");
    assert!(say(&Config::Off, &armed(), &link()).contains("fiscal.since_ms"));
    assert!(say(&Config::From(1), &Arming { armed: false, ..armed() }, &link()).contains("not armed"));
    assert!(say(&Config::From(1), &Arming { sale_unit: String::new(), ..armed() }, &link()).contains("sale unit"));
    assert!(say(&Config::From(1), &armed(), &LinkState { usable: false, ..link() }).contains("not configured"));
    assert!(say(&Config::From(1), &armed(), &LinkState { halted: true, ..link() }).contains("halted"));
    assert!(say(&Config::From(1), &armed(), &LinkState { failures: 1, ..link() }).contains("did not work"));
    assert!(say(&Config::From(1), &armed(), &LinkState { last_ok_ms: 0, ..link() }).contains("did not work"));
}

#[test]
fn only_the_exact_word_yes_arms() {
    let from = |v: &'static str| arming(&move |k: &str| (k == ARMED).then(|| v.to_string()));
    assert!(from("yes").armed);
    for v in ["true", "1", "on", "YES", ""] {
        assert!(!from(v).armed, "{v:?} is not yes");
    }
}

/// ARMING NEEDS THE CONSEQUENCE TYPED; twin: with it, it arms.
#[test]
fn arming_without_the_phrase_is_refused_and_with_it_writes_yes() {
    let on = ArmIn { armed: Some(true), ..ArmIn::default() };
    assert!(arm(&on).unwrap_err().contains(CONFIRM));
    let wrong = ArmIn { confirm: Some("yes".into()), ..on.clone() };
    assert!(arm(&wrong).is_err());
    let right = ArmIn { confirm: Some(CONFIRM.into()), ..on };
    assert_eq!(arm(&right).unwrap(), vec![(ARMED, "yes".to_string())]);
    let cancel = ArmIn { cancel_armed: Some(true), ..ArmIn::default() };
    assert!(arm(&cancel).is_err(), "the cancel switch needs it too");
}

#[test]
fn disarming_needs_nothing_and_a_sale_unit_is_plain_text() {
    let off = ArmIn { armed: Some(false), sale_unit: Some(" DELIVERY ".into()), ..ArmIn::default() };
    assert_eq!(arm(&off).unwrap(), vec![(ARMED, String::new()), (SALE_UNIT, "DELIVERY".to_string())]);
    let bad = ArmIn { sale_unit: Some("7\"}".into()), ..ArmIn::default() };
    assert!(arm(&bad).is_err());
}

fn view(id: &str, o: Value) -> OrderView {
    OrderView { order_id: id.into(), kind: 1, seq: 1, order_json: o.to_string() }
}

/// A REFUND AFTER THE SEND OWES A CANCEL ENTRY. Twins: a refund before the
/// send, a sent order not refunded, and one already queued owe none.
#[test]
fn a_refund_after_the_send_owes_one_cancel_entry() {
    let sent = |status: &str| serde_json::json!({ "status": status, "fiscal": { "fic": "FIC1", "sale_id": 9001 }, "refund": { "reason": "refused_at_door" } });
    let orders = vec![
        view("r1", sent("REFUNDING")),
        view("r2", sent("DELIVERED")),
        view("r3", serde_json::json!({ "status": "COMPENSATED_REFUND" })),
        view("r4", sent("COMPENSATED_REFUND")),
    ];
    let owed = cancels_owed(&orders, &|id| id == "r4", 5);
    assert_eq!(owed.len(), 1);
    assert_eq!((owed[0].order_id.as_str(), owed[0].sale_id, owed[0].reason.as_str()), ("r1", 9001, "refused_at_door"));
    assert_eq!(owed[0].shape["withoutfiscalization"], Value::Bool(false));
}
