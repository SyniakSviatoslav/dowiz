//! W-KVPERF (2026-10-28): the v3 merged order is placed by one binary search per overlay key,
//! not rebuilt over the whole base on every `open`/`reopen`. Two things must hold: every
//! reader answers exactly as before (the OLD two-pointer merge is kept here as the oracle),
//! and a `reopen` + `get` reads O(D log n) cells, not O(n).
use super::*;
use crate::delta::{append_delta_with, Appended, Op};
use crate::zc::KvIn;
use crate::Kv;
use crate::{Store, View};

/// The merge `finish` did before W-KVPERF, verbatim: the oracle for `slot(i)`.
fn old_merge<C: Image>(kv: &KvIn<C>) -> Vec<Slot> {
    let mut out = Vec::with_capacity(kv.n + kv.ov.len());
    let (mut i, mut j) = (0usize, 0usize);
    while i < kv.n || j < kv.ov.len() {
        let ord = if i == kv.n {
            Ordering::Greater
        } else if j == kv.ov.len() {
            Ordering::Less
        } else {
            kv.cmp_key(i, &kv.ov[j].0).unwrap()
        };
        let put = j < kv.ov.len() && kv.ov[j].1.op == delta::OP_PUT;
        match ord {
            Ordering::Less => {
                out.push(Slot::Base(i));
                i += 1;
            }
            Ordering::Greater => {
                if put {
                    out.push(Slot::Delta(j));
                }
                j += 1;
            }
            Ordering::Equal => {
                if put {
                    out.push(Slot::Delta(j));
                }
                i += 1;
                j += 1;
            }
        }
    }
    out
}

fn base_kv(n: usize, vlen: usize) -> Kv {
    let mut kv = Kv { entries: Vec::new() };
    for i in 0..n {
        kv.entries.push((format!("k{:07}", 2 * i + 1), vec![(i % 251) as u8; vlen]));
    }
    kv
}

/// Append `batches` one generation each; mirror into `model`. Every batch must append.
fn v3(base: &Kv, batches: &[Vec<Op>], model: &mut Kv) -> Vec<u8> {
    let mut st = Store::from_bytes(&base.compacted_bytes_fit(1 << 30).unwrap());
    for ops in batches {
        for op in ops {
            match op {
                Op::Put(k, v) => model.put(k, v),
                Op::Remove(k) => {
                    model.remove(k);
                }
            }
        }
        let a = append_delta_with(&mut st, ops, usize::MAX, 0).unwrap();
        assert!(matches!(a, Appended::Delta(_)), "{a:?}");
    }
    st.to_bytes_trimmed()
}

/// put (new key before the first, between two, after the last), overwrite, tombstone (first,
/// middle, last, absent), and a re-put of a key removed one generation earlier.
fn edits() -> Vec<Vec<Op<'static>>> {
    vec![
        vec![Op::Remove("k0000005"), Op::Put("k0000000", b"before-first"), Op::Remove("k0000001")],
        vec![
            Op::Put("k0000004", b"between"),
            Op::Put("k0000003", b"overwritten"),
            Op::Remove("k0000039"),
            Op::Remove("k0000002"),
            Op::Put("k9999999", b"after-last"),
            Op::Put("k0000005", b"re-put"),
        ],
    ]
}

#[test]
fn v3_get_len_key_prefix_agree_through_every_reader() {
    let base = base_kv(20, 9);
    let mut model = Kv { entries: base.entries.clone() };
    let b = v3(&base, &edits(), &mut model);
    let loaded = Kv::load_checked(&Store::from_bytes(&b)).unwrap();
    assert_eq!(loaded.entries, model.entries, "load_checked");
    let v = View::new(&b);
    let open = KvIn::open(&v).unwrap();
    let reopen = KvIn::reopen(&v, open.checked()).unwrap();
    for kv in [&open, &reopen] {
        assert_eq!((0..kv.len()).map(|i| kv.slot(i).unwrap()).collect::<Vec<_>>(), old_merge(kv), "slot vs the old merge");
        assert!(kv.slot(kv.len()).is_err(), "one past the end");
        assert_eq!(kv.len(), model.entries.len(), "len");
        for (i, (k, val)) in model.entries.iter().enumerate() {
            assert_eq!(&*kv.key(i).unwrap(), k.as_bytes(), "key {i}");
            assert_eq!(&*kv.value(i).unwrap(), &val[..], "value {i}");
        }
        let mut probes: Vec<String> = (0..45).map(|i| format!("k{i:07}")).collect();
        probes.extend(["".into(), "k".into(), "k9999999".into(), "z".into()]);
        for p in &probes {
            let want = loaded.get(p);
            assert_eq!(kv.get(p.as_bytes()).unwrap().map(|c| c.into_owned()), want, "get {p}");
        }
        for pre in ["", "k", "k000000", "k000001", "k0000005", "k9", "z"] {
            let want: Vec<usize> = (0..model.entries.len()).filter(|&i| model.entries[i].0.starts_with(pre)).collect();
            assert_eq!(kv.prefix_range(pre.as_bytes()).unwrap(), want, "prefix {pre:?}");
        }
        assert_eq!(kv.snapshot_root_u64().unwrap(), model.snapshot_root_u64(), "fold");
    }
    assert_eq!(reopen.get(b"k0000005").unwrap().as_deref(), Some(&b"re-put"[..]), "the re-put wins");
    assert_eq!(reopen.get(b"k0000039").unwrap(), None, "the tombstone answers absent");
}

/// THE DEFECT, MEASURED IN CELLS: `reopen` + `get` on a 4000-key v3 image with one record
/// read every base key before (the merge); now ~log2(4000) keys per overlay key.
#[test]
fn v3_reopen_and_get_read_log_n_cells_not_n() {
    let base = base_kv(4000, 16);
    let mut model = Kv { entries: base.entries.clone() };
    let b = v3(&base, &[vec![Op::Put("k0001000", b"new")]], &mut model);
    let v = View::new(&b);
    let ck = KvIn::open(&v).unwrap().checked();
    let before = v.reads();
    let r = KvIn::reopen(&v, ck).unwrap();
    assert_eq!(r.get(b"k0003999").unwrap().as_deref(), Some(&[(1999 % 251) as u8; 16][..]));
    let reads = v.reads() - before;
    println!("v3 reopen + get: {reads} cells read for 4000 keys");
    assert!(reads < 1000, "{reads} cells read: the merge walked the base");
}

#[test]
fn remove_many_equals_remove_per_key_and_writes_the_same_bytes() {
    let base = base_kv(300, 5);
    let keys: Vec<String> = (0..700).step_by(3).map(|i| format!("k{i:07}")).chain(["k0000003".to_string(), "nope".into()]).collect();
    let mut one = Kv { entries: base.entries.clone() };
    let hit = keys.iter().filter(|k| one.remove(k)).count();
    let mut many = Kv { entries: base.entries.clone() };
    assert_eq!(many.remove_many(keys.iter().map(String::as_str)), hit, "count");
    assert!(hit > 50, "{hit}");
    assert_eq!(many.entries, one.entries);
    assert_eq!(many.compacted_bytes_fit(1 << 24).unwrap(), one.compacted_bytes_fit(1 << 24).unwrap(), "bytes");
    assert_eq!(many.remove_many([]), 0);
}

/// `cargo test --release --lib w_kvperf_measure -- --ignored --nocapture`: old path (the
/// merge above, what reopen paid before) vs new, same process, same images.
#[test]
#[ignore]
fn w_kvperf_measure() {
    use std::time::Instant;
    let base = base_kv(100_000, 256);
    let base_bytes = base.compacted_bytes_fit(1 << 30).unwrap();
    for d in [0usize, 1, 32] {
        let names: Vec<String> = (0..d).map(|i| format!("k{:07}", 6000 * i + 2)).collect();
        let val = [7u8; 256];
        let ops: Vec<Op> = names.iter().map(|k| Op::Put(k, &val)).collect();
        let b = if d == 0 { base_bytes.clone() } else { v3(&base, &[ops], &mut Kv { entries: Vec::new() }) };
        let v = View::new(&b);
        let ck = KvIn::open(&v).unwrap().checked();
        let reps = 200;
        let (mut tn, mut to) = (0f64, 0f64);
        for i in 0..reps {
            let key = format!("k{:07}", 2 * (i * 487 % 100_000) + 1);
            let t = Instant::now();
            let r = KvIn::reopen(&v, ck).unwrap();
            assert!(r.get(key.as_bytes()).unwrap().is_some());
            tn += t.elapsed().as_secs_f64();
            let t = Instant::now();
            let r = KvIn::reopen(&v, ck).unwrap();
            let m = if r.ov.is_empty() { Vec::new() } else { old_merge(&r) };
            assert!(r.get(key.as_bytes()).unwrap().is_some() && m.len() <= r.n + d);
            to += t.elapsed().as_secs_f64();
        }
        println!("MEASURE reopen+get 1e5x256 deltas={d}: new {:.2} us  old(control) {:.2} us", tn / reps as f64 * 1e6, to / reps as f64 * 1e6);
    }
    for n in [100_000usize, 1_000_000] {
        let base = base_kv(n, 16);
        let keys: Vec<String> = (0..n).step_by(2).map(|i| format!("k{:07}", 2 * i + 1)).collect();
        let mut many = Kv { entries: base.entries.clone() };
        let t = Instant::now();
        let got = many.remove_many(keys.iter().map(String::as_str));
        let tm = t.elapsed().as_secs_f64();
        assert_eq!(got, keys.len());
        let mut one = Kv { entries: base.entries.clone() };
        let sample = if n > 100_000 { 2_000 } else { keys.len() };
        let t = Instant::now();
        for k in &keys[..sample] {
            assert!(one.remove(k));
        }
        let ts = t.elapsed().as_secs_f64();
        println!(
            "MEASURE remove 50% of {n}x16: remove_many {:.1} ms total | remove per key {:.2} us/call over {sample} calls (control; x{} = {:.1} s projected)",
            tm * 1e3, ts / sample as f64 * 1e6, keys.len(), ts / sample as f64 * keys.len() as f64
        );
    }
}
