//! The object's log mark (W-HUBCRC): taken by a load, carried across the object's own log
//! writes, dropped whenever bytes enter from storage or a whole-image write. The RED proof
//! is `a_cold_read_drops_the_mark_and_storage_is_scanned_whole`: one named cell changed in
//! storage, which a kept mark would skip. Its twins for the stock log and the entry logs
//! (wallet ledger, till) are the `*_mark_*` tests at the end: one named changed byte in
//! storage, at or below the record the mark names, which only a full scan quarantines.
use super::carried;
use crate::fold::projection::Written;
use crate::hubdo::host::mem::{Harness, T0};
use crate::wire::Call;
use serde_json::{json, Value};
use worker::Method;

fn placed(id: &str) -> Value {
    json!({
        "kind": dowiz_hub::EventKind::Placed as u8,
        "order_id": id,
        "payload": json!({ "id": id, "status": "PENDING", "total": 1200 }).to_string(),
        "clock": T0 as u64,
    })
}

fn append(h: &Harness, generation: i64, id: &str) -> worker::Result<crate::wire::Reply> {
    h.try_call(Call::new("https://hub/fold/append", Method::Post).unwrap().with_header("x-generation", &generation.to_string()).with_json(&placed(id)))
}

fn mark_gen(h: &Harness) -> Option<i64> {
    mark_of(h, "log")
}

fn mark_of(h: &Harness, id: &str) -> Option<i64> {
    h.obj.log_marks.borrow().get(id).map(|(g, _)| *g)
}

/// The rule, as a table: carried only by the object's own writes, from the generation it
/// describes, to the generation the write made.
#[test]
fn carried_only_across_the_objects_own_writes_from_its_generation() {
    let mut hub = dowiz_hub::Hub::create_sized(64 * 1024).unwrap();
    hub.append(dowiz_hub::EventKind::Placed, "o1", "{}", 1, [0u8; 32]).unwrap();
    let (_, m) = dowiz_hub::Hub::load_since(&hub.to_bytes_trimmed(), None).unwrap();
    let m = m.unwrap();
    let ev = dowiz_hub::Event { kind: dowiz_hub::EventKind::Placed, order_id: "o2".into(), order_json: "{}".into(), seq: 2 };
    assert_eq!(carried(Some((4, m)), &Written::Appended(ev.clone()), 4, 5), Some((5, m)));
    assert_eq!(carried(Some((4, m)), &Written::Log(vec![ev.clone()]), 4, 5), Some((5, m)));
    assert_eq!(carried(Some((4, m)), &Written::Tail(vec![ev.clone()]), 4, 5), Some((5, m)), "W-LOOPB's appended-only command write");
    assert_eq!(carried(Some((4, m)), &Written::Whole, 4, 5), None, "a whole-image write drops it");
    assert_eq!(carried(Some((3, m)), &Written::Appended(ev), 4, 5), None, "a mark of another generation is dropped");
    assert_eq!(carried(None, &Written::Whole, 4, 5), None);
}

/// POSITIVE: every append leaves the mark at the generation it wrote, and the log the next
/// turn reads through it is the whole log.
#[test]
fn appends_carry_the_mark_and_read_through_it() {
    let h = Harness::new();
    for (g, id) in ["o1", "o2", "o3"].iter().enumerate() {
        assert_eq!(append(&h, g as i64, id).unwrap().status_code(), 200);
        // The first append CREATES the log in memory -- nothing was loaded, so nothing is marked.
        let want = if g == 0 { None } else { Some(g as i64 + 1) };
        assert_eq!(mark_gen(&h), want, "after {id}");
    }
    assert_eq!(h.get("/fold/orders").body_value().as_array().map(Vec::len), Some(3));
}

/// A WHOLE-IMAGE PUT (`/img/log`) DROPS THE MARK: those bytes came from outside the object.
#[test]
fn a_whole_image_put_drops_the_mark() {
    let h = Harness::new();
    append(&h, 0, "o1").unwrap();
    append(&h, 1, "o2").unwrap();
    assert_eq!(mark_gen(&h), Some(2));
    let bytes = h.obj.mem.borrow().get("log").map(|(_, b)| b.clone()).unwrap();
    assert_eq!(h.put("log", 2, &bytes).status_code(), 200);
    assert_eq!(mark_gen(&h), None, "dropped by the put");
    assert_eq!(append(&h, 3, "o3").unwrap().status_code(), 200, "the next load scans in full and marks again");
    assert_eq!(mark_gen(&h), Some(4));
}

/// The log's newest chain id, as the 32 raw bytes the image stores it as.
fn tip_bytes(h: &Harness) -> Vec<u8> {
    let hex = dowiz_hub::Hub::load(&h.obj.mem.borrow().get("log").unwrap().1).unwrap().tip().unwrap();
    (0..32).map(|i| u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap()).collect()
}

/// THE RED PROOF. Three appends; then one byte of record o2's `prev` (payload cells 7..10,
/// which hold o1's id) is changed IN STORAGE and the object loses its copy. The next append
/// re-reads storage and must refuse it, as a cold object does. o2 is the record the object's
/// mark names (taken by the load before o3's append, carried across it), so a mark kept
/// across that read would anchor on o2 and never hash it. Positive twin: the same eviction
/// without the change appends.
///
/// FOUND BY ADJACENCY: a record stores its id (cells 3..6) and then its `prev` (cells
/// 7..10), so o2's `prev` is the 32 bytes right after o2's id where those bytes are o1's id.
/// Two earlier versions did not discriminate, both caught by running with the forget
/// removed: the LAST copy of an id is a ROOT's cell (every load checks the root), and o3's
/// `prev` is NEWER than the mark (the carried anchor stays on o2, so o3 is always hashed).
#[test]
fn a_cold_read_drops_the_mark_and_storage_is_scanned_whole() {
    for corrupt in [false, true] {
        let h = Harness::new();
        append(&h, 0, "o1").unwrap();
        let o1_id = tip_bytes(&h);
        append(&h, 1, "o2").unwrap();
        let o2_pair: Vec<u8> = tip_bytes(&h).into_iter().chain(o1_id.iter().copied()).collect();
        append(&h, 2, "o3").unwrap();
        assert_eq!(mark_gen(&h), Some(3));
        if corrupt {
            let mut kv = h.host.kv.borrow_mut();
            let crate::hubdo::host::mem::Stored::Bytes(b) = kv.get_mut("c:log:0").expect("chunk 0") else { panic!("chunk is not bytes") };
            let at = b.windows(64).position(|w| w == o2_pair.as_slice()).expect("o2's id then its prev, in chunk 0");
            b[at + 32 + 1] ^= 0x01;
        }
        h.obj.mem.borrow_mut().remove("log");
        match (corrupt, append(&h, 3, "o4")) {
            (false, Ok(r)) => assert_eq!(r.status_code(), 200),
            (true, Err(e)) => assert!(e.to_string().contains("unreadable"), "{e}"),
            (c, r) => panic!("corrupt={c}: {:?}", r.map(|r| r.status_code())),
        }
        if corrupt {
            assert!(append(&h.cold(), 3, "o4").is_err(), "a cold object refuses the same storage");
        }
    }
}

// ── the stock log and the entry logs (W-HUBCRC follow-up) ──

/// Changes one byte of `needle` (it must occur exactly once) in chunk 0 of `id` in STORAGE.
fn corrupt_stored(h: &Harness, id: &str, needle: &[u8]) {
    let mut kv = h.host.kv.borrow_mut();
    let key = format!("c:{id}:0");
    let crate::hubdo::host::mem::Stored::Bytes(b) = kv.get_mut(&key).expect("chunk 0") else { panic!("chunk is not bytes") };
    let at = b.windows(needle.len()).position(|w| w == needle).expect("the needle is stored");
    assert_eq!(b.windows(needle.len()).rposition(|w| w == needle), Some(at), "the needle is stored once");
    b[at + 1] ^= 0x01;
}

/// One stock turn the way `stock_move` / `write_both` take it: `stock_log` (-> `load_stock`),
/// one append, `put_derived`.
fn stock_turn(h: &Harness, item: &str, qty: i64) {
    crate::edge::mem::block_on(async {
        let (gen, mut log) = h.obj.stock_log().await.unwrap();
        log.append(&dowiz_hub::stock::StockEvent::Received { item: item.into(), qty }).unwrap();
        assert!(h.obj.put_derived(crate::hubstore::IMAGE_STOCK, gen, &log.to_bytes_trimmed()).await.unwrap().is_some());
    })
}

/// How many records the stock log quarantines, as the object loads it now.
fn stock_bad(h: &Harness) -> usize {
    crate::edge::mem::block_on(h.obj.stock_log()).unwrap().1.quarantined()
}

/// THE STOCK LOG'S RED PROOF. Three turns leave the mark on the second record (taken by the
/// third turn's load, carried across its write). One byte of that record ("nori-r2") is
/// changed in storage and the object loses its copy: the next load re-reads storage and
/// must hash it -- quarantined, as a cold object quarantines it. A mark kept across the
/// read would name the record by its cell, header and id, all unchanged, and skip it.
#[test]
fn a_cold_read_drops_the_stock_mark_and_storage_is_scanned_whole() {
    let h = Harness::new();
    stock_turn(&h, "rice", 100);
    assert_eq!(mark_of(&h, "stock"), None, "the first turn creates the log: nothing loaded");
    stock_turn(&h, "nori-r2", 50);
    stock_turn(&h, "tuna", 7);
    assert_eq!(mark_of(&h, "stock"), Some(3), "carried by the object's own writes");
    assert_eq!(stock_bad(&h), 0, "positive twin: the clean log reads through the mark");
    corrupt_stored(&h, "stock", b"nori-r2");
    h.obj.mem.borrow_mut().remove("stock");
    assert_eq!(stock_bad(&h), 1, "the stored change is hashed and quarantined");
    assert_eq!(crate::edge::mem::block_on(h.cold().obj.stock_log()).unwrap().1.quarantined(), 1, "as a cold object sees it");
    assert_eq!(mark_of(&h, "stock"), None, "a quarantined stock log is not marked");
}

/// One entry-log turn the way `ledger_log` / `till_state` take it: `image` + `load_entries`,
/// one append, `put_derived`.
fn entry_turn(h: &Harness, id: &str, subject: &str) {
    crate::edge::mem::block_on(async {
        let (gen, mut log) = match h.obj.image(id).await.unwrap() {
            Some((m, b)) => (m.generation, h.obj.load_entries(id, m.generation, &b).unwrap()),
            None => (0, dowiz_hub::logimage::LogImage::create().unwrap()),
        };
        log.append("tx", subject, &format!(r#"{{"tx":"{subject}","amount":1200}}"#)).unwrap();
        assert!(h.obj.put_derived(id, gen, &log.to_bytes()).await.unwrap().is_some());
    })
}

/// The entry log as the object loads it now: `Err` = refused.
fn entries_load(h: &Harness, id: &str) -> Result<usize, dowiz_hub::HubError> {
    crate::edge::mem::block_on(async {
        let (m, b) = h.obj.image(id).await.unwrap().unwrap();
        h.obj.load_entries(id, m.generation, &b).map(|l| l.len())
    })
}

/// Where record r2's `prev` (= r1's id) sits in `now`, given `once`, the image after r1
/// alone. r2's `prev` is the one place where NEW bytes (cells that differ from `once`, or
/// lie past its end) repeat a whole non-zero 32-byte window of `once`'s arena: r2's own id
/// and its root's tip are new values, and every older copy of r1's id is unchanged.
fn r2_prev_at(once: &[u8], now: &[u8]) -> usize {
    let arena = 1024 * 8;
    let nonzero = |w: &[u8]| w.chunks(8).all(|c| c.iter().any(|&x| x != 0));
    let old: Vec<&[u8]> = (arena..once.len().saturating_sub(31)).step_by(8).map(|o| &once[o..o + 32]).filter(|w| nonzero(w)).collect();
    (arena..now.len().saturating_sub(31))
        .step_by(8)
        .find(|&o| {
            let w = &now[o..o + 32];
            let new = once.get(o..o + 32).is_none_or(|was| was != w);
            new && nonzero(w) && old.contains(&w)
        })
        .expect("r2's prev")
}

/// THE ENTRY LOGS' RED PROOF, for the wallet ledger and the till. An entry log's
/// `quarantined()` re-hashes every record on its own, so a changed payload byte would read
/// the same with or without a mark (the first version of this test, green under the
/// mutation, showed it): what only the load's scan decides is a REFUSAL. So one byte of
/// r2's `prev` -- r2 is the record the mark names after three turns -- is changed in
/// storage, and the next load after the object loses its copy must refuse the log, as a
/// cold object does. A mark kept across the read would skip r2 and load it.
#[test]
fn a_cold_read_drops_the_entry_log_mark_and_storage_is_scanned_whole() {
    let mut missed = Vec::new();
    for id in [crate::wallet::IMAGE_LEDGER, crate::command::till::IMAGE_TILL] {
        let h = Harness::new();
        entry_turn(&h, id, "leg-r1");
        let once = h.obj.mem.borrow().get(id).unwrap().1.clone();
        entry_turn(&h, id, "leg-r2");
        entry_turn(&h, id, "leg-r3");
        assert_eq!(mark_of(&h, id), Some(3), "{id}: carried by the object's own writes");
        assert_eq!(entries_load(&h, id).ok(), Some(3), "{id}: positive twin: reads through the mark");
        {
            let mut kv = h.host.kv.borrow_mut();
            let crate::hubdo::host::mem::Stored::Bytes(b) = kv.get_mut(&format!("c:{id}:0")).expect("chunk 0") else { panic!("chunk is not bytes") };
            let at = r2_prev_at(&once, b);
            b[at + 1] ^= 0x01;
        }
        h.obj.mem.borrow_mut().remove(id);
        missed.extend(entries_load(&h, id).is_ok().then_some(id));
        assert!(entries_load(&h.cold(), id).is_err(), "{id}: a cold object refuses the stored change");
    }
    assert!(missed.is_empty(), "the stored change was NOT hashed after a cold read of: {missed:?}");
}

/// A WHOLE PUT DROPS A STOCK MARK, as it drops the order log's: `/img/stock` bytes came from
/// outside the object.
#[test]
fn a_whole_stock_put_drops_the_mark() {
    let h = Harness::new();
    stock_turn(&h, "rice", 100);
    stock_turn(&h, "nori", 50);
    assert_eq!(mark_of(&h, "stock"), Some(2));
    let bytes = h.obj.mem.borrow().get("stock").map(|(_, b)| b.clone()).unwrap();
    assert_eq!(h.put("stock", 2, &bytes).status_code(), 200);
    assert_eq!(mark_of(&h, "stock"), None, "dropped by the put");
}
