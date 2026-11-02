//! `last_commit_in` (W-ES-P10, FORMAT §7 phase P2): the one rule every reader of bytes applies
//! to the last commit, on images the real writer (`Store::commit_bytes`, anchored) made.
//!
//! RED MUTANTS (applied by hand, run, reverted; the hand-back quotes the runs):
//!   M10a  `last_commit_in` returns `Ok(a)` for `Torn` too (no refusal) -> `a_torn_range_names_
//!         the_object_that_fails` and `_anchor_when_every_object_checks` both go red.
//!   M10b  the walk skips `check_obj_in` -> the first test gets `Anchor`, not `Object`.

use super::*;
use crate::format::{anchor_of, without_anchor, SB_ANC, SB_CRC, SB_USED};
use crate::Store;

/// Two commits: the first a 3-cell object, the second (the LAST) a 5-cell object whose
/// payload is `100..105`. Returns the store and the last commit's object.
fn two_commits() -> (Store, usize) {
    let mut st = Store::create_bytes(1 << 16).unwrap();
    let mut tx = st.begin().unwrap();
    let a = st.alloc(&mut tx, 3, 0x11).unwrap();
    for i in 0..3 {
        st.put_cell(a, i, 7 + i as i64);
    }
    st.seal(a);
    st.commit_bytes(&tx, a);
    let mut tx = st.begin().unwrap();
    let b = st.alloc(&mut tx, 5, 0x22).unwrap();
    for i in 0..5 {
        st.put_cell(b, i, 100 + i as i64);
    }
    st.seal(b);
    st.commit_bytes(&tx, b);
    (st, b)
}

#[test]
fn a_whole_image_holds_and_an_unanchored_one_is_absent() {
    let (st, b) = two_commits();
    let sb = crate::format::pick_checked_in(&st).unwrap();
    let a = last_commit_in(&st, &sb).unwrap();
    assert!(matches!(a, Anchor::Holds { mark, used } if mark == b && used == sb.arena_used as usize), "{a:?}");
    let old = Store::from_bytes(&without_anchor(&st.to_bytes_trimmed()));
    let sb = crate::format::pick_checked_in(&old).unwrap();
    assert_eq!(last_commit_in(&old, &sb), Ok(Anchor::Absent));
}

#[test]
fn a_torn_range_names_the_object_that_fails() {
    let (mut st, b) = two_commits();
    st.cells[b + 2 + 3] ^= 1; // one bit of the last commit's payload; its crc was not re-sealed
    let sb = crate::format::pick_checked_in(&st).unwrap();
    let e = last_commit_in(&st, &sb).unwrap_err();
    assert!(matches!(e, LastCommit::Object(BadCrc { obj, got: Some(_), .. }) if obj == b), "{e:?}");
    // `check_obj_in` is `Store::check_obj`, over `Cells`
    assert_eq!(check_obj_in(&st, b), st.check_obj(b));
    assert_eq!(check_obj_in(&st, b - 5), st.check_obj(b - 5));
}

#[test]
fn a_torn_range_whose_objects_all_check_is_the_anchor() {
    let (mut st, b) = two_commits();
    st.cells[b + 2 + 3] ^= 1;
    st.seal(b); // the object is whole again by its own crc; only the anchor still disagrees
    let sb = crate::format::pick_checked_in(&st).unwrap();
    let e = last_commit_in(&st, &sb).unwrap_err();
    assert!(matches!(e, LastCommit::Anchor(Anchor::Torn { mark, got: Some(_), .. }) if mark == b as i64), "{e:?}");
    // re-anchoring over the changed bytes (what a WRITER does) makes it hold again
    let mark = st.cells[sb.at + SB_ANC] >> 32;
    let used = st.cells[sb.at + SB_USED];
    st.cells[sb.at + SB_ANC] = anchor_of(&st, mark, used);
    st.cells[sb.at + SB_CRC] = crate::crc32_cells(&st.cells, sb.at, 15) as i64;
    let sb = crate::format::pick_checked_in(&st).unwrap();
    assert!(matches!(last_commit_in(&st, &sb), Ok(Anchor::Holds { .. })));
}

#[test]
fn an_older_commit_is_outside_the_rule() {
    // The documented half: a flipped bit in the FIRST commit's object is not the anchor's business.
    let (mut st, b) = two_commits();
    let a = b - 5; // the 3-cell object: 2 header + 3 payload cells before `b`
    assert_eq!(st.obj_len(a), 3);
    st.cells[a + 2] ^= 1;
    let sb = crate::format::pick_checked_in(&st).unwrap();
    assert!(matches!(last_commit_in(&st, &sb), Ok(Anchor::Holds { .. })));
    assert!(st.check_obj(a).is_err(), "the older object is still corrupt; verify(Reachable/WholeArena) is what finds it");
}
