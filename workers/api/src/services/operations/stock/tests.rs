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

/// W-STOCK P5: the Stock screen's answer carries the supplier cards and the
/// order list from the same fold; a supply's `supplier` text names its card.
#[test]
fn the_screen_carries_the_cards_and_the_order_list() {
    use serde_json::json;
    let mut cat = dowiz_hub::catalog::Catalog::create().unwrap();
    cat.set_supply("rice", &json!({ "id": "rice", "name": "Rice", "unit": "g", "kind": "food_ingredient", "supplier": "Sea", "packs": [{ "name": "5 kg", "qty": 5000 }] }).to_string());
    cat.set_supply("nori", &json!({ "id": "nori", "name": "Nori", "unit": "unit" }).to_string());
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    let now = 1_790_000_000_000;
    log.set_clock(now - 1000);
    log.append(&StockEvent::Received { item: "rice".into(), qty: 10_000 }).unwrap();
    log.append(&StockEvent::Wasted { item: "rice".into(), qty: 2800, reason: WasteReason::Spoiled, by: "p".into() }).unwrap();
    log.append_note(suppliers::CARD, &json!({ "card": { "id": "sea", "name": "Sea", "days": [1, 4], "leadDays": 1 } }).to_string()).unwrap();
    let v = shelf(&cat, &log, now).unwrap();
    assert_eq!(v["supplierCards"][0]["id"], "sea");
    let g = &v["orderList"]["groups"][0];
    assert_eq!(g["supplier"]["id"], "sea");
    // 2800 used in its one day; par 2800 x (1 lead + 4 days between Monday and Thursday) = 14 000; 7200 free; two 5 kg sacks.
    assert_eq!((g["lines"][0]["id"].clone(), g["lines"][0]["suggest"].clone()), (json!("rice"), json!(10_000)));
    assert_eq!(v["orderList"]["groups"].as_array().unwrap().len(), 1, "nori: nothing used, nothing on order -- not listed");
    assert_eq!(v["supplies"].as_array().unwrap().len(), 2, "a note is not a supply");
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

/// A ПФ ON THE STOCK SCREEN IS PRICED FROM ITS LEAVES' AVERAGE (2026-10-17):
/// the console's tech card reads a prep's `costPerBasis` from this list, and
/// on a venue whose raw supplies carry only a weighted average (no list
/// price) every prep read null. Twin: a prep with one leaf priced by
/// NEITHER stays null -- never a partial sum.
#[test]
fn a_prep_whose_leaves_have_only_an_average_is_priced_on_the_shelf() {
    use dowiz_hub::stock::cost::Price;
    use serde_json::json;
    let mut cat = dowiz_hub::catalog::Catalog::create().unwrap();
    cat.set_supply("rice", &json!({ "id": "rice", "name": "Rice", "unit": "g", "kind": "food_ingredient" }).to_string());
    cat.set_supply("salt", &json!({ "id": "salt", "name": "Salt", "unit": "g", "kind": "condiment" }).to_string());
    cat.set_supply("nori", &json!({ "id": "nori", "name": "Nori", "unit": "g", "kind": "food_ingredient" }).to_string());
    cat.set_supply("rice-ready", &json!({ "id": "rice-ready", "name": "Rice ready", "unit": "g", "kind": "prep", "card": { "lines": [{ "item": "rice", "qty": 1000 }, { "item": "salt", "qty": 10 }], "yield": 1000 } }).to_string());
    cat.set_supply("roll", &json!({ "id": "roll", "name": "Roll", "unit": "g", "kind": "prep", "card": { "lines": [{ "item": "rice", "qty": 100 }, { "item": "nori", "qty": 3 }], "yield": 100 } }).to_string());
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    // Rice at 230 lek per kg, salt at 70 per kg: no list price anywhere.
    log.receive_priced("rice", 1000, &Price { unit_cost: 230, per: 1000, supplier: None, doc: None }).unwrap();
    log.receive_priced("salt", 1000, &Price { unit_cost: 70, per: 1000, supplier: None, doc: None }).unwrap();
    let v = shelf(&cat, &log, 1_790_000_000_000).unwrap();
    let row = |id: &str| v["supplies"].as_array().unwrap().iter().find(|r| r["id"] == id).cloned().unwrap();
    // (1000 x 230 000 + 10 x 70 000) / 1000 = 230 700 millionths per g -> 23.07 per 100 g -> 23.
    assert_eq!(row("rice-ready")["costPerBasis"], json!(23), "priced from the leaves' average");
    assert_eq!(row("roll")["costPerBasis"], serde_json::Value::Null, "nori has no price at all: unknown, not partial");
    assert_eq!(row("rice")["costPerBasis"], serde_json::Value::Null, "a raw row keeps its own (absent) list price");
}
