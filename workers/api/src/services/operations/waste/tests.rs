//! A11 (§2.1): the waste report folds BOTH logs, each row one event with its signer.
//!
//! The order log is written by the REAL `command::amend::decide`, so a change to
//! what an amendment records shows up here rather than in a hand-made fixture.

use super::*;
use crate::command::amend::{self, AmendIn, Op};
use crate::hubdo::OrderView;
use dowiz_hub::stock::meta::Meta;
use dowiz_hub::stock::{StockLog, WasteReason};
use dowiz_hub::{EventKind, Hub};

const T0: u64 = 1_790_000_000_000;

fn placed(id: &str) -> Value {
    json!({
        "id": id, "status": "CONFIRMED", "location_id": "v1",
        "items": [
            {"product_id": "maki", "quantity": 2, "unit_price": 600, "name": "Maki"},
            {"product_id": "beer", "quantity": 1, "unit_price": 300, "name": "Beer"}
        ],
        "subtotal": 1500, "discount": 0, "delivery_fee": 0, "tip": 0, "total": 1500,
        "fulfilment": {"kind": "dine_in", "table": "7"}
    })
}

/// A round placed, moved to `status`, and then amended by the real command.
fn amended(hub: &mut Hub, id: &str, status: &str, line: usize, reason: &str, by: &str) {
    hub.append(EventKind::Placed, id, &placed(id).to_string(), T0, [0u8; 32]).unwrap();
    hub.append(EventKind::Advanced, id, &json!({"_d": true, "status": status}).to_string(), T0 + 1, [0u8; 32])
        .unwrap();
    let state = crate::fold::fold(hub.history(id).iter().map(|e| e.order_json.as_str()));
    let current = OrderView { order_id: id.into(), kind: EventKind::Advanced as u8, seq: T0 + 1, order_json: state.to_string() };
    let input = AmendIn {
        order_id: id.into(), location_id: "v1".into(), base_seq: T0 + 1,
        ops: vec![Op::Remove { line }], by: by.into(), reason: Some(reason.into()),
        may_void: true, boms: vec![], now_ms: (T0 + 5) as i64,
    };
    let mut shelf = StockLog::create_sized(64 * 1024).unwrap();
    amend::decide(hub, &mut shelf, Some(&current), &input).expect("the amendment lands");
}

fn binned(item: &str, qty: i64, reason: WasteReason, by: &str) -> StockEvent {
    StockEvent::Wasted { item: item.into(), qty, reason, by: by.into() }
}

/// §2.1 CHECK, natively: bin 2 as `dropped`, void one line after `PREPARING`
/// with `dropped` -> 3 rows, 2 of them stock events, each with its signer.
#[test]
fn the_report_reads_both_logs() {
    let stock = vec![
        StockEvent::Received { item: "rice".into(), qty: 100 },
        binned("rice", 1, WasteReason::Dropped, "p_anna"),
        binned("rice", 1, WasteReason::Dropped, "p_ben"),
    ];
    let mut hub = Hub::create_sized(64 * 1024).unwrap();
    amended(&mut hub, "r1", "PREPARING", 0, "dropped", "p_cara");
    let rows = fold(&stock, &hub.events_oldest_first());
    assert_eq!(rows.len(), 3, "{rows:?}");
    assert_eq!(rows.iter().filter(|r| r.source == "stock").count(), 2);
    let void = rows.iter().find(|r| r.source == "void").unwrap();
    assert_eq!(
        (void.item.as_str(), void.qty, void.by.as_deref(), void.order.as_deref(), void.at),
        ("maki", 2, Some("p_cara"), Some("r1"), Some((T0 + 5) as i64)),
    );
    let t = totals(&rows);
    assert_eq!(t["byReason"]["stock"]["dropped"], 2);
    assert_eq!(t["byReason"]["void"]["dropped"], 2);
    assert_eq!(t["bySigner"]["void"]["p_cara"], 2);
}

/// Only what was BINNED: a void before the kitchen went back on the shelf,
/// and a void after it for another reason is not a dropped plate. Positive
/// twin: the same void after the kitchen, `dropped`, is a row.
#[test]
fn only_dropped_voids_after_the_kitchen_are_waste() {
    let mut hub = Hub::create_sized(64 * 1024).unwrap();
    amended(&mut hub, "before", "CONFIRMED", 0, "dropped", "p1");
    amended(&mut hub, "mistake", "READY", 0, "mistake", "p1");
    assert!(fold(&[], &hub.events_oldest_first()).is_empty());
    amended(&mut hub, "after", "READY", 1, "dropped", "p1");
    let rows = fold(&[], &hub.events_oldest_first());
    assert_eq!(rows.len(), 1);
    assert_eq!((rows[0].item.as_str(), rows[0].order.as_deref()), ("beer", Some("after")));
}

/// A write-off recorded before signers is REPORTED, as unsigned -- not
/// dropped from the report, not given somebody's name.
#[test]
fn an_unsigned_old_write_off_is_reported_as_unsigned() {
    let rows = fold(&[binned("tuna", 3, WasteReason::Spoiled, ""), binned("tuna", 1, WasteReason::Unsold, "p1")], &[]);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].by, None);
    assert_eq!(rows[1].by.as_deref(), Some("p1"));
    assert_eq!(totals(&rows)["bySigner"]["stock"]["(unsigned)"], 3);
}

/// Lifecycle movements are not waste.
#[test]
fn received_reserved_and_counted_are_not_waste() {
    let stock = vec![
        StockEvent::Received { item: "rice".into(), qty: 9 },
        StockEvent::Reserved { item: "rice".into(), qty: 2, order_id: "o".into() },
        StockEvent::Stocktake { item: "rice".into(), observed: 5, stocktake_id: "s".into(), by: "p".into() },
    ];
    assert!(fold(&stock, &[]).is_empty());
    assert_eq!(fold(&[binned("rice", 1, WasteReason::StaffMeal, "p")], &[]).len(), 1);
}

fn back(item: &str, qty: i64, resell: bool) -> StockEvent {
    StockEvent::Returned {
        item: item.into(), qty, order_id: "o9".into(), resell,
        by: "courier-7".into(), chosen_by: "owner1".into(),
    }
}

/// §2.4: food back from a door and binned is waste, reason `returned`, with the
/// courier as `by`, the chooser beside it and the order named.
#[test]
fn food_binned_after_a_refused_door_is_a_returned_row() {
    let rows = fold(&[back("rice", 200, false)], &[]);
    assert_eq!(rows, vec![WasteRow {
        source: "stock", item: "rice".into(), qty: 200, reason: "returned".into(),
        by: Some("courier-7".into()), order: Some("o9".into()), at: None, chosen_by: Some("owner1".into()),
        value: None, lot: None,
    }]);
    let t = totals(&rows);
    assert_eq!(t["byReason"]["stock"]["returned"], json!(200));
    assert_eq!(t["bySigner"]["stock"]["courier-7"], json!(200));
}

/// Twin: the same food resold is back on the shelf, not waste.
#[test]
fn resold_food_is_not_waste() {
    assert!(fold(&[back("rice", 200, true)], &[]).is_empty());
    assert_eq!(fold(&[back("rice", 200, true), back("nori", 2, false)], &[]).len(), 1);
}

/// R5: A WRITE-OFF CARRIES ITS VALUE AND ITS DATE, through the real log: a
/// priced delivery, a write-off stamped with the average of its moment and
/// dated by the clock, and an old undated unvalued one beside it. The totals
/// sum money across reasons and count what could not be valued.
#[test]
fn a_write_off_is_dated_and_valued_and_an_old_one_is_not_invented() {
    use dowiz_hub::stock::StockLog;
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    log.append(&binned("nori", 0, WasteReason::Spoiled, "p")).unwrap_err();
    log.append(&StockEvent::Received { item: "nori".into(), qty: 10 }).unwrap();
    log.append(&binned("nori", 1, WasteReason::Spoiled, "p_old")).unwrap();
    log.set_clock(1_790_000_000_000);
    log.receive_with("salmon", 1000, &Meta { unit_cost: Some(3000), per: Some(1000), ..Meta::default() }).unwrap();
    log.append_with(&binned("salmon", 200, WasteReason::Dropped, "p_cook"), &Meta { lot: Some("L1".into()), ..Meta::default() })
        .unwrap();
    let j = log.journal().unwrap();
    let rows = fold_entries(&j.entries, &[]);
    assert_eq!(rows.len(), 2);
    assert_eq!((rows[0].at, rows[0].value), (None, None), "the old one: no date, no invented value");
    assert_eq!((rows[1].at, rows[1].value, rows[1].lot.as_deref()), (Some(1_790_000_000_000), Some(600), Some("L1")));
    let t = totals(&rows);
    assert_eq!((t["value"].clone(), t["unvalued"].clone()), (json!(600), json!(1)));
    assert_eq!(t["valueByReason"]["dropped"], json!(600));
    // Prep is not waste.
    let prep = StockEvent::Produced {
        item: "salmon".into(), qty: 100, out: 55, stage: dowiz_hub::stock::PrepStage::Clean, into: None, by: "p".into(),
    };
    assert!(fold(&[prep], &[]).is_empty());
}
