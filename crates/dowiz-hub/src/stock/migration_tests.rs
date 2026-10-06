//! THE MIGRATION LAWS OF THE STOCK RECORD FORMAT (W-TASTE2 S7c; R-S7 research 2026-10-06 §E6).
//!
//! A schema change is an inclusion F: old fields -> new fields. bebop never rewrites the log: the
//! DECODER supplies the new field's default (Sigma_F, "by = ''" for a record written before
//! 2026-09-23, commit 405da324). Two laws make that safe, and `tools/gates/migration-law.sh` refuses
//! a format change that does not name a test holding them for the new version:
//!   1. Delta o Sigma = id: an old record, decoded and written again, then stripped of the new
//!      fields, is the old record field for field (nothing the old writer said is lost or changed);
//!   2. readers agree: decode(old) == decode(encode(decode(old))) (re-encoding is a fixed point).
//! Version 1 of every kind holds the base law instead: decode(encode(x)) == x.

use super::{decode, encode, PrepStage, StockEvent, WasteReason};
use serde_json::{Map, Value};

fn fields(rec: &str) -> Map<String, Value> {
    serde_json::from_str::<Value>(rec).ok().and_then(|v| v.as_object().cloned()).unwrap_or_default()
}

/// Both laws for `old` (a record as an older writer wrote it) that gained `added` fields.
fn laws(old: &str, added: &[&str]) {
    let first = decode(old).unwrap_or_else(|| panic!("an old record must still decode: {old}"));
    let again = encode(&first);
    let mut delta = fields(&again);
    for f in added {
        assert!(delta.remove(*f).is_some(), "the new writer writes {f}: {again}");
    }
    assert_eq!(delta, fields(old), "law 1, Delta(Sigma(old)) == old: {old} -> {again}");
    assert_eq!(decode(&again), Some(first), "law 2, readers agree on {old}");
}

/// Base law, every kind at its current version: decode(encode(x)) == x.
#[test]
fn every_kind_round_trips() {
    let s = |x: &str| x.to_string();
    let all = vec![
        StockEvent::Received { item: s("rice"), qty: 5 },
        StockEvent::Reserved { item: s("rice"), qty: 2, order_id: s("o1") },
        StockEvent::Consumed { item: s("rice"), qty: 2, order_id: s("o1") },
        StockEvent::Released { item: s("rice"), qty: 1, order_id: s("o1") },
        StockEvent::Wasted { item: s("rice"), qty: 1, reason: WasteReason::Spoiled, by: s("u1") },
        StockEvent::Stocktake { item: s("rice"), observed: 9, stocktake_id: s("st1"), by: s("u1") },
        StockEvent::Served { item: s("rice"), qty: 1, order_id: s("o2") },
        StockEvent::Unserved { item: s("rice"), qty: 1, order_id: s("o2") },
        StockEvent::Returned { item: s("rice"), qty: 1, order_id: s("o3"), resell: true, by: s("c1"), chosen_by: s("u1") },
        StockEvent::Produced { item: s("salmon"), qty: 1000, out: 700, stage: PrepStage::Clean, into: Some(s("fillet")), by: s("u1") },
        StockEvent::Produced { item: s("rice"), qty: 1000, out: 2400, stage: PrepStage::Cook, into: None, by: s("u1") },
        StockEvent::Removed { item: s("rice"), by: s("u1") },
        StockEvent::Cooked { item: s("rice"), qty: 300, into: s("sushi-rice"), act: s("a1"), by: s("u1") },
        StockEvent::Made { item: s("sushi-rice"), qty: 900, planned: 950, gross: 1000, act: s("a1"), by: s("u1") },
    ];
    for ev in all {
        let rec = encode(&ev);
        assert_eq!(decode(&rec).as_ref(), Some(&ev), "{rec}");
    }
}

/// `wasted` v2 (405da324, 2026-09-23): gained `by`.
#[test]
fn wasted_gained_by_and_the_laws_hold() {
    laws(r#"{"k":"wasted","item":"rice","qty":3,"reason":"spoiled"}"#, &["by"]);
    assert_eq!(decode(r#"{"k":"wasted","item":"rice","qty":3,"reason":"spoiled"}"#),
        Some(StockEvent::Wasted { item: "rice".into(), qty: 3, reason: WasteReason::Spoiled, by: String::new() }), "Sigma: by = \"\"");
}

/// `stocktake` v2 (405da324, 2026-09-23): gained `by`.
#[test]
fn stocktake_gained_by_and_the_laws_hold() {
    laws(r#"{"k":"stocktake","item":"rice","observed":7,"id":"st-1"}"#, &["by"]);
}
