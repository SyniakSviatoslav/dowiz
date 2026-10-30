//! The v4 constants, the writer's refusals (each with its positive twin), migration from the three
//! committed v1/v2/v3 fixtures, the golden bytes of one small tree, and the zero-copy read.

use alloc::vec::Vec;
use alloc::{format, vec};

use super::seq_tests::{commit, k, put, v};
use super::*;
use crate::delta::KeyOp;
use crate::{Kv, Store, View};

/// SHA-256 (FIPS 180-4), for deriving `st_digest` here: bebop-kv has no sha256 and takes no deps.
fn sha256(data: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3, 0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
    let mut h: [u32; 8] = [0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19];
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    for blk in msg.chunks(64) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([blk[4 * i], blk[4 * i + 1], blk[4 * i + 2], blk[4 * i + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }
        let mut s = h;
        for i in 0..64 {
            let t1 = s[7].wrapping_add(s[4].rotate_right(6) ^ s[4].rotate_right(11) ^ s[4].rotate_right(25)).wrapping_add((s[4] & s[5]) ^ (!s[4] & s[6])).wrapping_add(K[i]).wrapping_add(w[i]);
            let t2 = (s[0].rotate_right(2) ^ s[0].rotate_right(13) ^ s[0].rotate_right(22)).wrapping_add((s[0] & s[1]) ^ (s[0] & s[2]) ^ (s[1] & s[2]));
            s = [t1.wrapping_add(t2), s[0], s[1], s[2], s[3].wrapping_add(t1), s[4], s[5], s[6]];
        }
        for i in 0..8 {
            h[i] = h[i].wrapping_add(s[i]);
        }
    }
    let mut out = [0u8; 32];
    for i in 0..8 {
        out[4 * i..4 * i + 4].copy_from_slice(&h[i].to_be_bytes());
    }
    out
}

fn st_digest(s: &str) -> i64 {
    let h = sha256(s.as_bytes());
    u32::from_be_bytes([h[28], h[29], h[30], h[31]]) as i64
}

#[test]
fn digests_are_st_digest_of_their_layouts() {
    // The control: the same function gives the two digests the format already pins.
    assert_eq!(st_digest("KV{i64,ref [i64],ref [i64],ref [i64],ref [i64]}"), crate::DIGEST_KV_ROOT);
    assert_eq!(st_digest("arr i64"), crate::DIGEST_ARR_I64);
    assert_eq!(
        (DIGEST_KV4_LEAF, DIGEST_KV4_NODE, DIGEST_KV4_VAL),
        (st_digest(LAYOUT_KV4_LEAF), st_digest(LAYOUT_KV4_NODE), st_digest(LAYOUT_KV4_VAL))
    );
}

fn fresh() -> Store {
    Store::create_bytes(1 << 20).unwrap()
}

#[test]
fn unsorted_ops_are_refused_and_the_image_is_unchanged() {
    let mut st = fresh();
    let before = st.cells.clone();
    let (a, b) = (k(2), k(1));
    let r = stage_tree(&mut st, &[KeyOp::Put(&a, b"x"), KeyOp::Put(&b, b"y")], FANOUT);
    assert!(matches!(r, Err(WriteError::Unsorted)), "{r:?}");
    let dup = stage_tree(&mut st, &[KeyOp::Put(&a, b"x"), KeyOp::Remove(&a)], FANOUT);
    assert!(matches!(dup, Err(WriteError::Unsorted)), "{dup:?}");
    assert_eq!(st.cells, before);
    // Twin: the same two keys in order commit.
    commit_tree_bytes(&mut st, &[KeyOp::Put(&b, b"y"), KeyOp::Put(&a, b"x")], FANOUT).unwrap();
    assert_eq!(TreeIn::open(&st).unwrap().len(), 2);
}

#[test]
fn a_key_past_the_limit_and_a_bad_fanout_are_refused() {
    let mut st = fresh();
    let big = vec![b'k'; crate::MAX_KEY_BYTES + 1];
    assert!(matches!(stage_tree(&mut st, &[KeyOp::Put(&big, b"")], FANOUT), Err(WriteError::Store(_))));
    assert!(matches!(stage_tree(&mut st, &[], 3), Err(WriteError::Fanout(3))));
    // Twins: a key AT the limit, fanout 4.
    commit_tree_bytes(&mut st, &[KeyOp::Put(&big[1..], b"v")], MIN_FANOUT).unwrap();
    assert_eq!(TreeIn::open(&st).unwrap().get(&big[1..]).unwrap().as_deref(), Some(&b"v"[..]));
}

#[test]
fn a_v2_image_is_not_a_tree_and_its_writer_refuses() {
    let mut kv = Kv { entries: Vec::new() };
    kv.put("a", b"1");
    let mut st = Store::from_bytes(&kv.compacted_bytes(1 << 16).unwrap());
    assert_eq!(TreeIn::open(&st).err(), Some(TreeError::NotTree));
    assert!(matches!(stage_tree(&mut st, &[KeyOp::Put(b"b", b"2")], FANOUT), Err(WriteError::NotTree { v: 2 })));
    // Twin: its migration IS a tree with the same entries.
    let t4 = Store::from_bytes(&migrate_to_tree(&st, 1 << 20, FANOUT).unwrap());
    assert_eq!(TreeIn::open(&t4).unwrap().get(b"a").unwrap().as_deref(), Some(&b"1"[..]));
}

/// THE MIGRATION LAW on the committed fixtures of every older version: same entries, same fold.
#[test]
fn v1_v2_v3_fixtures_migrate_with_the_same_fold() {
    let fixtures: [&[u8]; 3] = [include_bytes!("../../../bebop-wasm/fixtures/kv.store"), include_bytes!("../../../bebop-wasm/fixtures/kv2.store"), include_bytes!("../../../bebop-wasm/fixtures/kv3.store")];
    for (i, bytes) in fixtures.into_iter().enumerate() {
        let src = Store::from_bytes(bytes);
        assert_eq!(Kv::version(&src), i as i64 + 1);
        let kv = Kv::load_checked(&src).unwrap();
        for b in [MIN_FANOUT, FANOUT] {
            let img = migrate_to_tree(&src, 64 << 20, b).unwrap();
            let t = TreeIn::open(View::new(&img)).unwrap();
            t.verify().unwrap();
            assert_eq!(t.fold().unwrap(), kv.snapshot_root_u64(), "fixture v{} b={b}", i + 1);
            let want: Vec<(Vec<u8>, Vec<u8>)> = kv.entries.iter().map(|(a, b)| (a.clone().into_bytes(), b.clone())).collect();
            assert_eq!(t.entries().unwrap(), want);
        }
    }
}

/// A get over a byte image BORROWS the value and the scan borrows keys and values (AC-ES-51).
#[test]
fn reads_over_bytes_are_zero_copy() {
    let mut st = fresh();
    commit(&mut st, &(0..100).map(|i| put(i, 0)).collect(), FANOUT);
    let img = st.to_bytes_trimmed();
    let t = TreeIn::open(View::new(&img)).unwrap();
    let got = t.get(&k(42)).unwrap().unwrap();
    assert!(got.is_borrowed());
    assert_eq!(&*got, &v(42, 0)[..]);
    assert!(t.iter().all(|e| e.map(|(a, b)| a.is_borrowed() && b.is_borrowed()).unwrap()));
}

/// FNV-1a of the trimmed image of one small tree, PINNED: the layout is a format, so any change
/// to the bytes the writer produces must be a deliberate re-pin (design P8b "golden bytes").
#[test]
fn golden_bytes_of_one_small_tree() {
    let mut st = fresh();
    commit(&mut st, &(0..9).map(|i| put(i, 0)).collect(), MIN_FANOUT);
    commit(&mut st, &vec![put(3, 1), super::seq_tests::del(7)], MIN_FANOUT);
    let t = TreeIn::open(&st).unwrap();
    assert_eq!((t.len(), t.depth()), (8, 2));
    let fnv = st.to_bytes_trimmed().iter().fold(crate::FNV_OFFSET, |h, &b| (h ^ b as u64).wrapping_mul(crate::FNV_PRIME));
    assert_eq!(format!("{fnv:016x}"), GOLDEN);
}
const GOLDEN: &str = "0372514c9e93342e";

/// The pieces hold no more than their items. `split_off` left each head its whole capacity: n^2/64
/// items' worth at n entries, 1.2 GB at 5e4, and a 1e5 build took the box down three times.
#[test]
fn split_even_retains_only_its_items() {
    let n = 10_000;
    let items: Vec<write::Item> = (0..n).map(|i| (Vec::new(), i)).collect();
    let pieces = write::split_even(items, 32);
    assert_eq!(pieces.len(), n.div_ceil(32));
    assert!(pieces.iter().all(|p| p.len().abs_diff(n / pieces.len()) <= 1));
    assert!(pieces.iter().flatten().map(|it| it.1).eq(0..n), "the order survives");
    let cap: usize = pieces.iter().map(Vec::capacity).sum();
    assert!(cap <= 2 * n, "capacity {cap} for {n} items");
}
