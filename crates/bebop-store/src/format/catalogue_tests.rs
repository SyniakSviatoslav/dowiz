//! T-ES-02: THE NAMED-CORRUPTED-CELL CATALOGUE (R-EMBED-STORE §4.1, AC-ES-02). One test per
//! named cell of a three-commit v3 image (`img3`: a compacted base, a put, a remove), each
//! asserting the exact refusal -- never a panic, never a silently different answer -- and,
//! for the superblock-level cells, that the borrowed reader (`View`) answers like `Store`;
//! for the KV-level cells, that the zero-copy reader (`KvIn`) refuses like `Kv::load_checked`.
//! No generator, no seed: operator rule, no fuzzers in this repo.

use super::tests::{img3, resb};
use super::*;
use crate::kv::zc::KvIn;
use crate::verify::KvError;
use crate::kv::Kv;
use crate::{pick_in, Store, StoreError, View};

/// `Store::pick_checked`, asserting the borrowed reader agrees on the same cells.
pub(super) fn pick(st: &Store) -> Result<Superblock, PickError> {
    let bytes = st.to_bytes();
    let r = st.pick_checked();
    assert_eq!(pick_in(&View::new(&bytes)), r.ok(), "View and Store disagree");
    // `Store::pick` is the wasm reader's own lean copy of the rule (its byte ratchet only
    // falls); every named cell here holds it to `pick_checked_in`'s answer.
    assert_eq!(st.pick(), r.ok(), "Store::pick and pick_checked disagree");
    r
}

/// `Kv::load_checked`, asserting the zero-copy reader refuses with the same error.
pub(super) fn kv(st: &Store) -> Result<Kv, KvError> {
    let bytes = st.to_bytes();
    let v = View::new(&bytes);
    let r = Kv::load_checked(st);
    assert_eq!(KvIn::open(&v).err(), r.as_ref().err().copied(), "KvIn and Kv::load_checked disagree");
    r
}

pub(super) fn live(st: &Store) -> usize {
    st.pick_checked().unwrap().at
}
pub(super) fn pt(st: &Store) -> usize {
    st.pick_checked().unwrap().root as usize
}
/// Set payload cell `i` of `obj` and RESEAL it, so the check under test is the one that answers.
pub(super) fn fix(st: &mut Store, obj: usize, i: usize, v: i64) {
    st.put_cell(obj, i, v);
    st.seal(obj);
}
pub(super) fn root(st: &Store) -> usize {
    st.root().unwrap()
}
pub(super) fn newest(st: &Store) -> usize {
    st.follow(root(st), 6).unwrap()
}

#[test]
fn c01_both_magics() {
    let mut st = img3();
    st.cells[SB_A] = 0;
    st.cells[SB_B] = 0;
    assert_eq!(pick(&st), Err(PickError::NoValidSuperblock));
    assert!(matches!(st.begin(), Err(StoreError::NoSuperblock)));
}

/// A bad-checksum superblock is not a superblock: the OTHER generation is read (the one
/// fallback the format allows), and it is the generation before the remove.
#[test]
fn c02_live_superblock_crc15() {
    let mut st = img3();
    let at = live(&st);
    st.cells[at + SB_CRC] += 1;
    assert_eq!(pick(&st).unwrap().generation, 2);
    assert!(kv(&st).unwrap().get("p:01").is_some(), "generation 2 still holds p:01");
}

#[test]
fn c03_newer_superblock_used_past_the_image() {
    let mut st = img3();
    let (at, n) = (live(&st), st.cells.len());
    st.cells[at + SB_USED] = n as i64 + 1;
    resb(&mut st, at);
    assert_eq!(pick(&st), Err(PickError::ArenaUsedOutside { at, used: n as i64 + 1, have: n }));
}

#[test]
fn c04_superblock_used_negative() {
    let mut st = img3();
    let at = live(&st);
    st.cells[at + SB_USED] = -1;
    resb(&mut st, at);
    assert!(matches!(pick(&st), Err(PickError::ArenaUsedOutside { used: -1, .. })));
}

#[test]
fn c05_superblock_parttab_ref_negative() {
    let mut st = img3();
    let at = live(&st);
    st.cells[at + SB_PARTTAB] = -5;
    resb(&mut st, at);
    assert_eq!(pick(&st), Err(PickError::PartTabOutside { at, pt: -5 }));
}

#[test]
fn c06_superblock_parttab_ref_past_the_end() {
    let mut st = img3();
    let (at, n) = (live(&st), st.cells.len() as i64);
    st.cells[at + SB_PARTTAB] = n - 3;
    resb(&mut st, at);
    assert_eq!(pick(&st), Err(PickError::PartTabOutside { at, pt: n - 3 }));
}

#[test]
fn c07_parttab_root_negative_raw() {
    let mut st = img3();
    let p = pt(&st);
    st.cells[p + PT_ROOT] = -1;
    assert!(matches!(pick(&st), Err(PickError::PartTabBad { got: Some(_), .. })));
}

#[test]
fn c08_parttab_root_negative_resealed() {
    let mut st = img3();
    let (at, p) = (live(&st), pt(&st));
    fix(&mut st, p, PT_ROOT - 2, -1);
    assert_eq!(pick(&st), Err(PickError::RootOutside { at, root: -1 }));
}

#[test]
fn c09_parttab_cursor_bit_raw() {
    let mut st = img3();
    let p = pt(&st);
    st.cells[p + PT_CURSOR] ^= 1;
    assert!(matches!(pick(&st), Err(PickError::PartTabBad { pt: q, got: Some(_), .. }) if q == p));
}

#[test]
fn c10_parttab_cursor_behind_used_resealed() {
    let mut st = img3();
    let (p, used) = (pt(&st), st.pick_checked().unwrap().arena_used);
    fix(&mut st, p, PT_CURSOR - 2, used - 20);
    assert!(pick(&st).is_ok(), "a sealed PartTab passes pick; begin is where a cursor becomes a write");
    assert!(matches!(st.begin(), Err(StoreError::Corrupt("the write cursor is behind the superblock's arena_used"))));
}

#[test]
fn c11_parttab_cursor_past_the_arena_resealed() {
    let mut st = img3();
    let (p, at) = (pt(&st), live(&st));
    let cap = st.cells[at + SB_CAPACITY];
    fix(&mut st, p, PT_CURSOR - 2, ARENA as i64 + cap + 1);
    assert!(matches!(st.begin(), Err(StoreError::Corrupt("the write cursor is outside the arena"))));
}

#[test]
fn c12_parttab_length_short_resealed() {
    let mut st = img3();
    let (at, p) = (live(&st), pt(&st));
    st.cells[p] = (st.cells[p] & !0xFFFF_FFFF) | 18;
    st.seal(p);
    assert!(matches!(pick(&st), Err(PickError::PartTabBad { at: a, got: None, .. }) if a == at));
}

/// W-F1: the live superblock names a whole, sealed PartTab -- but at the OTHER page's home
/// (the F1 image: live A naming 528). The next commit from A writes 528 in place, so a crash
/// before its superblock lands would refuse instead of reading generation k. Refused by name.
fn misplaced() -> (Store, usize, i64) {
    let mut st = img3();
    let (at, p) = (live(&st), pt(&st));
    let away = (SB_B - at) + PT_PAGE_OFFSET;
    for i in 0..PARTTAB_OBJ_CELLS {
        st.cells[away + i] = st.cells[p + i];
    }
    st.cells[at + SB_PARTTAB] = away as i64;
    resb(&mut st, at);
    (st, at, away as i64)
}

#[test]
fn c31_parttab_ref_moved_off_its_home_resealed() {
    let (st, at, away) = misplaced();
    assert_eq!(st.pick_checked(), Err(PickError::PartTabMisplaced { at, pt: away }));
    assert!(matches!(st.begin(), Err(StoreError::Image(PickError::PartTabMisplaced { .. }))));
}

#[test]
fn c32_parttab_ref_moved_store_pick() {
    assert_eq!(misplaced().0.pick(), None, "the wasm reader's lean copy");
}

#[test]
fn c33_parttab_ref_moved_view() {
    let bytes = misplaced().0.to_bytes();
    assert_eq!(View::new(&bytes).pick(), None);
    let (st, at, away) = misplaced();
    assert_eq!(pick(&st), Err(PickError::PartTabMisplaced { at, pt: away }));
}
