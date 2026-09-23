//! A11 (§2.1): the movement a request becomes, and what it may not say.

use super::*;
use dowiz_hub::stock::{StockLog, StockError};

fn body(raw: &str) -> StockMoveIn {
    serde_json::from_str(raw).expect("a well-formed body")
}

/// THE SIGNER IS NOT THE CALLER'S TO NAME. A body carrying `by` does not parse
/// -- the route answers 400 before anything is signed. Positive twin: the same
/// body without it parses, and the event carries the AUTHENTICATED id.
#[test]
fn a_body_cannot_name_its_signer() {
    let forged = r#"{"item":"rice","qty":2,"reason":"dropped","by":"p_boss"}"#;
    assert!(serde_json::from_str::<StockMoveIn>(forged).is_err(), "`by` in the body is refused");
    let ev = movement("wasted", body(r#"{"item":"rice","qty":2,"reason":"dropped"}"#), "p_anna", 7).unwrap();
    assert_eq!(dowiz_hub::stock::signer(&ev), Some("p_anna"));
}

/// §2.1 CHECK: `"reason":"soggy"` -> 400 naming the words, and the log's
/// `len()` did not move. A missing reason is the same 400, never `Spoiled`.
#[test]
fn an_unknown_or_missing_reason_is_a_400_and_writes_nothing() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.append(&StockEvent::Received { item: "rice".into(), qty: 10 }).unwrap();
    for raw in [
        r#"{"item":"rice","qty":2,"reason":"soggy"}"#,
        r#"{"item":"rice","qty":2}"#,
        r#"{"item":"rice","qty":2,"reason":"  "}"#,
    ] {
        match movement("wasted", body(raw), "p_anna", 7) {
            Err((400, said)) => {
                assert!(said.contains("spoiled, dropped, unsold, returned, staff_meal"), "{said}")
            }
            other => panic!("{raw}: {other:?}"),
        }
    }
    assert_eq!(log.len(), 1, "no refusal reached the log");
    // Positive twin: each word of the set becomes that reason.
    for r in WasteReason::ALL {
        let raw = format!(r#"{{"item":"rice","qty":1,"reason":"{}"}}"#, r.as_str());
        match movement("wasted", body(&raw), "p_anna", 7).unwrap() {
            StockEvent::Wasted { reason, .. } => assert_eq!(reason, r),
            other => panic!("{other:?}"),
        }
    }
    log.append(&movement("wasted", body(r#"{"item":"rice","qty":2,"reason":"returned"}"#), "p_anna", 7).unwrap())
        .unwrap();
    assert_eq!(log.len(), 2);
}

/// A count is signed by who counted, and stamped with the moment.
#[test]
fn a_count_is_signed_by_the_caller() {
    match movement("stocktake", body(r#"{"item":"nori","observed":5}"#), "p_owner", 42).unwrap() {
        StockEvent::Stocktake { by, stocktake_id, observed, .. } => {
            assert_eq!((by.as_str(), stocktake_id.as_str(), observed), ("p_owner", "st_42", 5));
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(movement("stocktake", body(r#"{"item":"nori"}"#), "p", 1).unwrap_err().0, 400);
}

/// An empty signer can never reach the log, even if a route resolved one: the
/// ledger's write door refuses it (defence in depth under this function).
#[test]
fn an_empty_signer_is_refused_at_the_write_door() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.append(&StockEvent::Received { item: "rice".into(), qty: 10 }).unwrap();
    let ev = movement("wasted", body(r#"{"item":"rice","qty":1,"reason":"spoiled"}"#), "", 1).unwrap();
    assert!(matches!(log.append(&ev), Err(StockError::Unsigned)));
    let ev = movement("wasted", body(r#"{"item":"rice","qty":1,"reason":"spoiled"}"#), "p", 1).unwrap();
    assert!(log.append(&ev).is_ok());
}

/// The order lifecycle's three are not a human's to write, and the rest is 400.
#[test]
fn only_three_movements_exist() {
    assert!(movement("received", body(r#"{"item":"rice","qty":3}"#), "p", 1).is_ok());
    for k in ["reserved", "consumed", "released", "eaten"] {
        assert_eq!(movement(k, body(r#"{"item":"rice","qty":3}"#), "p", 1).unwrap_err().0, 400, "{k}");
    }
    assert_eq!(movement("received", body(r#"{"item":"  ","qty":3}"#), "p", 1).unwrap_err().0, 400);
}
