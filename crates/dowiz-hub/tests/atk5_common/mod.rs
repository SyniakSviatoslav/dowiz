//! W-MG-ATK5: the fixtures every attack on the merge shares. Real ML-DSA-65, real files, the real
//! `SealedHub::merge`. Nothing here re-implements a rule.
#![allow(dead_code)]

use einstore::evlog::doc::keys::KeyOp;
use dowiz_hub::doc_seal::merge::{merge_report, Merged, Recheck};
use dowiz_hub::doc_seal::{DeviceKey, SealedHub};
use dowiz_hub::stock::{StockEvent, StockLedger, WasteReason};

pub fn key(seed: u8) -> DeviceKey {
    DeviceKey::from_seed(&[seed; 32])
}
pub fn a() -> DeviceKey {
    key(0xA1)
}
pub fn b() -> DeviceKey {
    key(0xB2)
}
pub fn c() -> DeviceKey {
    key(0xC3)
}
pub fn received(item: &str, qty: i64) -> StockEvent {
    StockEvent::Received { item: item.to_string(), qty }
}
pub fn served(item: &str, qty: i64, order: &str) -> StockEvent {
    StockEvent::Served { item: item.to_string(), qty, order_id: order.to_string() }
}
pub fn unserved(item: &str, qty: i64, order: &str) -> StockEvent {
    StockEvent::Unserved { item: item.to_string(), qty, order_id: order.to_string() }
}
pub fn reserved(item: &str, qty: i64, order: &str) -> StockEvent {
    StockEvent::Reserved { item: item.to_string(), qty, order_id: order.to_string() }
}
pub fn consumed(item: &str, qty: i64, order: &str) -> StockEvent {
    StockEvent::Consumed { item: item.to_string(), qty, order_id: order.to_string() }
}
pub fn released(item: &str, qty: i64, order: &str) -> StockEvent {
    StockEvent::Released { item: item.to_string(), qty, order_id: order.to_string() }
}
pub fn wasted(item: &str, qty: i64) -> StockEvent {
    StockEvent::Wasted { item: item.to_string(), qty, reason: WasteReason::Spoiled, by: "p".to_string() }
}
pub fn counted(item: &str, observed: i64, id: &str) -> StockEvent {
    StockEvent::Stocktake { item: item.to_string(), observed, stocktake_id: id.to_string(), by: "p".to_string() }
}
pub fn removed(item: &str) -> StockEvent {
    StockEvent::Removed { item: item.to_string(), by: "p".to_string() }
}

pub fn opened(bytes: &[u8], seed: u8) -> SealedHub {
    let mut sh = SealedHub::load(bytes).unwrap();
    sh.set_entropy([seed; 32]);
    sh
}
pub fn trimmed(sh: &SealedHub) -> Vec<u8> {
    sh.to_bytes_trimmed()
}
pub fn holds(bytes: &[u8]) -> bool {
    SealedHub::load(bytes).unwrap().verify().unwrap().holds()
}

/// One genesis: A writes `past`, admits B and C. Then the devices part.
pub fn genesis(past: &[StockEvent]) -> Vec<u8> {
    let (ka, kb, kc) = (a(), b(), c());
    let mut sh = SealedHub::create(&ka).unwrap();
    sh.set_entropy([1; 32]);
    sh.sync_stock(past, &ka, 1).unwrap();
    sh.commit_keyop(&KeyOp::Admit(kb.key()), &ka, 2).unwrap();
    sh.commit_keyop(&KeyOp::Admit(kc.key()), &ka, 2).unwrap();
    trimmed(&sh)
}
/// `file` after `key` synced `moves` offline.
pub fn offline(file: &[u8], seed: u8, key: &DeviceKey, moves: &[StockEvent]) -> Vec<u8> {
    let mut sh = opened(file, seed);
    sh.sync_stock(moves, key, 100).unwrap();
    trimmed(&sh)
}
/// `ours` with `theirs` merged in by `key`, the file and what the merge said.
pub fn merged(ours: &[u8], theirs: &[u8], key: &DeviceKey, seed: u8) -> (Vec<u8>, Merged) {
    let mut sh = opened(ours, seed);
    let m = sh.merge(theirs, key, 500).unwrap();
    (trimmed(&sh), m)
}
/// The conflicts a `Sealed` names, as debug text; `None` when no MERGE was written.
pub fn named(m: &Merged) -> Option<String> {
    match m {
        Merged::Sealed { conflicts, .. } => Some(format!("{conflicts:?}")),
        _ => None,
    }
}
/// `(recheck of each MERGE, doc_verify's clean)`.
pub fn audit(bytes: &[u8]) -> (Vec<Recheck>, bool) {
    let sh = SealedHub::load(bytes).unwrap();
    (sh.merge_lines().iter().map(|l| l.recheck).collect(), merge_report(&sh, true).1)
}
pub fn fold(bytes: &[u8]) -> Result<StockLedger, String> {
    StockLedger::history(&SealedHub::load(bytes).unwrap().stock_events()).map_err(|e| e.to_string())
}
