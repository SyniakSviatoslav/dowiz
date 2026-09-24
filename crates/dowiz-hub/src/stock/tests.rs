//! A11 (BLUEPRINT-OPERATIONAL-BLIND-SPOTS §2.1): a write-off names a person, and
//! the records written before that rule existed still fold -- to the same
//! ledger they always did.

use super::*;

/// HEAD's encoder as of 2026-09-22 (`git show HEAD:crates/dowiz-hub/src/stock.rs`),
/// copied VERBATIM except that it ignores `by`, which did not exist. It is the
/// wire format every live venue's stock log was written in.
fn old_encode(ev: &StockEvent) -> String {
    match ev {
        StockEvent::Received { item, qty } => {
            format!(r#"{{"k":"received","item":"{}","qty":{qty}}}"#, esc(item))
        }
        StockEvent::Reserved { item, qty, order_id } => format!(
            r#"{{"k":"reserved","item":"{}","qty":{qty},"order":"{}"}}"#,
            esc(item),
            esc(order_id)
        ),
        StockEvent::Consumed { item, qty, order_id } => format!(
            r#"{{"k":"consumed","item":"{}","qty":{qty},"order":"{}"}}"#,
            esc(item),
            esc(order_id)
        ),
        StockEvent::Released { item, qty, order_id } => format!(
            r#"{{"k":"released","item":"{}","qty":{qty},"order":"{}"}}"#,
            esc(item),
            esc(order_id)
        ),
        StockEvent::Wasted { item, qty, reason, by: _ } => format!(
            r#"{{"k":"wasted","item":"{}","qty":{qty},"reason":"{}"}}"#,
            esc(item),
            reason.as_str()
        ),
        StockEvent::Stocktake { item, observed, stocktake_id, by: _ } => format!(
            r#"{{"k":"stocktake","item":"{}","observed":{observed},"id":"{}"}}"#,
            esc(item),
            esc(stocktake_id)
        ),
        // Did not exist in HEAD's encoder: nothing old can have written one.
        StockEvent::Served { .. } | StockEvent::Returned { .. } | StockEvent::Unserved { .. } => String::new(),
    }
}

fn wasted(item: &str, qty: Qty, reason: WasteReason, by: &str) -> StockEvent {
    StockEvent::Wasted { item: item.into(), qty, reason, by: by.into() }
}
fn counted(item: &str, observed: Qty, by: &str) -> StockEvent {
    StockEvent::Stocktake { item: item.into(), observed, stocktake_id: "st_1".into(), by: by.into() }
}

/// A venue's history as it was written before signers: every kind of event,
/// two write-offs and a count among them, none with a `by`.
fn history() -> Vec<StockEvent> {
    vec![
        StockEvent::Received { item: "rice".into(), qty: 100 },
        StockEvent::Reserved { item: "rice".into(), qty: 10, order_id: "o1".into() },
        wasted("rice", 5, WasteReason::Spoiled, ""),
        counted("nori", 42, ""),
        StockEvent::Consumed { item: "rice".into(), qty: 10, order_id: "o1".into() },
        StockEvent::Reserved { item: "nori".into(), qty: 4, order_id: "o2".into() },
        StockEvent::Released { item: "nori".into(), qty: 4, order_id: "o2".into() },
        StockEvent::Received { item: "nori".into(), qty: 3 },
        wasted("nori", 2, WasteReason::Dropped, ""),
    ]
}

/// The image a live venue holds: those events, in the OLD bytes, through the
/// same chained record writer the log has always used.
fn old_image() -> Vec<u8> {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    for ev in history() {
        log.write_payload(old_encode(&ev).into_bytes()).unwrap();
    }
    log.to_bytes_trimmed()
}

/// §2.1 CHECK: "a decode of a pre-change record folds to the same ledger".
/// THE LIVE-VENUE TEST. rice 100 - 5 wasted - 10 consumed = 85; nori counted
/// at 42, +3, -2 = 43; nothing left reserved.
#[test]
fn an_image_written_before_signers_folds_to_the_same_ledger() {
    let bytes = old_image();
    assert!(!String::from_utf8_lossy(&bytes).contains("\"by\""), "the fixture really is the old encoding");
    let log = StockLog::load(&bytes).expect("an old image still loads");
    assert_eq!(log.events(), history(), "every old record decodes, the unsigned ones as by = \"\"");
    let led = log.ledger().expect("an old image still folds");
    assert_eq!(led.level("rice"), StockLevel { on_hand: 85, reserved: 0 });
    assert_eq!(led.level("nori"), StockLevel { on_hand: 43, reserved: 0 });
    assert!(led.stranded().is_empty());
    // The same numbers the pre-change fold gave: the new encoding of the same
    // history folds identically, so `by` moved nothing but the signer.
    let led_new = StockLedger::fold(&history()).unwrap();
    assert_eq!(led.items(), led_new.items());
}

/// And the venue keeps TRADING on top of that history: an order reserves, and
/// a signed write-off lands, against a fold that contains unsigned records.
/// (Before this fix both were refused, because `append` folds first.)
#[test]
fn a_venue_with_unsigned_history_can_still_take_orders_and_write_off() {
    let mut log = StockLog::load(&old_image()).unwrap();
    log.append_all(&[StockEvent::Reserved { item: "rice".into(), qty: 20, order_id: "o3".into() }])
        .expect("an order reserves against an old history");
    log.append(&wasted("rice", 1, WasteReason::Returned, "p_anna")).expect("a signed write-off lands");
    let log = StockLog::load(&log.to_bytes()).unwrap();
    assert_eq!(log.ledger().unwrap().level("rice"), StockLevel { on_hand: 84, reserved: 20 });
    let evs = log.events();
    assert_eq!(evs.len(), 11);
    assert_eq!(signer(evs.last().unwrap()), Some("p_anna"));
    assert_eq!(signer(&evs[2]), None, "the pre-signer write-off is reported as unsigned, not invented");
}

/// §2.1 CHECK: "a `Wasted` with empty `by` is refused" -- for a NEW write, and
/// nothing reaches the log. Positive twin: the same event signed lands.
#[test]
fn a_new_write_off_without_a_signer_is_refused_and_writes_nothing() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.append(&StockEvent::Received { item: "rice".into(), qty: 10 }).unwrap();
    for blank in ["", "   "] {
        assert!(matches!(log.append(&wasted("rice", 2, WasteReason::Dropped, blank)), Err(StockError::Unsigned)));
        assert!(matches!(log.append_all(&[wasted("rice", 2, WasteReason::Dropped, blank)]), Err(StockError::Unsigned)));
    }
    assert_eq!(log.len(), 1, "the refusals wrote nothing");
    log.append(&wasted("rice", 2, WasteReason::Dropped, "p_anna")).expect("signed, it lands");
    assert_eq!(log.len(), 2);
    assert_eq!(log.ledger().unwrap().level("rice").on_hand, 8);
}

/// The same rule for a count. Positive twin included.
#[test]
fn a_new_count_without_a_signer_is_refused() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    assert!(matches!(log.append(&counted("nori", 5, "")), Err(StockError::Unsigned)));
    assert_eq!(log.len(), 0);
    log.append(&counted("nori", 5, "p_owner")).expect("signed, it lands");
    assert_eq!(log.ledger().unwrap().level("nori").on_hand, 5);
}

/// The order lifecycle signs nothing and needs no signer: the rule is about the
/// two events a PERSON causes, not every event.
#[test]
fn lifecycle_events_need_no_signer() {
    assert!(signed(&StockEvent::Received { item: "x".into(), qty: 1 }).is_ok());
    assert!(signed(&StockEvent::Reserved { item: "x".into(), qty: 1, order_id: "o".into() }).is_ok());
    assert!(signed(&wasted("x", 1, WasteReason::Spoiled, "")).is_err());
}

/// The closed set: two new words, round-tripping, and nothing else admitted.
#[test]
fn the_reason_set_is_closed_and_has_returned_and_staff_meal() {
    for r in WasteReason::ALL {
        assert_eq!(WasteReason::from_str(r.as_str()), Some(r));
        let ev = wasted("tuna", 1, r, "p1");
        assert_eq!(decode(&encode(&ev)).as_ref(), Some(&ev));
    }
    assert_eq!(WasteReason::allowed_words(), "spoiled, dropped, unsold, returned, staff_meal");
    assert_eq!(WasteReason::from_str("soggy"), None);
    assert_eq!(WasteReason::from_str("Spoiled"), None, "wire words are exact");
}

/// A hostile item name in an OLD record cannot forge a signer into it.
#[test]
fn an_item_name_cannot_forge_a_signer() {
    let ev = wasted(r#"x","by":"boss"#, 1, WasteReason::Spoiled, "");
    match decode(&old_encode(&ev)) {
        Some(StockEvent::Wasted { item, by, .. }) => {
            assert_eq!(item, r#"x","by":"boss"#);
            assert_eq!(by, "");
        }
        other => panic!("{other:?}"),
    }
}
