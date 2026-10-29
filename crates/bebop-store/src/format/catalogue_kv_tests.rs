//! T-ES-02, the KV-level half of the named-corrupted-cell catalogue (`catalogue_tests.rs` holds
//! the superblock/PartTab half and the shared helpers; split at the 300-line ratchet).

use super::catalogue_tests::*;
use super::tests::{img3, resb};
use super::*;
use crate::verify::KvError;
use crate::{Store, StoreError, MIN_BYTES};

#[test]
fn c13_kidx_length_claim_bit_31() {
    let mut st = img3();
    let kidx = st.follow(root(&st), 1).unwrap();
    st.cells[kidx] |= 1 << 31;
    assert!(matches!(kv(&st), Err(KvError::BadCrc(b)) if b.obj == kidx && b.got.is_none()));
}

#[test]
fn c14_vblob_crc_plus_one() {
    let mut st = img3();
    let vblob = st.follow(root(&st), 4).unwrap();
    st.cells[vblob + 1] = st.cells[vblob + 1].wrapping_add(1 << 32);
    assert!(matches!(kv(&st), Err(KvError::BadCrc(b)) if b.obj == vblob && b.got.is_some()));
}

/// The older record's NEXT turned back onto the newest: a loop. Bounded by D and refused.
#[test]
fn c15_delta_next_ref_backwards_loop() {
    let mut st = img3();
    let (new, old) = (newest(&st), st.follow(newest(&st), 0).unwrap());
    fix(&mut st, old, 0, new as i64 - old as i64);
    assert_eq!(kv(&st).err(), Some(KvError::NotKv));
}

#[test]
fn c16_root_delta_ref_wild() {
    let mut st = img3();
    let r = root(&st);
    fix(&mut st, r, 6, i64::MAX - 3);
    assert_eq!(kv(&st).err(), Some(KvError::NotKv));
}

#[test]
fn c17_kidx_offset_past_its_blob() {
    let mut st = img3();
    let kidx = st.follow(root(&st), 1).unwrap();
    fix(&mut st, kidx, 0, 1 << 40);
    assert_eq!(kv(&st).err(), Some(KvError::NotKv));
}

#[test]
fn c18_root_version_4() {
    let mut st = img3();
    let r = root(&st);
    fix(&mut st, r, 5, 4);
    // W-ES-P8A: v4 (the B+tree) is refused BY NAME, before cells 1-4 are read as v1-v3 arrays.
    assert_eq!(kv(&st).err(), Some(KvError::UnknownVersion { v: 4 }));
}

#[test]
fn c19_root_chain_count_plus_one() {
    let mut st = img3();
    let r = root(&st);
    fix(&mut st, r, 7, 3);
    assert_eq!(kv(&st).err(), Some(KvError::NotKv));
}

#[test]
fn c20_root_chain_count_minus_one() {
    let mut st = img3();
    let r = root(&st);
    fix(&mut st, r, 7, 1);
    assert_eq!(kv(&st).err(), Some(KvError::NotKv));
}

#[test]
fn c21_record_op_3() {
    let mut st = img3();
    let n = newest(&st);
    fix(&mut st, n, 1, 3);
    assert_eq!(kv(&st).err(), Some(KvError::NotKv));
}

#[test]
fn c22_remove_record_with_a_value_length() {
    let mut st = img3();
    let n = newest(&st);
    assert_eq!(st.get(n, 1), crate::kv::delta::OP_REMOVE);
    fix(&mut st, n, 3, 1);
    assert_eq!(kv(&st).err(), Some(KvError::NotKv));
}

#[test]
fn c23_empty_image() {
    let st = Store::from_bytes(&[]);
    assert_eq!(pick(&st), Err(PickError::NoValidSuperblock));
    assert_eq!(kv(&st).err(), Some(KvError::NotKv));
}

#[test]
fn c24_eight_bytes() {
    let st = Store::from_bytes(&crate::MAGIC.to_le_bytes());
    assert_eq!(pick(&st), Err(PickError::NoValidSuperblock));
}

/// The positive twin of every cut: a fresh store one cell short of `MIN_BYTES` is TRIMMED
/// (its arena is empty), so it reads as the empty store it is.
#[test]
fn c25_fresh_store_one_cell_below_the_floor() {
    let st = Store::create_bytes(MIN_BYTES).unwrap();
    let back = Store::from_bytes(&st.to_bytes()[..MIN_BYTES - 8]);
    assert_eq!(pick(&back).unwrap().generation, 0);
    assert_eq!(back.cells, st.cells, "padded back to its capacity");
}

#[test]
fn c26_cut_one_cell_below_arena_used() {
    let st = img3();
    let used = st.pick_checked().unwrap().arena_used as usize;
    let cut = Store::from_bytes(&st.to_bytes_trimmed()[..(used - 1) * 8]);
    assert!(matches!(pick(&cut), Err(PickError::ArenaUsedOutside { have, .. }) if have == used - 1));
}

#[test]
fn c27_cut_half_a_cell_below_arena_used() {
    let st = img3();
    let b = st.to_bytes_trimmed();
    let cut = Store::from_bytes(&b[..b.len() - 4]);
    assert!(matches!(pick(&cut), Err(PickError::ArenaUsedOutside { .. })));
}

/// An absurd capacity is not padded to, and the bytes that ARE there still read.
#[test]
fn c28_capacity_absurd() {
    let mut st = img3();
    let at = live(&st);
    st.cells[at + SB_CAPACITY] = (crate::MAX_PAD_CELLS as i64) * 4;
    resb(&mut st, at);
    let used = st.pick_checked().unwrap().arena_used as usize;
    let back = Store::from_bytes(&st.to_bytes()[..used * 8]);
    assert_eq!(back.cells.len(), used, "not padded");
    assert_eq!(kv(&back).unwrap().entries, kv(&img3()).unwrap().entries);
}

#[test]
fn c29_capacity_negative() {
    let mut st = img3();
    let at = live(&st);
    st.cells[at + SB_CAPACITY] = -1;
    resb(&mut st, at);
    assert!(matches!(st.begin(), Err(StoreError::Corrupt("the write cursor is outside the arena"))));
}

/// The anchor is inside the superblock's crc: changing it alone is a bad superblock.
#[test]
fn c30_anchor_cell_changed_without_its_crc() {
    let mut st = img3();
    let at = live(&st);
    st.cells[at + SB_ANC] ^= 1;
    assert_eq!(pick(&st).unwrap().generation, 2);
}
