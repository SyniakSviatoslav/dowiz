//! `Staged` against the oracle it replaces: a decoded `Kv` with the same writes applied.
use super::*;
use crate::{Kv, View};
use alloc::string::ToString;
use alloc::vec;

fn image(entries: &[(&str, &str)]) -> Vec<u8> {
    let kv = Kv { entries: entries.iter().map(|(k, v)| (k.to_string(), v.as_bytes().to_vec())).collect() };
    kv.compacted_bytes(16 * 1024).unwrap()
}

fn oracle(entries: &[(&str, &str)], batch: &Batch) -> Kv {
    let mut kv = Kv { entries: entries.iter().map(|(k, v)| (k.to_string(), v.as_bytes().to_vec())).collect() };
    for op in batch.ops() {
        match op {
            crate::delta::KeyOp::Put(k, v) => kv.put(core::str::from_utf8(k).unwrap(), v),
            crate::delta::KeyOp::Remove(k) => {
                kv.remove(core::str::from_utf8(k).unwrap());
            }
        }
    }
    kv
}

const BASE: [(&str, &str); 4] = [("a", "1"), ("c", "3"), ("e", "5"), ("g", "7")];

fn batch() -> Batch {
    let mut b = Batch::default();
    b.put(b"b", b"2").unwrap();
    b.put(b"c", b"33").unwrap();
    b.remove(b"e").unwrap();
    b.remove(b"z").unwrap();
    b.put(b"h", b"8").unwrap();
    b
}

#[test]
fn reads_through_a_batch_answer_what_the_decoded_kv_with_the_writes_answers() {
    let bytes = image(&BASE);
    let base = KvIn::open(View::new(&bytes)).unwrap();
    let b = batch();
    let s = Staged::new(&base, &b);
    let want = oracle(&BASE, &b);
    let got: Vec<(Vec<u8>, Vec<u8>)> = s.entries().unwrap().into_iter().map(|(k, v)| (k.into_owned(), v.into_owned())).collect();
    let want_entries: Vec<(Vec<u8>, Vec<u8>)> = want.entries.iter().map(|(k, v)| (k.as_bytes().to_vec(), v.clone())).collect();
    assert_eq!(got, want_entries);
    assert_eq!(s.len().unwrap(), want.entries.len());
    assert_eq!(s.fold_root().unwrap(), want.snapshot_root_u64());
    assert_eq!(s.get(b"c").unwrap().map(Bytes::into_owned), Some(b"33".to_vec()));
    assert_eq!(s.get(b"e").unwrap(), None);
    assert_eq!(s.get(b"a").unwrap().map(Bytes::into_owned), Some(b"1".to_vec()));
    assert!(s.contains(b"h").unwrap());
    assert!(!s.contains(b"z").unwrap());
}

#[test]
fn a_v2_value_in_a_byte_image_is_read_in_place() {
    let bytes = image(&BASE);
    let base = KvIn::open(View::new(&bytes)).unwrap();
    let b = Batch::default();
    let s = Staged::new(&base, &b);
    assert!(s.get(b"g").unwrap().unwrap().is_borrowed());
}

#[test]
fn a_range_scan_honours_both_bounds_and_an_empty_range_is_empty() {
    let bytes = image(&BASE);
    let base = KvIn::open(View::new(&bytes)).unwrap();
    let b = batch();
    let s = Staged::new(&base, &b);
    let keys = |lo: Bound<&[u8]>, hi: Bound<&[u8]>| -> Vec<Vec<u8>> { s.cursor(lo, hi).unwrap().map(|e| e.unwrap().0.into_owned()).collect() };
    assert_eq!(keys(Bound::Included(&b"b"[..]), Bound::Excluded(&b"g"[..])), vec![b"b".to_vec(), b"c".to_vec()]);
    assert_eq!(keys(Bound::Excluded(&b"c"[..]), Bound::Unbounded), vec![b"g".to_vec(), b"h".to_vec()]);
    assert_eq!(keys(Bound::Included(&b"g"[..]), Bound::Included(&b"a"[..])), Vec::<Vec<u8>>::new());
}
