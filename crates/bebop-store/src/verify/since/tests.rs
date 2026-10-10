//! Tests for `since.rs` (W-HUBCRC): every case compares the marked scan against the FULL
//! `chain_scan` over the same bytes. One named corrupted cell per case; no fuzzing
//! (operator rule). Cells of a v2 record's payload: 2 = `next` ref, 3..6 = id,
//! 7..10 = `prev`, 12 = the first cell of the record's own bytes.
use super::*;
use crate::evlog::Record;

fn rec(n: u8, payload: &[u8]) -> Record {
    Record { id: [n; 32], prev: [n.wrapping_sub(1); 32], actor_pubkey: [0; 32], actor_seq: n as u64, payload: payload.to_vec() }
}

fn append(st: &mut Store, from: u8, to: u8) {
    for i in from..=to {
        EvLog::append_tip_bytes(st, &rec(i, b"{\"kind\":\"placed\",\"total\":1500}")).unwrap();
    }
}

/// A 6-record log, its mark, and the same log with records 7 and 8 appended after it --
/// what the DO holds after one turn: the mark of the bytes it verified, then its own append.
fn marked_then_appended() -> (Store, ChainMark) {
    let mut st = Store::create_bytes(64 * 1024).unwrap();
    EvLog::init_bytes(&mut st).unwrap();
    append(&mut st, 1, 6);
    let (_, mark) = EvLog::chain_scan_since(&st, None).unwrap();
    append(&mut st, 7, 8);
    (st, mark.expect("a non-empty log has a mark"))
}

/// The records newest first (cell indexes).
fn records(st: &Store) -> Vec<usize> {
    let mut out = Vec::new();
    let mut cur = st.follow(st.root().unwrap(), 1);
    while let Some(o) = cur {
        out.push(o);
        cur = st.follow(o, 2);
    }
    out
}

fn flip(st: &mut Store, obj: usize, i: usize) {
    st.cells[obj + 2 + i] ^= 0x100;
}

/// The marked scan's verdict and count, with the full scan's beside it.
fn both(st: &Store, mark: &ChainMark) -> (Result<Option<usize>, BadCrc>, Result<Option<usize>, BadCrc>) {
    let since = EvLog::chain_scan_since(st, Some(mark)).map(|(s, _)| s.chained);
    let full = EvLog::chain_scan(st).map(|s| s.chained);
    (since, full)
}

/// POSITIVE TWIN: a clean append after the mark reads exactly as the full scan does, and
/// the mark it hands back names the NEW newest record.
#[test]
fn a_clean_append_after_the_mark_reads_as_the_full_scan() {
    let (st, mark) = marked_then_appended();
    let (since, full) = both(&st, &mark);
    assert_eq!(since, Ok(Some(8)));
    assert_eq!(since, full);
    let (_, next) = EvLog::chain_scan_since(&st, Some(&mark)).unwrap();
    let next = next.unwrap();
    assert!(next.names(&st, records(&st)[0]) && next.below == 8, "the mark moves to the newest record");
    assert_eq!(EvLog::chain_scan_since(&st, Some(&next)).unwrap().0.chained, Some(8), "and holds on the same bytes");
}

/// A CHANGED BYTE IN A RECORD ADDED AFTER THE MARK (payload cell 12 of record 8) is
/// QUARANTINED by the marked scan exactly as by the full one: new records are always hashed.
#[test]
fn a_flipped_byte_in_a_record_added_after_the_mark_is_quarantined() {
    let (mut st, mark) = marked_then_appended();
    let newest = records(&st)[0];
    flip(&mut st, newest, 12);
    let (s, _) = EvLog::chain_scan_since(&st, Some(&mark)).unwrap();
    assert_eq!(s.chained, Some(8));
    assert_eq!(s.quarantined.iter().map(|b| b.obj).collect::<Vec<_>>(), vec![newest], "named");
    assert_eq!(s, EvLog::chain_scan(&st).unwrap(), "same as the full scan");
}

/// A CHANGED `prev` IN A NEW RECORD (payload cell 7 of record 8) is REFUSED, marked or not:
/// a changed ref cannot be told from a changed `prev` (`chain_scan`'s rule (b)).
#[test]
fn a_changed_prev_in_a_new_record_is_refused_with_or_without_the_mark() {
    let (mut st, mark) = marked_then_appended();
    let newest = records(&st)[0];
    flip(&mut st, newest, 7);
    let (since, full) = both(&st, &mark);
    assert_eq!(since.map_err(|b| b.obj), Err(newest));
    assert_eq!(since, full);
}

/// THE MARK'S OWN RECORD CHANGED -- its id (payload cell 3) or its crc header cell -- so the
/// mark does not name it: the scan hashes it and everything older, i.e. the full scan.
#[test]
fn a_changed_anchor_record_falls_back_to_the_full_scan() {
    for (what, cell) in [("id cell 3", Some(3usize)), ("crc header cell", None)] {
        let (mut st, mark) = marked_then_appended();
        let anchor = records(&st)[2]; // record 6: 8, 7, then the marked one
        match cell {
            Some(i) => flip(&mut st, anchor, i),
            None => st.cells[anchor + 1] ^= 1 << 40,
        }
        assert!(!mark.names(&st, anchor), "{what}: the mark no longer names its record");
        let s = EvLog::chain_scan_since(&st, Some(&mark));
        assert_eq!(s.as_ref().map(|r| &r.0), EvLog::chain_scan(&st).as_ref(), "{what}: same as the full scan");
    }
}

/// THE COUNT BELOW THE MARK MUST HOLD: record 2's `next` ref (payload cell 2) cut to null
/// ends the walk one record early. The mark's record is found, but 5 records sit at or
/// below it where it proved 6, so the scan falls back to the full one -- which hashes
/// record 2 and quarantines it, and reports the short chain the caller refuses.
#[test]
fn a_chain_cut_below_the_mark_falls_back_to_the_full_scan() {
    let (mut st, mark) = marked_then_appended();
    let second = records(&st)[6];
    st.cells[second + 2 + 2] = 0;
    let (s, _) = EvLog::chain_scan_since(&st, Some(&mark)).unwrap();
    assert_eq!(s.chained, Some(7), "the walk ends at record 2");
    assert_eq!(s.quarantined.iter().map(|b| b.obj).collect::<Vec<_>>(), vec![second], "record 2 was hashed again");
    assert_eq!(s, EvLog::chain_scan(&st).unwrap());
}

/// A MARK FROM ANOTHER IMAGE (a log laid out differently: longer records) names no record
/// here, so it changes nothing.
#[test]
fn a_mark_from_another_layout_is_ignored() {
    let (_, mark) = marked_then_appended();
    let mut other = Store::create_bytes(64 * 1024).unwrap();
    EvLog::init_bytes(&mut other).unwrap();
    for i in 1..=8 {
        EvLog::append_tip_bytes(&mut other, &rec(i, b"{\"kind\":\"placed\",\"total\":1500,\"note\":\"a longer record\"}")).unwrap();
    }
    let newest = records(&other)[0];
    flip(&mut other, newest, 12);
    let (s, _) = EvLog::chain_scan_since(&other, Some(&mark)).unwrap();
    assert_eq!(s, EvLog::chain_scan(&other).unwrap());
    assert_eq!(s.quarantined.len(), 1);
}

/// THE LIMIT, STATED (module doc): a changed byte BELOW the mark's record (payload cell 12
/// of record 2) is not hashed again. The full scan quarantines it; the marked one does
/// not see it. This is why only a holder that never let the bytes leave its memory may
/// keep a mark, and why every cold load passes none.
#[test]
fn a_mark_does_not_rehash_below_its_record() {
    let (mut st, mark) = marked_then_appended();
    let second = records(&st)[6];
    flip(&mut st, second, 12);
    let full = EvLog::chain_scan(&st).unwrap();
    assert_eq!(full.quarantined.iter().map(|b| b.obj).collect::<Vec<_>>(), vec![second], "a cold load sees it");
    let (marked, _) = EvLog::chain_scan_since(&st, Some(&mark)).unwrap();
    assert_eq!((marked.chained, marked.quarantined.len()), (Some(8), 0), "a marked load does not");
}

/// An empty log has no mark, and scanning it with one from elsewhere is the empty answer.
#[test]
fn an_empty_log_has_no_mark() {
    let mut st = Store::create_bytes(64 * 1024).unwrap();
    EvLog::init_bytes(&mut st).unwrap();
    let (s, m) = EvLog::chain_scan_since(&st, None).unwrap();
    assert_eq!((s.chained, m), (Some(0), None));
    let (_, mark) = marked_then_appended();
    assert_eq!(EvLog::chain_scan_since(&st, Some(&mark)).unwrap().0.chained, Some(0));
}
