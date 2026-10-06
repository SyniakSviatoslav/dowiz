//! W-STORE2: a station bound to a storage draws from it; an unbound venue
//! draws exactly as before; the binding survives checkpoints; a binding to a
//! storage the venue does not have (or archived) is refused, naming it.

use super::*;
use crate::stock::journal::Journal;
use crate::stock::meta::Meta;
use crate::stock::oldimage_tests::{fold_text, old_history};
use crate::stock::storages::{Storage, FREEZER};
use crate::stock::{draws_for, settle, Draw, StockLog};

const SUSHI: &str = r#"{"id":"roll","station":"sushi","bom":[{"supply":"salmon","qty":100},{"supply":"rice","qty":50}]}"#;
const PLATE: &str = r#"{"id":"plate","bom":[{"supply":"salmon","qty":50}]}"#;
const DRINK: &str = r#"{"id":"spritz","station":"bar","bom":[{"supply":"lemon","qty":1}]}"#;

fn into(store: &str) -> Meta {
    Meta { store: Some(store.into()), ..Meta::default() }
}
fn place(log: &mut StockLog, order: &str, lines: &[(&str, i64)]) {
    let lines: Vec<(String, i64)> = lines.iter().map(|(p, n)| (p.to_string(), *n)).collect();
    log.append_draws(&draws_for(order, &lines)).unwrap();
}
fn cook(log: &mut StockLog, order: &str, consume: bool) {
    let led = log.ledger().unwrap();
    log.append_all(&settle(&led, order, consume)).unwrap();
}
fn sell(log: &mut StockLog, order: &str, lines: &[(&str, i64)]) {
    place(log, order, lines);
    cook(log, order, true);
}
fn levels(log: &StockLog, item: &str) -> Vec<(String, Qty)> {
    let j = log.journal().unwrap();
    j.stores.of(item, &j.ledger)
}
fn s(v: &[(&str, Qty)]) -> Vec<(String, Qty)> {
    v.iter().map(|(a, b)| (a.to_string(), *b)).collect()
}
/// The storages sum to the shelf after EVERY record, and the fold through
/// the newest checkpoint is the fold from genesis.
fn assert_sound(log: &StockLog) {
    let mut j = Journal::default();
    for (n, rec) in log.raw().iter().enumerate() {
        j.step(rec).unwrap();
        for (item, l) in j.ledger.items() {
            let sum: Qty = j.stores.of(&item, &j.ledger).iter().map(|x| x.1).sum();
            assert_eq!(sum, l.on_hand, "record {n}: {item}'s storages sum to {sum}, the shelf says {}", l.on_hand);
        }
    }
    let now = log.journal_now().unwrap();
    assert_eq!(now.stores, j.stores, "the checkpointed storages differ from the fold from genesis");
    log.verify_checkpoints().unwrap();
}

/// THE ROW: a dish of a bound station draws from that storage, whatever
/// received the item last; an unbound station keeps Poster's rule; an order
/// with both splits its draw by what each dish used.
#[test]
fn a_bound_station_draws_from_its_storage() {
    let mut log = StockLog::create_sized(128 * 1024).unwrap();
    log.set_checkpoint_every(3);
    log.receive_with("salmon", 2000, &into(FREEZER)).unwrap();
    log.receive_with("salmon", 1000, &into("kitchen")).unwrap();
    log.receive_with("rice", 5000, &into("kitchen")).unwrap();
    log.bind_station("sushi", FREEZER, "owner").unwrap();
    sell(&mut log, "o1", &[(SUSHI, 2)]);
    assert_eq!(levels(&log, "salmon"), s(&[(FREEZER, 1800), ("kitchen", 1000)]), "the sushi station draws from the freezer");
    sell(&mut log, "o2", &[(PLATE, 1)]);
    assert_eq!(levels(&log, "salmon"), s(&[(FREEZER, 1800), ("kitchen", 950)]), "the unbound kitchen draws from where it was received");
    sell(&mut log, "o3", &[(SUSHI, 1), (PLATE, 2)]);
    assert_eq!(levels(&log, "salmon"), s(&[(FREEZER, 1700), ("kitchen", 850)]), "100 g sushi + 100 g plate, split");
    // The till's `served` (no reservation) follows the same binding.
    log.append_served_draws(&draws_for("t1", &[(SUSHI.into(), 1)])).unwrap();
    assert_eq!(levels(&log, "salmon"), s(&[(FREEZER, 1600), ("kitchen", 850)]));
    // A cancelled order moves nothing and leaves nothing held.
    place(&mut log, "o4", &[(SUSHI, 3)]);
    cook(&mut log, "o4", false);
    assert_eq!(levels(&log, "salmon"), s(&[(FREEZER, 1600), ("kitchen", 850)]));
    assert!(log.journal().unwrap().stores.held.is_empty());
    // Unbound again: Poster's rule (the kitchen received last).
    log.bind_station("sushi", "", "owner").unwrap();
    sell(&mut log, "o5", &[(SUSHI, 1)]);
    assert_eq!(levels(&log, "salmon"), s(&[(FREEZER, 1600), ("kitchen", 750)]));
    assert_sound(&log);
}

/// A reservation of a bound station survives a checkpoint written between
/// it and its `consumed` (section `B`), and the binding itself (section `K`).
#[test]
fn a_bound_reservation_survives_a_checkpoint() {
    let mut log = StockLog::create_sized(128 * 1024).unwrap();
    log.set_checkpoint_every(1);
    log.receive_with("lemon", 40, &into("kitchen")).unwrap();
    log.bind_station("bar", "bar", "owner").unwrap();
    place(&mut log, "o1", &[(DRINK, 4)]);
    log.receive_with("rice", 10, &into("kitchen")).unwrap();
    let cp = log.journal_now().unwrap();
    assert_eq!(cp.stores.bound().get("bar").map(String::as_str), Some("bar"));
    assert_eq!(cp.stores.held.len(), 1, "the checkpoint carries the held share");
    cook(&mut log, "o1", true);
    assert_eq!(levels(&log, "lemon"), s(&[("bar", -4), ("kitchen", 40)]), "drawn from the bar even though it holds none");
    assert_sound(&log);
}

/// LACK OF STOCK is decided as on the unbound path: the WHOLE shelf. A bound
/// storage that holds none goes negative there; nothing falls through.
#[test]
fn a_bound_storage_without_stock_behaves_like_the_unbound_path() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.receive_with("lemon", 3, &into("kitchen")).unwrap();
    log.bind_station("bar", "bar", "owner").unwrap();
    let n = log.len();
    assert!(log.append_draws(&draws_for("big", &[(DRINK.into(), 4)])).is_err(), "4 of a counted 3: refused as always");
    assert_eq!(log.len(), n);
    sell(&mut log, "ok", &[(DRINK, 3)]);
    assert_eq!(levels(&log, "lemon"), s(&[("bar", -3), ("kitchen", 3)]));
    assert_eq!(log.ledger().unwrap().level("lemon").on_hand, 0);
}

/// REFUSED, nothing written: an unknown or archived storage (the words name
/// it), a station that is not one, unsigned. A bound storage is never
/// archived; unbound, it can be.
#[test]
fn a_binding_names_an_open_storage_of_this_venue() {
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.put_storage(&Storage { id: "cellar".into(), name: "Cellar".into(), archived: false }).unwrap();
    log.put_storage(&Storage { id: "old".into(), name: "Old".into(), archived: true }).unwrap();
    let n = log.len();
    let e = log.bind_station("bar", "attic", "owner").unwrap_err().to_string();
    assert!(e.contains("attic"), "the refusal names the storage: {e}");
    let e = log.bind_station("bar", "old", "owner").unwrap_err().to_string();
    assert!(e.contains("old"), "{e}");
    assert!(log.bind_station("grill", "cellar", "owner").is_err());
    assert!(matches!(log.bind_station("bar", "cellar", " "), Err(StockError::Unsigned)));
    assert_eq!(log.len(), n, "a refused binding wrote something");
    log.bind_station("bar", "cellar", "owner").unwrap();
    assert_eq!(log.bindings().unwrap().get("bar").map(String::as_str), Some("cellar"));
    let archive = Storage { id: "cellar".into(), name: "Cellar".into(), archived: true };
    let e = log.put_storage(&archive).unwrap_err().to_string();
    assert!(e.contains("bar"), "{e}");
    log.bind_station("bar", "", "owner").unwrap();
    log.put_storage(&archive).unwrap();
    assert!(log.bindings().unwrap().is_empty());
}

/// AN UNBOUND VENUE DRAWS EXACTLY AS BEFORE: dishes with stations and no
/// binding write the very bytes a draw with no station knowledge writes, no
/// record carries `drawn`, and the storages stay unused.
#[test]
fn an_unbound_venue_draws_as_before() {
    let history = |strip: bool| {
        let mut log = StockLog::create_sized(64 * 1024).unwrap();
        log.set_checkpoint_every(4);
        log.set_clock(1_759_000_000_000);
        log.receive_with("salmon", 2000, &Meta::default()).unwrap();
        log.receive_with("lemon", 50, &Meta::default()).unwrap();
        for i in 0..6 {
            let lines = vec![(SUSHI.to_string(), 1 + i % 2), (DRINK.to_string(), 2), (PLATE.to_string(), 1)];
            let mut d: Vec<Draw> = draws_for(&format!("o{i}"), &lines);
            if strip {
                d.iter_mut().for_each(|x| x.stations.clear());
            }
            log.append_draws(&d).unwrap();
            cook(&mut log, &format!("o{i}"), i % 3 != 0);
        }
        log
    };
    let (with, without) = (history(false), history(true));
    assert_eq!(with.to_bytes_trimmed(), without.to_bytes_trimmed(), "station knowledge changed an unbound venue's bytes");
    assert!(with.raw().iter().all(|r| !r.contains("drawn")));
    assert!(!with.journal().unwrap().stores.is_used());
}

/// AN OLD IMAGE that then binds a station: its old checkpoints still verify
/// byte for byte, and the fold of the old part is unchanged.
#[test]
fn an_old_image_folds_byte_equal_after_a_binding() {
    let mut log = old_history();
    let before = fold_text(&log.journal().unwrap());
    log.bind_station("sushi", FREEZER, "owner").unwrap();
    assert_eq!(fold_text(&log.journal().unwrap()), before, "a binding moved the shelf, the cost, a lot or the carry");
    log.verify_checkpoints().unwrap();
    let back = StockLog::load(&log.to_bytes_trimmed()).unwrap();
    assert_eq!(back.bindings().unwrap().get("sushi").map(String::as_str), Some(FREEZER));
}

/// The `drawn` key's own edges: merged per storage, "" for an unbound share,
/// a malformed key read as none.
#[test]
fn the_drawn_key_round_trips() {
    let mut b = Bound::new();
    assert_eq!(resolve(&b, &[("sushi".into(), 5)]), None, "nothing bound: no key");
    set(&mut b, "sushi", "freezer");
    set(&mut b, "bar", "freezer");
    let k = resolve(&b, &[("sushi".into(), 5), ("bar".into(), 2), ("kitchen".into(), 3)]).unwrap();
    assert_eq!(k, "=3;freezer=7");
    assert_eq!(parse_drawn(&k), Some(vec![(String::new(), 3), ("freezer".into(), 7)]));
    assert_eq!(resolve(&b, &[("kitchen".into(), 3)]), None, "only unbound shares: no key");
    assert_eq!(parse_drawn("freezer=0"), None);
    assert_eq!(parse_drawn("Bad Id=3"), None);
    assert_eq!(parse_drawn("freezer"), None);
    assert_eq!(station_of(SUSHI), "sushi");
    assert_eq!(station_of(PLATE), "kitchen");
    assert_eq!(station_of(r#"{"station":"grill"}"#), "kitchen");
}

/// OPERATOR 2026-10-05: an AMENDED room round (`room::rules::restock`,
/// released and reserved again) keeps drawing from the bound storage; with
/// nothing bound it writes the very bytes `append_all` wrote before.
#[test]
fn an_amended_room_round_draws_from_the_bound_storage() {
    use serde_json::json;
    let items = vec![json!({ "product_id": "roll", "quantity": 2 }), json!({ "product_id": "plate", "quantity": 1 })];
    let boms = vec![("roll".to_string(), SUSHI.to_string()), ("plate".to_string(), PLATE.to_string())];
    let run = |bind: bool| {
        let mut log = StockLog::create_sized(64 * 1024).unwrap();
        log.set_clock(1_759_000_000_000);
        log.receive_with("salmon", 2000, &into(FREEZER)).unwrap();
        log.receive_with("salmon", 1000, &into("kitchen")).unwrap();
        log.receive_with("rice", 500, &into("kitchen")).unwrap();
        if bind {
            log.bind_station("sushi", FREEZER, "owner").unwrap();
        }
        place(&mut log, "r1", &[(SUSHI, 1)]);
        crate::room::rules::restock(&mut log, "r1", &items, &boms).unwrap();
        cook(&mut log, "r1", true);
        log
    };
    let bound = run(true);
    assert_eq!(levels(&bound, "salmon"), s(&[(FREEZER, 1800), ("kitchen", 950)]), "200 g sushi from the freezer, 50 g plate from the kitchen");
    assert_sound(&bound);
    // Unbound: the bytes of the pre-W-STORE2 door (append_all, no meta).
    let unbound = run(false);
    let mut old = StockLog::create_sized(64 * 1024).unwrap();
    old.set_clock(1_759_000_000_000);
    old.receive_with("salmon", 2000, &into(FREEZER)).unwrap();
    old.receive_with("salmon", 1000, &into("kitchen")).unwrap();
    old.receive_with("rice", 500, &into("kitchen")).unwrap();
    place(&mut old, "r1", &[(SUSHI, 1)]);
    let led = old.ledger().unwrap();
    let lines: Vec<(String, i64)> = vec![(SUSHI.into(), 2), (PLATE.into(), 1)];
    let mut evs = settle(&led, "r1", false);
    evs.extend(crate::stock::reservations_for("r1", &lines));
    old.append_all(&evs).unwrap();
    cook(&mut old, "r1", true);
    assert_eq!(unbound.to_bytes_trimmed(), old.to_bytes_trimmed(), "an unbound amended round changed its bytes");
}
