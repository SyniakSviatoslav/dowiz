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

/// A transfer's quantity is read as every other stock quantity is
/// (`recipe::qty`: text or number, three decimals). UPDATED 2026-10-17: this
/// test pinned the refusal of a fraction by name ("move whole units of g
/// between storages for now"); a fraction now MOVES exactly
/// (`a_fraction_moves_exactly_between_storages`). What is still refused:
/// more than three decimals, and no quantity at all -- never "how much?".
#[test]
fn a_transfer_reads_its_quantity_as_typed() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    turn(&mut log, &input("received", json!({ "item": "salmon", "qty": 2000, "store": "freezer" })), false).unwrap();
    let n = log.len();
    let (status, said) = turn(&mut log, &input("moved", json!({ "item": "salmon", "qty": "1.0001", "from": "freezer", "to": "kitchen" })), false).unwrap_err();
    assert_eq!(status, 400);
    assert!(said.starts_with("Salmon: ") && said.contains("at most 3 decimals"), "{said}");
    let (status, said) = turn(&mut log, &input("moved", json!({ "item": "salmon", "from": "freezer", "to": "kitchen" })), false).unwrap_err();
    assert_eq!(status, 400);
    assert!(said.starts_with("Salmon: ") && said != "how much?", "{said}");
    assert_eq!(log.len(), n, "nothing written");
    // TWINS: a whole quantity typed as text, or with zero decimals, moves.
    turn(&mut log, &input("moved", json!({ "item": "salmon", "qty": "500", "from": "freezer", "to": "kitchen" })), false).unwrap();
    let (shown, _) = turn(&mut log, &input("moved", json!({ "item": "salmon", "qty": "250,000", "from": "freezer", "to": "kitchen" })), false).unwrap();
    assert_eq!(shown["qty"], json!(250));
    assert_eq!(by_store(&log, "salmon"), json!({ "freezer": 1250, "kitchen": 750 }));
}

// ---- FRACTIONAL TRANSFERS (operator 2026-10-17: "зроби дробове переміщення між складами") ----

fn input_with_pieces(kind: &str, body: Value) -> StockTurnIn {
    let mut i = input(kind, body);
    i.supplies.extend(supplies_of(vec![("nori".into(), json!({ "name": "Nori", "unit": "unit" }).to_string())]));
    i
}

/// `item`'s exact level per storage, as the Stock screen reads it: the
/// `exact` map when a fraction is anywhere, else the whole `stores`.
fn exact(log: &StockLog, id: &str) -> Value {
    let l = levels(&log.journal().unwrap(), id);
    match l.get("exact") {
        Some(e) => e.clone(),
        None => Value::Object(l["stores"].as_object().unwrap().iter().map(|(k, v)| (k.clone(), json!(v.to_string()))).collect()),
    }
}

/// THE INVARIANT: the storages' exact levels sum to the item's exact shelf
/// (`on_hand x 10^6 - carry`), in millionths, from the checkpoint fold AND
/// from genesis. Answers that sum.
fn sums(log: &StockLog, id: &str) -> i64 {
    let (led, carry) = log.shelf_exact().unwrap();
    let want = carry.exact(id, led.level(id).on_hand);
    let sum = |v: Value| -> i64 {
        v.as_object().unwrap().values().map(|q| crate::recipe::qty::parse(q.as_str().unwrap()).unwrap().uq()).sum()
    };
    assert_eq!(sum(exact(log, id)), want, "{id}: storages {} vs the shelf {want}", exact(log, id));
    let now = log.journal_now().unwrap();
    assert_eq!(sum(levels(&now, id).get("exact").cloned().unwrap_or_else(|| exact(log, id))), want, "{id}: through the checkpoint");
    want
}

#[test]
fn a_fraction_moves_exactly_between_storages() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    let t = |log: &mut StockLog, kind: &str, b: Value| turn(log, &input_with_pieces(kind, b), false);
    t(&mut log, "received", json!({ "item": "salmon", "qty": 1000, "store": "freezer" })).unwrap();
    t(&mut log, "received", json!({ "item": "nori", "qty": 3, "store": "bar" })).unwrap();
    let n = log.len();
    let (shown, _) = t(&mut log, "moved", json!({ "item": "salmon", "qty": "0,5", "from": "freezer", "to": "kitchen" })).unwrap();
    assert_eq!(log.len(), n + 1, "one note");
    assert_eq!(shown["qty"], json!("0.5"));
    assert_eq!(shown["exact"], json!({ "freezer": "999.5", "kitchen": "0.5" }));
    assert!(shown["stores"].is_object(), "the old whole field is still answered: {shown}");
    // Half a piece, typed as a number.
    let (shown, _) = t(&mut log, "moved", json!({ "item": "nori", "qty": 0.5, "from": "bar", "to": "kitchen" })).unwrap();
    assert_eq!(shown["exact"], json!({ "bar": "2.5", "kitchen": "0.5" }));
    // A thousandth, and back again.
    t(&mut log, "moved", json!({ "item": "salmon", "qty": "0.001", "from": "freezer", "to": "bar" })).unwrap();
    assert_eq!(exact(&log, "salmon"), json!({ "bar": "0.001", "freezer": "999.499", "kitchen": "0.5" }));
    // The item's total did not move by one millionth.
    assert_eq!(sums(&log, "salmon"), 1000 * 1_000_000);
    assert_eq!(sums(&log, "nori"), 3 * 1_000_000);
    assert_eq!(log.ledger().unwrap().level("salmon").on_hand, 1000);
}

#[test]
fn a_move_past_what_the_storage_holds_is_refused_by_name() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    turn(&mut log, &input("received", json!({ "item": "salmon", "qty": 2, "store": "freezer" })), false).unwrap();
    turn(&mut log, &input("moved", json!({ "item": "salmon", "qty": "0.5", "from": "freezer", "to": "kitchen" })), false).unwrap();
    let n = log.len();
    let (status, said) = turn(&mut log, &input("moved", json!({ "item": "salmon", "qty": "0.501", "from": "kitchen", "to": "bar" })), false).unwrap_err();
    assert_eq!((status, said.as_str()), (409, "Salmon: kitchen holds only 0.5"));
    assert_eq!(log.len(), n, "nothing written");
    // TWIN: exactly what it holds moves, and the kitchen is empty.
    turn(&mut log, &input("moved", json!({ "item": "salmon", "qty": "0.5", "from": "kitchen", "to": "bar" })), false).unwrap();
    assert_eq!(exact(&log, "salmon"), json!({ "bar": "0.5", "freezer": "1.5" }));
    assert_eq!(sums(&log, "salmon"), 2_000_000);
}

/// A RETRY OF THE SAME MOVE moves once: the console sends one `moveId` per
/// Move sheet, and the hub writes a transfer with an id it has seen never
/// again -- it answers the levels as they are, `replayed: true`.
#[test]
fn the_same_move_sent_twice_moves_once() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    turn(&mut log, &input("received", json!({ "item": "salmon", "qty": 2, "store": "freezer" })), false).unwrap();
    let mv = json!({ "item": "salmon", "qty": "0.25", "from": "freezer", "to": "kitchen", "moveId": "mv-7f3a" });
    let (first, _) = turn(&mut log, &input("moved", mv.clone()), false).unwrap();
    let n = log.len();
    let (again, _) = turn(&mut log, &input("moved", mv.clone()), false).unwrap();
    assert_eq!((first["replayed"].clone(), again["replayed"].clone()), (json!(false), json!(true)));
    assert_eq!(log.len(), n, "a retry wrote a second transfer");
    assert_eq!(again["exact"], first["exact"]);
    assert_eq!(exact(&log, "salmon"), json!({ "freezer": "1.75", "kitchen": "0.25" }));
    // The same id for ANOTHER transfer is refused, by its id; a bad id is a 400.
    let other = json!({ "item": "salmon", "qty": "0.5", "from": "freezer", "to": "kitchen", "moveId": "mv-7f3a" });
    let (status, said) = turn(&mut log, &input("moved", other), false).unwrap_err();
    assert!(status == 409 && said.contains("mv-7f3a"), "{status} {said}");
    let bad = json!({ "item": "salmon", "qty": "0.5", "from": "freezer", "to": "kitchen", "moveId": "a b" });
    assert_eq!(turn(&mut log, &input("moved", bad), false).unwrap_err().0, 400);
    assert_eq!(log.len(), n);
    // TWIN: a new id moves again.
    let next = json!({ "item": "salmon", "qty": "0.25", "from": "freezer", "to": "kitchen", "moveId": "mv-7f3b" });
    turn(&mut log, &input("moved", next), false).unwrap();
    assert_eq!(exact(&log, "salmon"), json!({ "freezer": "1.5", "kitchen": "0.5" }));
}

/// A COUNT IN ONE STORAGE keeps the OTHER storages' fractions: the shelf is
/// told the counted storage plus every other one EXACTLY (before 2026-10-17
/// the others were added back as whole levels and their fraction was lost).
#[test]
fn a_count_in_one_storage_keeps_the_others_fractions() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    turn(&mut log, &input("received", json!({ "item": "salmon", "qty": 10, "store": "freezer" })), false).unwrap();
    turn(&mut log, &input("moved", json!({ "item": "salmon", "qty": "2.5", "from": "freezer", "to": "kitchen" })), false).unwrap();
    turn(&mut log, &input("count", json!({ "lines": [{ "item": "salmon", "observed": "1.25" }], "store": "kitchen" })), false).unwrap();
    assert_eq!(exact(&log, "salmon"), json!({ "freezer": "7.5", "kitchen": "1.25" }));
    assert_eq!(sums(&log, "salmon"), 8_750_000);
    // Counted in the freezer (not where the next sale draws from).
    turn(&mut log, &input("count", json!({ "lines": [{ "item": "salmon", "observed": "7.125" }], "store": "freezer" })), false).unwrap();
    assert_eq!(exact(&log, "salmon"), json!({ "freezer": "7.125", "kitchen": "1.25" }));
    assert_eq!(sums(&log, "salmon"), 8_375_000);
    // TWIN: a whole count in a whole storage stays whole.
    turn(&mut log, &input("count", json!({ "lines": [{ "item": "salmon", "observed": 7 }], "store": "freezer" })), false).unwrap();
    assert_eq!(exact(&log, "salmon"), json!({ "freezer": "7", "kitchen": "1.25" }));
    assert_eq!(sums(&log, "salmon"), 8_250_000);
}

/// Deliveries and write-offs that NAME a storage keep it, fraction and all;
/// ones that do not land where the item was last received (the default rule).
#[test]
fn deliveries_and_write_offs_keep_their_storages_fraction() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    turn(&mut log, &input("received", json!({ "item": "salmon", "qty": "2.5", "store": "freezer" })), false).unwrap();
    turn(&mut log, &input("received", json!({ "item": "salmon", "qty": "0.75", "store": "bar" })), false).unwrap();
    assert_eq!(exact(&log, "salmon"), json!({ "bar": "0.75", "freezer": "2.5" }));
    turn(&mut log, &input("wasted", json!({ "item": "salmon", "qty": "0.5", "reason": "spoiled", "store": "freezer" })), false).unwrap();
    assert_eq!(exact(&log, "salmon"), json!({ "bar": "0.75", "freezer": "2" }));
    // No storage named: the bar received it last.
    turn(&mut log, &input("received", json!({ "item": "salmon", "qty": "1.125" })), false).unwrap();
    assert_eq!(exact(&log, "salmon"), json!({ "bar": "1.875", "freezer": "2" }));
    assert_eq!(sums(&log, "salmon"), 3_875_000);
}
