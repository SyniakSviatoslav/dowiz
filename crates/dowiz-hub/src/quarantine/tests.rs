//! The append-log quarantine policy (operator 2026-10-05), one named corrupted cell each:
//! a changed payload byte is QUARANTINED and the venue keeps serving; a change that breaks
//! the walk (root, `next` ref, `prev` id) REFUSES. No fuzzing (operator rule).
use crate::logimage::LogImage;
use crate::{EventKind, Hub, HubError};
use bebop_store::Store;

fn hub_bytes(n: u64, json_len: usize) -> Vec<u8> {
    let mut hub = Hub::create_sized(64 * 1024).unwrap();
    let json = format!(r#"{{"status":"new","note":"{}"}}"#, "x".repeat(json_len));
    for i in 0..n {
        hub.append(EventKind::Placed, &format!("ord-{i}"), &json, i, [0u8; 32]).unwrap();
    }
    hub.to_bytes_trimmed()
}

/// The `at`-th record of a newest-first walk.
fn record(st: &Store, at: usize) -> usize {
    let mut o = st.follow(st.root().unwrap(), 1).unwrap();
    for _ in 0..at {
        o = st.follow(o, 2).unwrap();
    }
    o
}

/// One byte of payload cell `cell` of the `at`-th record, flipped, NOT re-sealed.
fn flipped(bytes: &[u8], at: usize, cell: usize) -> (Vec<u8>, usize) {
    let mut st = Store::from_bytes(bytes);
    let o = record(&st, at);
    st.cells[o + 2 + cell] ^= 0x100;
    (st.to_bytes(), o)
}

/// THE VENUE STAYS UP. One changed payload byte in the middle record of five: the hub
/// LOADS, serves the other four, names the fifth as `crc`, keeps the law, keeps taking
/// orders -- and the quarantine survives a reload. Before the operator's answer this
/// image was refused whole (a 500 on every request of the venue).
#[test]
fn a_changed_record_byte_is_quarantined_and_the_venue_keeps_serving() {
    let (bad, obj) = flipped(&hub_bytes(5, 20), 2, 12);
    let mut hub = Hub::load(&bad).expect("a bad record must not refuse the log");
    assert_eq!(hub.len(), 5);
    assert_eq!(hub.events().len(), 4, "the changed record is not served");
    let q = hub.quarantined();
    assert_eq!(q.len(), 1, "{q:?}");
    assert_eq!((q[0].reason, q[0].at), ("crc", 2), "named, at its place");
    assert_eq!(hub.len(), hub.events().len() + q.len(), "the law");
    assert_eq!(hub.crc_quarantined().iter().map(|b| b.obj).collect::<Vec<_>>(), vec![obj]);
    hub.append(EventKind::Placed, "ord-new", r#"{"status":"new"}"#, 9, [0u8; 32]).expect("still takes orders");
    assert_eq!(hub.events().len(), 5);
    let again = Hub::load(&hub.to_bytes_trimmed()).unwrap();
    assert_eq!(again.quarantined().len(), 1, "still quarantined after a save and reload");
}

/// A GROW MUST NOT LAUNDER IT. The copy into the doubled image carries the failed crc,
/// so the record is still quarantined afterwards rather than re-sealed into a valid one.
#[test]
fn a_grow_carries_the_quarantine_instead_of_resealing_it() {
    let (bad, _) = flipped(&hub_bytes(3, 20), 1, 12);
    let mut hub = Hub::load(&bad).unwrap();
    let before = hub.usage().capacity_cells;
    let big = format!(r#"{{"status":"new","note":"{}"}}"#, "y".repeat(400));
    let mut i = 10;
    while hub.usage().capacity_cells == before {
        hub.append(EventKind::Placed, &format!("g-{i}"), &big, i, [0u8; 32]).unwrap();
        i += 1;
        assert!(i < 2000, "the image never grew");
    }
    assert_eq!(hub.crc_quarantined().len(), 1, "carried through the grow");
    assert_eq!(hub.quarantined().iter().filter(|q| q.reason == "crc").count(), 1);
    assert_eq!(hub.len(), hub.events().len() + hub.quarantined().len());
}

/// WHERE IT STILL REFUSES (1): the `next` ref of a record, aimed one record further back.
/// The walk would skip a record; the record's own `prev` id no longer names the record
/// the ref reaches, so the image is refused and the record is named.
#[test]
fn a_changed_next_ref_refuses_the_log() {
    let bytes = hub_bytes(5, 20);
    let mut st = Store::from_bytes(&bytes);
    let (o, skip) = (record(&st, 1), record(&st, 3));
    st.cells[o + 2 + 2] = skip as i64 - o as i64;
    match Hub::load(&st.to_bytes()) {
        Err(HubError::BadCrc(b)) => assert_eq!(b.obj, o),
        Err(e) => panic!("expected BadCrc at {o}, got {e:?}"),
        Ok(_) => panic!("a changed ref was served"),
    }
    // And a ref aimed out of the image: the chain falls short, refused as before.
    let mut st = Store::from_bytes(&bytes);
    st.cells[o + 2 + 2] = 1 << 40;
    assert!(matches!(Hub::load(&st.to_bytes()), Err(HubError::Corrupt { claimed: 5, .. })));
}

/// WHERE IT STILL REFUSES (2): the `prev` id cells -- a change there cannot be told from
/// a changed ref. And (3): the root, which holds the count and the newest ref.
#[test]
fn a_changed_prev_id_or_root_refuses_the_log() {
    let bytes = hub_bytes(4, 20);
    let (bad, o) = flipped(&bytes, 1, 7);
    assert!(matches!(Hub::load(&bad), Err(HubError::BadCrc(b)) if b.obj == o));
    let mut st = Store::from_bytes(&bytes);
    let root = st.root().unwrap();
    st.cells[root + 2 + 3] ^= 0x100; // a tip cell
    assert!(matches!(Hub::load(&st.to_bytes()), Err(HubError::BadCrc(b)) if b.obj == root));
}

/// The oldest record has no older neighbour to check its link against; the count covers
/// it. A changed payload byte there is quarantined like any other.
#[test]
fn the_oldest_record_is_quarantined_too() {
    let (bad, _) = flipped(&hub_bytes(3, 20), 2, 12);
    let hub = Hub::load(&bad).unwrap();
    assert_eq!(hub.quarantined().iter().map(|q| (q.reason, q.at)).collect::<Vec<_>>(), vec![("crc", 2)]);
}

/// The audit/error log follows the same policy as the order log.
#[test]
fn logimage_quarantines_a_changed_record_byte() {
    let mut log = LogImage::create_sized(64 * 1024).unwrap();
    for i in 0..4 {
        log.append("error", &format!("s{i}"), r#"{"msg":"a message long enough"}"#).unwrap();
    }
    let (bad, _) = flipped(&log.to_bytes(), 0, 12);
    let l = LogImage::load(&bad).expect("not refused");
    assert_eq!(l.len(), 4);
    assert_eq!(l.entries().len(), 3);
    let q = l.quarantined();
    assert_eq!((q.len(), q[0].reason, q[0].at), (1, "crc", 0));
}

/// "FORGET ME" ERASES A QUARANTINED RECORD WHOLE (operator 2026-10-06). Two records are
/// corrupted (ord-1, ord-3); ord-3's person is forgotten: that record becomes a tombstone
/// holding nothing of it ({"erased":"crc"}), re-sealed, out of the quarantine; ord-1, not
/// theirs, stays quarantined and verbatim.
#[test]
fn forget_me_erases_a_quarantined_record_of_that_person_whole() {
    // Cell 15 lies inside the JSON, so the record still reads as ord-3 / ord-1.
    let (once, _) = flipped(&hub_bytes(5, 20), 1, 15); // newest first: at 1 = ord-3
    let (twice, _) = flipped(&once, 3, 15); // at 3 = ord-1
    let mut hub = Hub::load(&twice).unwrap();
    assert_eq!(hub.quarantined().len(), 2);
    let n = hub.redact(|ev| (ev.order_id == "ord-3").then(|| r#"{"status":"new"}"#.to_string())).unwrap();
    assert_eq!(n, 1, "only the forgotten person's record");
    let again = Hub::load(&hub.to_bytes_trimmed()).unwrap();
    let q = again.quarantined();
    assert_eq!(q.len(), 1, "ord-1 (not theirs) stays quarantined: {q:?}");
    assert_eq!(again.len(), 5, "the chain keeps every link");
    let erased: Vec<_> = again.events().into_iter().filter(|e| e.order_id == "ord-3").collect();
    assert_eq!(erased.len(), 1);
    assert_eq!(erased[0].order_json, crate::forget::ERASED_WHOLE, "nothing of the corrupted record is kept");
    // The tombstone holds by LINK; ord-1 (still quarantined, not theirs) is the one record that
    // fails both id schemes -- chain_check reads quarantined records unfiltered (W-CRC limit).
    let c = again.chain_check();
    assert_eq!((c.redacted, c.broken), (1, 1), "{c:?}");
}

// ── W-HUBCRC: the same policy through `Hub::load_since` (the DO's per-turn load) ──

/// What the DO holds after one turn: five events loaded (and their mark), one appended,
/// the bytes it writes back. Returns those bytes and the mark of the five.
fn appended_after_mark() -> (Vec<u8>, crate::LogMark) {
    let (mut hub, mark) = Hub::load_since(&hub_bytes(5, 20), None).unwrap();
    hub.append(EventKind::Placed, "ord-new", r#"{"status":"new"}"#, 9, [0u8; 32]).unwrap();
    (hub.to_bytes_trimmed(), mark.expect("five records have a mark"))
}

/// POSITIVE TWIN: through the mark, the appended log loads as `Hub::load` loads it, and
/// the mark it hands back covers the new record too.
#[test]
fn load_since_after_an_append_reads_as_load() {
    let (bytes, mark) = appended_after_mark();
    let (hub, next) = Hub::load_since(&bytes, Some(&mark)).expect("a clean append loads");
    let cold = Hub::load(&bytes).unwrap();
    assert_eq!((hub.len(), hub.events()), (cold.len(), cold.events()));
    let next = next.expect("a mark for the six");
    assert_ne!(next, mark, "the mark moved to the appended record");
    assert_eq!(Hub::load_since(&bytes, Some(&next)).unwrap().0.len(), 6);
}

/// THE NAMED CELL FOR THE MARKED PATH: one byte of payload cell 12 of the APPENDED record,
/// changed. Loaded through the mark it is QUARANTINED -- named, counted, not served -- exactly
/// as a cold `Hub::load` quarantines it: records newer than the mark are always hashed.
#[test]
fn a_changed_byte_in_the_appended_record_is_quarantined_through_the_mark() {
    let (bytes, mark) = appended_after_mark();
    let (bad, obj) = flipped(&bytes, 0, 12);
    let (hub, _) = Hub::load_since(&bad, Some(&mark)).expect("a bad record must not refuse the log");
    assert_eq!((hub.len(), hub.events().len()), (6, 5), "the changed record is not served");
    assert_eq!(hub.crc_quarantined().iter().map(|b| b.obj).collect::<Vec<_>>(), vec![obj], "named");
    assert_eq!(hub.events(), Hub::load(&bad).unwrap().events(), "same as the cold load");
}

/// A CHANGED `prev` (payload cell 7) IN THE APPENDED RECORD REFUSES through the mark, with
/// the same `BadCrc` the cold load refuses with.
#[test]
fn a_changed_prev_in_the_appended_record_refuses_through_the_mark() {
    let (bytes, mark) = appended_after_mark();
    let (bad, obj) = flipped(&bytes, 0, 7);
    let marked = Hub::load_since(&bad, Some(&mark)).map(|_| ()).unwrap_err();
    assert!(matches!(marked, HubError::BadCrc(b) if b.obj == obj), "{marked:?}");
    assert_eq!(format!("{marked:?}"), format!("{:?}", Hub::load(&bad).map(|_| ()).unwrap_err()));
}

/// THE COUNT IS STILL CHECKED through the mark: the appended record's `next` ref (payload
/// cell 2) cut to null ends the walk after one record where the root says six -- refused
/// as `Corrupt`, as the cold load refuses it.
#[test]
fn a_cut_chain_refuses_through_the_mark() {
    let (bytes, mark) = appended_after_mark();
    let mut st = Store::from_bytes(&bytes);
    let newest = record(&st, 0);
    st.cells[newest + 2 + 2] = 0;
    let marked = Hub::load_since(&st.to_bytes(), Some(&mark)).map(|_| ()).unwrap_err();
    assert!(matches!(marked, HubError::Corrupt { claimed: 6, chained: Some(1) }), "{marked:?}");
}

// ── W-HUBCRC follow-up: the same marked path for `LogImage` and `StockLog` ──

fn ledger_appended_after_mark() -> (Vec<u8>, crate::LogMark) {
    let mut log = LogImage::create_sized(64 * 1024).unwrap();
    for i in 0..4 {
        log.append("tx", &format!("s{i}"), r#"{"msg":"a message long enough"}"#).unwrap();
    }
    let (mut log, mark) = LogImage::load_since(&log.to_bytes(), None).unwrap();
    log.append("tx", "s4", r#"{"msg":"the turn's own record"}"#).unwrap();
    (log.to_bytes(), mark.expect("four records have a mark"))
}

/// POSITIVE TWIN (LogImage): through the mark, the appended log reads as `load` reads it.
#[test]
fn logimage_load_since_after_an_append_reads_as_load() {
    let (bytes, mark) = ledger_appended_after_mark();
    let (log, next) = LogImage::load_since(&bytes, Some(&mark)).unwrap();
    assert_eq!(log.entries(), LogImage::load(&bytes).unwrap().entries());
    assert_eq!(log.len(), 5);
    assert!(next.is_some_and(|n| n != mark), "the mark moved to the appended record");
}

/// THE NAMED CELL FOR LogImage's MARKED PATH: payload cell 12 of the APPENDED record,
/// changed. Quarantined through the mark exactly as a cold load quarantines it.
#[test]
fn logimage_a_changed_byte_in_the_appended_record_is_quarantined_through_the_mark() {
    let (bytes, mark) = ledger_appended_after_mark();
    let (bad, _) = flipped(&bytes, 0, 12);
    let (log, _) = LogImage::load_since(&bad, Some(&mark)).expect("not refused");
    assert_eq!((log.len(), log.entries().len()), (5, 4));
    let q = log.quarantined();
    assert_eq!((q.len(), q[0].reason, q[0].at), (1, "crc", 0));
    assert_eq!(log.entries(), LogImage::load(&bad).unwrap().entries(), "same as the cold load");
}

fn stock_appended_after_mark() -> (Vec<u8>, crate::LogMark) {
    use crate::stock::{StockEvent, StockLog};
    let mut log = StockLog::create_sized(64 * 1024).unwrap();
    for q in [100, 50] {
        log.append(&StockEvent::Received { item: "rice".into(), qty: q }).unwrap();
    }
    let (mut log, mark) = StockLog::load_since(&log.to_bytes_trimmed(), None).unwrap();
    log.append(&StockEvent::Received { item: "rice".into(), qty: 7 }).unwrap();
    (log.to_bytes_trimmed(), mark.expect("a clean log has a mark"))
}

/// POSITIVE TWIN (StockLog): through the mark, the shelf folds as a cold load folds it.
#[test]
fn stocklog_load_since_after_an_append_reads_as_load() {
    use crate::stock::StockLog;
    let (bytes, mark) = stock_appended_after_mark();
    let (log, next) = StockLog::load_since(&bytes, Some(&mark)).unwrap();
    assert_eq!(log.events(), StockLog::load(&bytes).unwrap().events());
    assert_eq!(log.ledger().unwrap().level("rice").on_hand, 157);
    assert!(next.is_some_and(|n| n != mark), "a clean log hands back a new mark");
}

/// THE NAMED CELL FOR StockLog's MARKED PATH: payload cell 12 of the APPENDED record
/// (the 7 rice), changed. Quarantined through the mark as by a cold load -- the shelf folds
/// from the other two -- and the log now gets NO mark: `bad` must list every bad record,
/// which a later marked scan could not see.
#[test]
fn stocklog_a_changed_byte_in_the_appended_record_is_quarantined_through_the_mark() {
    use crate::stock::StockLog;
    let (bytes, mark) = stock_appended_after_mark();
    let (bad, _) = flipped(&bytes, 0, 12);
    let (log, next) = StockLog::load_since(&bad, Some(&mark)).expect("a bad stock record must not refuse the shelf");
    assert_eq!((log.events().len(), log.quarantined()), (2, 1));
    assert_eq!(log.ledger().unwrap().level("rice").on_hand, 150);
    assert_eq!(log.events(), StockLog::load(&bad).unwrap().events(), "same as the cold load");
    assert_eq!(next, None, "a quarantined stock log is never marked");
    assert_eq!(StockLog::load_since(&bad, None).unwrap().1, None, "nor on a full load");
}
