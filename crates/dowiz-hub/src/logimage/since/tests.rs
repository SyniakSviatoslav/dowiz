//! W-HUBCRC3: the mark (W-HUBCRC) and the carried scan (`crate::Seen`, W-LOOPB) are ONE
//! walk, so they must agree. A marked scan hashes only the records newer than its mark, so
//! a mark over a quarantined record would make the next load forget that record and SERVE
//! it. `quarantine::chain_is_whole_since` therefore hands out a mark only for a clean log.
//! One named corrupted cell per case (payload cell 12 = the record's own bytes); no fuzzing.
use crate::logimage::LogImage;
use crate::{EventKind, Hub};
use bebop_store::Store;

fn hub_bytes(n: u64) -> Vec<u8> {
    let mut hub = Hub::create_sized(64 * 1024).unwrap();
    for i in 0..n {
        hub.append(EventKind::Placed, &format!("ord-{i}"), r#"{"status":"new","note":"xxxxxxxxxxxxxxxxxxxx"}"#, i, [0u8; 32]).unwrap();
    }
    hub.to_bytes_trimmed()
}

/// Payload cell 12 of the `at`-th record of a newest-first walk, one byte flipped, not re-sealed.
fn flipped(bytes: &[u8], at: usize) -> Vec<u8> {
    let mut st = Store::from_bytes(bytes);
    let mut o = st.follow(st.root().unwrap(), 1).unwrap();
    for _ in 0..at {
        o = st.follow(o, 2).unwrap();
    }
    st.cells[o + 2 + 12] ^= 0x100;
    st.to_bytes()
}

/// POSITIVE TWIN: a clean log is marked, and the marked reload after an append carries the
/// same `Seen` as a cold load of the same bytes.
#[test]
fn a_clean_log_is_marked_and_its_marked_seen_is_the_cold_seen() {
    let (mut hub, mark) = Hub::load_since(&hub_bytes(5), None).unwrap();
    assert!(mark.is_some(), "a clean log has a mark");
    hub.append(EventKind::Placed, "ord-new", r#"{"status":"new"}"#, 9, [0u8; 32]).unwrap();
    let bytes = hub.to_bytes_trimmed();
    let (marked, next) = Hub::load_since(&bytes, mark.as_ref()).unwrap();
    let cold = Hub::load(&bytes).unwrap();
    assert_eq!(marked.seen, cold.seen);
    assert_eq!(marked.events(), cold.events());
    assert!(next.is_some());
}

/// THE MERGE'S NAMED CELL: payload cell 12 of record 2 of 5 (the middle), changed. The full
/// load quarantines it and hands back NO mark; after an append the reload (with whatever
/// mark the first load gave) still leaves that record out of `events()` and names it, as a
/// cold load does. A mark over it would skip its hash and serve its changed bytes.
#[test]
fn an_old_quarantined_record_is_never_marked_and_stays_out_after_a_reload() {
    let (mut hub, mark) = Hub::load_since(&flipped(&hub_bytes(5), 2), None).unwrap();
    hub.append(EventKind::Placed, "ord-new", r#"{"status":"new"}"#, 9, [0u8; 32]).unwrap();
    let bytes = hub.to_bytes_trimmed();
    let (again, next) = Hub::load_since(&bytes, mark.as_ref()).unwrap();
    let cold = Hub::load(&bytes).unwrap();
    assert_eq!((again.len(), again.events().len()), (6, 5), "the changed record is not served");
    assert_eq!(again.seen, cold.seen, "the bad set is the cold load's");
    assert_eq!(again.quarantined().len(), 1);
    assert_eq!((mark, next), (None, None), "a quarantined log is never marked");
}

/// The same for an entry log (the ledger, the till): `recent_checked` reads the carried
/// verdicts, so it must still refuse on the old bad record after a marked reload.
#[test]
fn an_entry_log_with_an_old_quarantined_record_is_never_marked() {
    let mut log = LogImage::create_sized(64 * 1024).unwrap();
    for i in 0..4 {
        log.append("tx", &format!("s{i}"), r#"{"msg":"a message long enough"}"#).unwrap();
    }
    let clean = log.to_bytes();
    assert!(LogImage::load_since(&clean, None).unwrap().1.is_some(), "positive twin: a clean ledger is marked");
    let (mut log, mark) = LogImage::load_since(&flipped(&clean, 1), None).unwrap();
    log.append("tx", "s4", r#"{"msg":"the turn's own record"}"#).unwrap();
    let bytes = log.to_bytes();
    let (again, next) = LogImage::load_since(&bytes, mark.as_ref()).unwrap();
    assert_eq!((mark, next), (None, None));
    assert_eq!(again.seen, LogImage::load(&bytes).unwrap().seen);
    assert!(again.recent_checked(10).is_err(), "the console still sees the quarantined record");
}
