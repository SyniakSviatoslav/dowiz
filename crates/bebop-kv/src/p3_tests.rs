//! W-ES-P3: the byte-key writers write what the `&str` ones always wrote.
use super::*;
use crate::delta::{append_delta, append_delta_ops, KeyOp, Op, MAX_DELTAS, DEAD_DIV};
use crate::zc::KvIn;

fn kv(n: usize) -> Kv {
    Kv { entries: (0..n).map(|i| (format!("k{i:03}"), format!("value {i}").into_bytes())).collect() }
}

#[test]
fn append_delta_ops_writes_the_same_bytes_as_append_delta() {
    let base = kv(12).compacted_bytes(64 * 1024).unwrap();
    let (mut a, mut b) = (Store::from_bytes(&base), Store::from_bytes(&base));
    let ops = [Op::Put("k003", b"new"), Op::Remove("k007"), Op::Put("zz", b"tail")];
    let key_ops: Vec<KeyOp> = ops.iter().map(|&o| KeyOp::from(o)).collect();
    let ra = append_delta(&mut a, &ops).unwrap();
    let rb = append_delta_ops(&mut b, &key_ops, MAX_DELTAS, DEAD_DIV).unwrap();
    assert_eq!(ra, rb);
    assert_eq!(a.to_bytes_trimmed(), b.to_bytes_trimmed());
}

#[test]
fn compacted_from_with_no_generation_is_kv_compacted_bytes() {
    let k = kv(30);
    let via_fn = compacted_from(k.entries.iter().map(|(a, b)| (a.as_bytes(), b.as_slice())), 64 * 1024, None).unwrap();
    assert_eq!(via_fn, k.compacted_bytes(64 * 1024).unwrap());
}

#[test]
fn a_compaction_after_generation_g_is_generation_g_plus_1_and_reads_the_same() {
    let k = kv(30);
    let entries = || k.entries.iter().map(|(a, b)| (a.as_bytes(), b.as_slice()));
    let plain = compacted_from_fit(entries(), 1 << 20, None).unwrap();
    let later = compacted_from_fit(entries(), 1 << 20, Some(41)).unwrap();
    assert_eq!(Store::from_bytes(&plain).pick().unwrap().generation, 1);
    assert_eq!(Store::from_bytes(&later).pick().unwrap().generation, 42);
    let fold = |b: &[u8]| KvIn::open(View::new(b)).unwrap().snapshot_root_u64().unwrap();
    assert_eq!(fold(&plain), fold(&later));
    assert_eq!(plain.len(), later.len());
}

#[test]
fn a_compaction_past_the_generation_limit_is_refused_by_name() {
    let k = kv(1);
    let e = compacted_from(k.entries.iter().map(|(a, b)| (a.as_bytes(), b.as_slice())), 16 * 1024, Some(MAX_GENERATION)).unwrap_err();
    assert!(matches!(e, StoreError::Limit { what: "generation", .. }), "{e:?}");
}
