//! T-LV-INC-02: kinds 3 and 4 -- a STEP equals the full recomputation over 10^4 events, a memo
//! that no longer describes its input is refolded, a corrupt one is not trusted; and the two
//! fixtures crates/bebop-wasm/fixtures/proj4{,.log}.store are what this writer writes.
use super::super::tests::{append, code, log_of};
use super::*;
use crate::kv::delta::{append_delta, append_delta_with, Appended, Op, MAX_DELTAS};
use crate::kv::Kv;
use crate::nodekey::sha256;
use crate::proj::{fn_key, lookup};
use crate::root_in;
use crate::zset::tests::{base, delta};

fn fsum() -> i64 {
    fn_key("log_summary")
}
fn fz() -> i64 {
    fn_key("zset_identity")
}
/// The full recomputation every step answers to.
fn full_z(st: &Store) -> ZSet<Vec<i64>> {
    let kv = Kv::load(st).expect("a KV");
    Identity.apply(&ZSet::from_pairs(kv.entries.into_iter().map(|(k, v)| ((k.into_bytes(), v), 1)).collect()))
}
fn full_blob(st: &Store) -> Vec<u8> {
    let mut rs = EvLog::walk(st);
    rs.reverse();
    LogSummary.full(&rs)
}

#[test]
fn digests_are_st_digest_of_their_layouts() {
    let lo32 = |s: &str| {
        let h = sha256(s.as_bytes());
        u32::from_be_bytes([h[28], h[29], h[30], h[31]]) as i64
    };
    assert_eq!((DIGEST_OUTB, DIGEST_OUTZ), (lo32(LAYOUT_OUTB), lo32(LAYOUT_OUTZ)));
}

/// Kind 3 over 10^4 events in uneven batches: every read after an append is a k-record
/// STEP, and its bytes equal the full fold's; the read after that is a HIT.
#[test]
fn step_equals_full_kind3() {
    let (mut st, mut prev) = log_of(1, 16 << 20);
    let (b0, how0) = eval_blob(&mut st, code(), fsum(), &LogSummary).unwrap();
    assert_eq!((b0, how0), (full_blob(&st), How::Full));
    let mut next = 1u64;
    for k in [1u64, 7, 50, 300, 1000, 2, 97, 3000, 5542].into_iter().cycle().take(9) {
        append(&mut st, &mut prev, next, k);
        next += k;
        let (b, how) = eval_blob(&mut st, code(), fsum(), &LogSummary).unwrap();
        assert_eq!(how, How::Step(k as usize), "next={next}");
        assert_eq!(b, full_blob(&st), "next={next}: step == full");
        assert_eq!(eval_blob(&mut st, code(), fsum(), &LogSummary).unwrap(), (b, How::Hit));
    }
    assert_eq!(next, 10_000);
}

/// Kind 4 over 10^4 KV edits (puts, removals, re-puts) under the writer's REAL compaction
/// policy: every read equals the full recomputation; between compactions it is a STEP over
/// exactly the records appended; a compaction (a fresh image) reads Full.
#[test]
fn step_equals_full_kind4() {
    let keys: Vec<String> = (0..60).map(|i| format!("dish/{i:03}")).collect();
    let mut st = base(&keys[..50].iter().map(|k| (k.as_str(), "100")).collect::<Vec<_>>());
    let (mut steps, mut fulls, mut compactions) = (0, 0, 0);
    for i in 0..10_000usize {
        let k = keys[(i * 7) % 60].as_str();
        let v = format!("{}", 100 + i);
        let op = if i % 5 == 0 { Op::Remove(k) } else { Op::Put(k, v.as_bytes()) };
        match append_delta(&mut st, &[op]).unwrap() {
            Appended::Delta(_) => {}
            Appended::Compact(_) => {
                let mut kv = Kv::load(&st).unwrap();
                match op {
                    Op::Put(k, v) => kv.put(k, v),
                    Op::Remove(k) => {
                        kv.remove(k);
                    }
                }
                st = Store::from_bytes(&kv.compacted_bytes(1 << 20).unwrap());
                compactions += 1;
            }
        }
        let (z, how) = eval_zset(&mut st, code(), fz(), &Identity).unwrap();
        assert_eq!(z, full_z(&st), "i={i}: step == full ({how:?})");
        match how {
            How::Step(1) => steps += 1,
            How::Full => fulls += 1,
            other => panic!("i={i}: {other:?}"),
        }
    }
    assert_eq!(fulls, compactions + 1, "Full only on the first image and on each fresh (compacted) one");
    assert!(steps > 9_000 && compactions > 0, "steps={steps} compactions={compactions}");
}

/// S-4 for kind 4. (i) An OLDER delta record rewritten and re-sealed IN PLACE at the same
/// generation: the newest record and the root did not move, only that record's h1 -- the
/// fingerprint refuses the memo. (ii) The base rewritten in the same image (`commit_into`).
/// (iii) A compacted (fresh) image carries no memo at all.
#[test]
fn redaction_drops_memo_kind4() {
    let mut st = base(&[("a", "1"), ("b", "22")]);
    delta(&mut st, &[Op::Put("a", b"x")]);
    delta(&mut st, &[Op::Put("c", b"3")]);
    let (stale, _) = eval_zset(&mut st, code(), fz(), &Identity).unwrap();
    let root = root_in(&st).unwrap();
    let oldest = *chain_in(&st, root).unwrap().last().unwrap();
    let gen = st.pick().unwrap().generation;
    st.put_cell(oldest.obj, oldest.val_cell(), b'y' as i64);
    st.seal(oldest.obj);
    assert_eq!(st.pick().unwrap().generation, gen, "rewritten at the same generation");
    let (z, how) = eval_zset(&mut st, code(), fz(), &Identity).unwrap();
    assert_eq!(how, How::Refold("tip"), "(i) the fingerprint refuses the memo");
    assert_ne!(z, stale);
    assert_eq!(z, full_z(&st), "(i) and the refold reads the rewritten value");

    let p = std::env::temp_dir().join("bebop_kinds_redact.store");
    let p = p.to_str().unwrap();
    let mut st = Store::create(p, 1 << 20).unwrap();
    Kv::init(&mut st, p).unwrap();
    let mut kv = Kv::load(&st).unwrap();
    kv.put("a", b"1");
    kv.commit_into(&mut st, p).unwrap();
    let _ = eval_zset(&mut st, code(), fz(), &Identity).unwrap();
    kv.put("a", b"2");
    kv.commit_into(&mut st, p).unwrap();
    let (z, how) = eval_zset(&mut st, code(), fz(), &Identity).unwrap();
    assert_eq!((how, z), (How::Refold("tip"), full_z(&st)), "(ii) a rewritten base");
    let _ = std::fs::remove_file(p);

    let mut fresh = Store::from_bytes(&Kv::load(&st).unwrap().compacted_bytes(1 << 20).unwrap());
    assert_eq!(eval_zset(&mut fresh, code(), fz(), &Identity).unwrap().1, How::Full, "(iii)");
}

/// S-4 for kind 3: the newest record's id re-chained in place (sproj.bp's sp_redact shape),
/// same generation and count -- the tip check refuses the memo.
#[test]
fn redaction_drops_memo_kind3() {
    let (mut st, _) = log_of(10, 1 << 20);
    let (stale, _) = eval_blob(&mut st, code(), fsum(), &LogSummary).unwrap();
    let root = st.root().unwrap();
    let tip = st.follow(root, 1).unwrap();
    st.put_cell(tip, 12, st.get(tip, 12) ^ 0x7878);
    st.put_cell(tip, 3, st.get(tip, 3) ^ 1);
    st.seal(tip);
    st.put_cell(root, 3, st.get(tip, 3));
    st.seal(root);
    let (b, how) = eval_blob(&mut st, code(), fsum(), &LogSummary).unwrap();
    assert_eq!(how, How::Refold("tip"));
    assert_ne!(b, stale);
    assert_eq!(b, full_blob(&st));
}

/// One named flipped cell -- row 0's first cell in the OUTZ -- and the memo is refolded,
/// not trusted. Its positive twin: the untouched memo is a HIT.
#[test]
fn a_corrupt_outz_is_refolded() {
    let mut st = base(&[("a", "1"), ("b", "22")]);
    let (z, _) = eval_zset(&mut st, code(), fz(), &Identity).unwrap();
    assert_eq!(eval_zset(&mut st, code(), fz(), &Identity).unwrap(), (z.clone(), How::Hit));
    let p = lookup(&st, proj_key(code(), fz(), KIND_ZSET)).unwrap();
    st.cells[p.out + 2 + 4] ^= 1 << 9;
    assert_eq!(eval_zset(&mut st, code(), fz(), &Identity).unwrap(), (z, How::Refold("crc")));
}

/// A kind-1 reader meets a kind-4 row in the same table: it is not its key, and a kind-4 OUT
/// under a kind-1 PROJ would be refused by `out_ok` (the old readers' rule, 2 cells).
#[test]
fn out_ok_is_per_kind() {
    let mut st = base(&[("a", "1")]);
    let _ = eval_zset(&mut st, code(), fz(), &Identity).unwrap();
    let p = lookup(&st, proj_key(code(), fz(), KIND_ZSET)).unwrap();
    assert!(out_ok(&st, KIND_ZSET, p.out));
    assert!(!out_ok(&st, 1, p.out) && !out_ok(&st, KIND_BLOB, p.out) && !out_ok(&st, 9, p.out));
}

fn wasm_fixture(name: &str) -> String {
    format!("{}/../bebop-wasm/fixtures/{name}", env!("CARGO_MANIFEST_DIR"))
}

/// proj4.store: kv2.store + 1 delta, a first read (Full), 2 more deltas, a second read that
/// STEPS by 2. proj4.log.store: 7 records, a first read, 2 more, a 2-record step.
/// `PROJ4_WRITE=1` writes them; otherwise the committed bytes must be exactly these.
#[test]
fn proj4_fixtures_are_what_this_writer_writes() {
    let mut st = Store::from_bytes(&std::fs::read(wasm_fixture("kv2.store")).unwrap());
    let ops: [&[Op]; 3] = [&[Op::Put("order/0002", b"cancelled")], &[Op::Put("zone/south", b"open")], &[Op::Remove("courier/alpha")]];
    assert!(matches!(append_delta_with(&mut st, ops[0], MAX_DELTAS, 0).unwrap(), Appended::Delta(_)));
    assert_eq!(eval_zset(&mut st, code(), fz(), &Identity).unwrap().1, How::Full);
    for o in &ops[1..] {
        assert!(matches!(append_delta_with(&mut st, o, MAX_DELTAS, 0).unwrap(), Appended::Delta(_)));
    }
    let (z, how) = eval_zset(&mut st, code(), fz(), &Identity).unwrap();
    assert_eq!((how, z), (How::Step(2), full_z(&st)));
    let kv = st.to_bytes_trimmed();

    let (mut st, mut prev) = log_of(7, 1 << 20);
    assert_eq!(eval_blob(&mut st, code(), fsum(), &LogSummary).unwrap().1, How::Full);
    append(&mut st, &mut prev, 7, 2);
    let (b, how) = eval_blob(&mut st, code(), fsum(), &LogSummary).unwrap();
    assert_eq!((how, b), (How::Step(2), full_blob(&st)));
    let log = st.to_bytes_trimmed();
    for (name, bytes) in [("proj4.store", kv), ("proj4.log.store", log)] {
        if std::env::var_os("PROJ4_WRITE").is_some() {
            std::fs::write(wasm_fixture(name), &bytes).unwrap();
        }
        assert_eq!(std::fs::read(wasm_fixture(name)).ok(), Some(bytes), "{name}: re-run with PROJ4_WRITE=1 only on purpose");
    }
}

/// MEASURED, not asserted (`cargo test --release --lib -- --ignored --nocapture measure_kinds`), each against
/// its full recomputation IN THE SAME PROCESS. proj4_price_step_us: a 165-dish catalogue (2,500-byte values,
/// the 548 KB shape), one price put appended, then eval_zset STEPS over that one record; control = Identity
/// over from_kv (the whole catalogue). proj3_cold_read_us: a 2,000-event log whose kind-3 memo was written at
/// 1,991 (the FRESH wake shape: 1,991 events), +9 records, then eval_blob; control = LogSummary.full.
#[test]
#[ignore]
fn measure_kinds() {
    use std::time::Instant;
    let keys: Vec<String> = (0..165).map(|i| format!("dish/{i:03}")).collect();
    let val = |i: usize, p: usize| format!("{p:08}{}", "x".repeat(2492 + i % 7));
    let mut kv = Kv { entries: Vec::new() };
    for (i, k) in keys.iter().enumerate() {
        kv.put(k, val(i, 100).as_bytes());
    }
    let img0 = kv.compacted_bytes(4 << 20).unwrap();
    // At the REAL chain lengths (the writer compacts at MAX_DELTAS = 32): the memo is current at chain
    // L - 1, one price put makes it L, and that one eval_zset is timed -- split into its parts.
    for l in [1usize, 8, 32] {
        let mut st = Store::from_bytes(&img0);
        let _ = eval_zset(&mut st, code(), fz(), &Identity).unwrap();
        for r in 0..l - 1 {
            let v = val(r, 300 + r);
            assert!(matches!(append_delta_with(&mut st, &[Op::Put(&keys[r], v.as_bytes())], MAX_DELTAS, 0).unwrap(), Appended::Delta(_)));
            let _ = eval_zset(&mut st, code(), fz(), &Identity).unwrap();
        }
        let base_img = st.to_bytes();
        let reps = 100u32;
        let (mut step, mut read, mut delta, mut full) = (0u128, 0u128, 0u128, 0u128);
        for r in 0..reps as usize {
            let mut st = Store::from_bytes(&base_img);
            let v = val(100, 900 + r);
            assert!(matches!(append_delta_with(&mut st, &[Op::Put(&keys[100], v.as_bytes())], MAX_DELTAS, 0).unwrap(), Appended::Delta(_)));
            let root = root_in(&st).unwrap();
            let p = lookup(&st, proj_key(code(), fz(), KIND_ZSET)).unwrap();
            let t = Instant::now();
            let _ = zset_of(&st, p.out);
            read += t.elapsed().as_nanos();
            let t = Instant::now();
            let _ = Identity.apply(&crate::zset::from_kv_chain(&st, root, 1).unwrap());
            delta += t.elapsed().as_nanos();
            let t = Instant::now();
            let f = Identity.apply(&crate::zset::from_kv(&st, root).unwrap());
            full += t.elapsed().as_nanos();
            let t = Instant::now();
            let (z, how) = eval_zset(&mut st, code(), fz(), &Identity).unwrap();
            step += t.elapsed().as_nanos();
            assert_eq!((how, z), (How::Step(1), f), "L={l}");
        }
        let us = |ns: u128| ns as f64 / 1e3 / reps as f64;
        println!(
            "measure proj4_price_step_us={:.1} chain={l} (memo read {:.1}, delta {:.1}, rest = lookup+fingerprint+write {:.1}) full_us={:.1} ratio={:.1}x",
            us(step), us(read), us(delta), us(step) - us(read) - us(delta), us(full), us(full) / us(step)
        );
    }

    let (mut st, prev) = log_of(1991, 16 << 20);
    let _ = eval_blob(&mut st, code(), fsum(), &LogSummary).unwrap();
    let img = st.to_bytes();
    let (mut step_ns, mut hit_ns, mut full_ns) = (0u128, 0u128, 0u128);
    let reps = 50;
    for _ in 0..reps {
        let mut st = Store::from_bytes(&img);
        let mut p = prev;
        append(&mut st, &mut p, 1991, 9);
        let t = Instant::now();
        let (b, how) = eval_blob(&mut st, code(), fsum(), &LogSummary).unwrap();
        step_ns += t.elapsed().as_nanos();
        assert_eq!(how, How::Step(9));
        let t = Instant::now();
        let (_, how) = eval_blob(&mut st, code(), fsum(), &LogSummary).unwrap();
        hit_ns += t.elapsed().as_nanos();
        assert_eq!(how, How::Hit);
        let t = Instant::now();
        let f = full_blob(&st);
        full_ns += t.elapsed().as_nanos();
        assert_eq!(b, f);
    }
    let us = |ns: u128| ns as f64 / 1e3 / reps as f64;
    println!(
        "measure proj3_cold_read_us={:.1} hit_us={:.1} full_us={:.1} ratio={:.1}x events=2000",
        us(step_ns),
        us(hit_ns),
        us(full_ns),
        us(full_ns) / us(step_ns)
    );
}
