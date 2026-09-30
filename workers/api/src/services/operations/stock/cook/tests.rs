//! R2 through the object's turn: `POST /api/owner/stock/cooked`.

use super::*;
use dowiz_hub::stock::cost::Price;
use serde_json::json;

const NOW: i64 = 1_790_416_800_000;

fn catalogue() -> Vec<(String, String)> {
    let raw = |id: &str, unit: &str| (id.to_string(), json!({ "id": id, "name": id, "unit": unit, "kind": "food_ingredient" }).to_string());
    vec![
        raw("vinegar", "ml"),
        raw("salt", "g"),
        raw("sugar", "g"),
        ("mitsukan".into(), json!({ "id": "mitsukan", "name": "Mitsukan", "unit": "g", "kind": "prep", "shelfDays": 3,
            "card": { "lines": [{ "item": "vinegar", "qty": 800 }, { "item": "salt", "qty": 50 }, { "item": "sugar", "qty": 150 }], "yield": 1000 } }).to_string()),
    ]
}

fn input(kind: &str, body: Value) -> StockTurnIn {
    StockTurnIn { kind: kind.into(), body, by: "p_cook".into(), now_ms: NOW, today: 20260929, supplies: supplies_for(kind, catalogue()), currency: "ALL".into() }
}

fn shelf() -> StockLog {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    for (i, c) in [("vinegar", 300), ("salt", 50), ("sugar", 150)] {
        log.receive_priced(i, 5000, &Price { unit_cost: c, per: 1000, supplier: None, doc: None }).unwrap();
    }
    log
}

#[test]
fn a_batch_is_cooked_through_the_turn_and_shows_its_loss() {
    let mut log = shelf();
    let (shown, told) = super::super::turn::run(&mut log, &input("cooked", json!({ "item": "mitsukan", "qty": 1000, "out": 960 })), false).unwrap();
    // W-PF3 T2: one message for the groups -- what came out, the loss, the cost.
    let said = json!({ "name": "Mitsukan", "out": 960, "unit": "g", "lossG": 40, "lossPm": 40, "value": 265, "currency": "ALL" });
    assert_eq!(told, vec![("stock.cooked", said)]);
    assert_eq!((shown["planned"].clone(), shown["out"].clone(), shown["gross"].clone(), shown["lossG"].clone()), (json!(1000), json!(960), json!(1000), json!(40)));
    assert_eq!((shown["yieldPm"].clone(), shown["cardPm"].clone(), shown["value"].clone()), (json!(960), json!(1000), json!(265)));
    assert_eq!(shown["lines"].as_array().unwrap().len(), 3);
    assert_eq!(shown["expiry"], json!("2026-10-02"), "today + the ПФ's shelf days");
    let led = log.ledger().unwrap();
    assert_eq!((led.level("mitsukan").on_hand, led.level("vinegar").on_hand), (960, 4200));
    // Only the object's supplies carry records, and only for this kind.
    assert!(supplies_for("received", catalogue()).values().all(|s| s.record.is_none()));
}

#[test]
fn what_is_not_an_act_is_refused_and_writes_nothing() {
    let mut log = shelf();
    let n = log.len();
    let run = |log: &mut StockLog, b: Value| super::super::turn::run(log, &input("cooked", b), false).map(|_| ()).map_err(|e| e.0);
    assert_eq!(run(&mut log, json!({ "item": "salt", "qty": 10 })), Err(400), "a raw item has no card");
    assert_eq!(run(&mut log, json!({ "item": "ghost", "qty": 10 })), Err(404));
    assert_eq!(run(&mut log, json!({ "item": "mitsukan" })), Err(400), "how much?");
    assert_eq!(run(&mut log, json!({ "item": "mitsukan", "qty": 10, "by": "someone" })), Err(400), "no signer from the body");
    assert_eq!(run(&mut log, json!({ "item": "mitsukan", "qty": 0 })), Err(409));
    assert_eq!(log.len(), n);
    // The twin: a whole act, and `out` defaults to what the card makes.
    assert_eq!(run(&mut log, json!({ "item": "mitsukan", "qty": 100 })), Ok(()));
    assert_eq!(log.ledger().unwrap().level("mitsukan").on_hand, 100);
}
