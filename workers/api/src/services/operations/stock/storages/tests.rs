//! P12/P13 through the stock door's turn: a delivery into a storage, a
//! transfer, a count in one storage, the storage cards, a freezing record.
use super::*;
use crate::services::operations::stock::turn::{run as turn, supplies_of, StockTurnIn};
use serde_json::json;

const NOW: i64 = 1_790_000_000_000;

fn input(kind: &str, body: Value) -> StockTurnIn {
    let supplies = supplies_of(vec![
        ("salmon".into(), json!({ "name": "Salmon", "unit": "g" }).to_string()),
        ("rice".into(), json!({ "name": "Rice", "unit": "g" }).to_string()),
    ]);
    StockTurnIn { kind: kind.into(), body, by: "p_anna".into(), now_ms: NOW, today: 20260926, supplies, currency: String::new() }
}

fn by_store(log: &StockLog, id: &str) -> Value {
    levels(&log.journal().unwrap(), id)["stores"].clone()
}

#[test]
fn a_delivery_a_transfer_and_a_count_land_in_their_storages() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    turn(&mut log, &input("received", json!({ "item": "salmon", "qty": 2000, "store": "freezer", "treated": "CERT-9" })), false).unwrap();
    let (shown, told) = turn(&mut log, &input("moved", json!({ "item": "salmon", "qty": 500, "from": "freezer", "to": "kitchen" })), false).unwrap();
    assert_eq!((shown["from"].as_str(), shown["to"].as_str(), told.len()), (Some("freezer"), Some("kitchen"), 0));
    assert_eq!(by_store(&log, "salmon"), json!({ "freezer": 1500, "kitchen": 500 }));
    // Counted 450 in the kitchen: the shelf is told 1500 + 450.
    turn(&mut log, &input("count", json!({ "lines": [{ "item": "salmon", "observed": 450 }], "store": "kitchen" })), false).unwrap();
    assert_eq!(by_store(&log, "salmon"), json!({ "freezer": 1500, "kitchen": 450 }));
    assert_eq!(log.ledger().unwrap().level("salmon").on_hand, 1950);
    assert_eq!(levels(&log.journal().unwrap(), "salmon")["home"], "kitchen", "the next sale draws from the kitchen");
    // Refused, nothing written: more than the storage holds, an unknown
    // storage, a storage id that is not one, an unknown supply.
    let n = log.len();
    assert_eq!(turn(&mut log, &input("moved", json!({ "item": "salmon", "qty": 451, "from": "kitchen", "to": "bar" })), false).unwrap_err().0, 409);
    assert_eq!(turn(&mut log, &input("received", json!({ "item": "rice", "qty": 5, "store": "cellar" })), false).unwrap_err().0, 409);
    assert_eq!(turn(&mut log, &input("received", json!({ "item": "rice", "qty": 5, "store": "Big Room" })), false).unwrap_err().0, 400);
    assert_eq!(turn(&mut log, &input("moved", json!({ "item": "tuna", "qty": 1, "from": "kitchen", "to": "bar" })), false).unwrap_err().0, 404);
    assert_eq!(log.len(), n);
}

#[test]
fn storage_cards_mint_rename_and_never_archive_with_stock() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    let (shown, _) = turn(&mut log, &input("storage", json!({ "card": { "name": "Wine cellar" } })), false).unwrap();
    assert_eq!(shown["id"], "wine-cellar");
    let (shown, _) = turn(&mut log, &input("storage", json!({ "card": { "name": "Холодильна камера" } })), false).unwrap();
    assert_eq!(shown["id"], "s5", "a name with no latin letters gets a minted id");
    turn(&mut log, &input("received", json!({ "item": "rice", "qty": 10, "store": "wine-cellar" })), false).unwrap();
    let archive = input("storage", json!({ "card": { "id": "wine-cellar", "name": "Wine cellar", "archived": true } }));
    let (status, said) = turn(&mut log, &archive, false).unwrap_err();
    assert_eq!(status, 409);
    assert!(said.contains("still holds 10 of rice"), "{said}");
    turn(&mut log, &input("moved", json!({ "item": "rice", "qty": 10, "from": "wine-cellar", "to": "kitchen" })), false).unwrap();
    turn(&mut log, &archive, false).unwrap();
    let all = list(&log);
    assert_eq!(all.as_array().unwrap().len(), 5);
    assert_eq!(all[3], json!({ "id": "wine-cellar", "name": "Wine cellar", "archived": true, "builtIn": false, "default": false }));
    assert!(turn(&mut log, &input("storage", json!({ "card": { "name": "x", "bogus": 1 } })), false).is_err(), "a strict card");
}

#[test]
fn a_freezing_record_says_which_rule_it_meets_and_never_blocks() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    let (short, _) = turn(&mut log, &input("frozen", json!({ "item": "salmon", "lot": "L1", "hours": 23, "tempC": -20 })), false).unwrap();
    assert_eq!(short["rule"], Value::Null, "-20 C for 23 h meets no rule, and is still recorded");
    let (ok, _) = turn(&mut log, &input("frozen", json!({ "item": "salmon", "lot": "L1", "hours": 24, "tempC": -20 })), false).unwrap();
    assert_eq!(ok["rule"], "-20C/24h");
    assert_eq!(log.trace().unwrap().freezing.len(), 2);
    assert_eq!(turn(&mut log, &input("frozen", json!({ "item": "salmon", "lot": "L1", "hours": 24 })), false).unwrap_err().0, 400);
}

#[test]
fn mint_id_is_latin_lowercase_and_unique() {
    let taken = vec![Storage { id: "bar".into(), name: String::new(), archived: false }];
    assert_eq!(mint_id("  Bar  ", &taken), "bar2");
    assert_eq!(mint_id("Cold room #2", &taken), "cold-room-2");
    assert_eq!(mint_id("", &taken), "s2");
}
