//! Projection nodes: a memo HIT equals a recompute, a STEP equals the full fold, and a
//! memo that no longer describes its input is refused (RT §6, S-2..S-5; K-4).

use super::*;
use crate::View;
use crate::evlog::{pack_payload, EvLog, Record};
use crate::kv::Kv;
use crate::nodekey::sha256;

fn id_of(prev: &[u8; 32], payload: &[u8]) -> [u8; 32] {
    let mut b = prev.to_vec();
    b.extend_from_slice(payload);
    sha256(&b)
}

/// Record `i`: a 330-byte payload (R §4's event size) whose bytes depend on `i`, chained
/// the way dowiz chains content ids (sha256 over prev || payload).
fn rec(i: u64, prev: &[u8; 32]) -> Record {
    let payload: Vec<u8> = (0..330u64).map(|j| ((i * 31 + j * 7) % 251) as u8).collect();
    Record { id: id_of(prev, &payload), prev: *prev, actor_pubkey: [0; 32], actor_seq: i, payload }
}

fn log_of(n: u64, bytes: usize) -> (Store, [u8; 32]) {
    let mut st = Store::create_bytes(bytes).unwrap();
    EvLog::init_bytes(&mut st).unwrap();
    let mut prev = [0u8; 32];
    for i in 0..n {
        let r = rec(i, &prev);
        prev = r.id;
        EvLog::append_tip_bytes(&mut st, &r).unwrap();
    }
    (st, prev)
}

fn append(st: &mut Store, prev: &mut [u8; 32], from: u64, k: u64) {
    for i in from..from + k {
        let r = rec(i, prev);
        *prev = r.id;
        EvLog::append_tip_bytes(st, &r).unwrap();
    }
}

fn code() -> i64 {
    rust_code()
}

fn fk() -> i64 {
    fn_key("st_log_step")
}

/// S-2: after k appends the memo is extended by k fold steps, and the value equals a
/// full refold (`hi == hf`), for several log sizes and several k.
#[test]
fn step_equals_full() {
    for n in [1u64, 7, 300] {
        let (mut st, mut prev) = log_of(n, 4 << 20);
        let (v0, how0) = eval_log(&mut st, code(), fk()).unwrap();
        assert_eq!(how0, How::Full, "n={n}: the first read has no memo");
        assert_eq!(v0, log_fold_full(&st).0, "n={n}: the full fold");
        let (v1, how1) = eval_log(&mut st, code(), fk()).unwrap();
        assert_eq!((v1, how1), (v0, How::Hit), "n={n}: the second read is a hit");
        let mut next = n;
        for k in [1u64, 2, 50] {
            append(&mut st, &mut prev, next, k);
            next += k;
            let (hi, how) = eval_log(&mut st, code(), fk()).unwrap();
            let (hf, len) = log_fold_full(&st);
            assert_eq!(how, How::Step(k as usize), "n={n} k={k}: extended, not refolded");
            assert_eq!(hi, hf, "n={n} k={k}: hi == hf");
            assert_eq!(len as u64, next, "n={n} k={k}: the full fold saw every record");
        }
    }
}

/// A HIT reads a constant number of cells, whatever the log's length: the same count at
/// 540 and at 5,400 events, through a borrowed view (no `from_bytes` copy at all).
#[test]
fn hit_is_o1() {
    let mut reads = Vec::new();
    for n in [540u64, 5400] {
        let (mut st, _) = log_of(n, 8 << 20);
        let (v, _) = eval_log(&mut st, code(), fk()).unwrap();
        let (_, how) = eval_log(&mut st, code(), fk()).unwrap();
        assert_eq!(how, How::Hit, "n={n}");
        let img = st.to_bytes_trimmed();
        let view = View::new(&img);
        let key = proj_key(code(), fk(), KIND_LOG);
        assert_eq!(read_hit(&view, key, code()), Some(v), "n={n}: the view answers the memo");
        reads.push(view.reads());
    }
    assert_eq!(reads[0], reads[1], "cells read by a hit at 540 vs 5,400 events");
    assert!(reads[0] < 200, "a hit read {} cells", reads[0]);
}

/// S-4, the spec's shape: record 3 of 10 is rewritten IN PLACE at the same generation and
/// the ids after it re-chained (so the tip moves). The next read must refold -- the tip
/// check refuses the stale memo -- and the value differs from the stale OUT.
#[test]
fn redaction_drops_memo() {
    let (mut st, _) = log_of(10, 1 << 20);
    let (stale, _) = eval_log(&mut st, code(), fk()).unwrap();
    let gen = st.pick().unwrap().generation;

    let mut objs = Vec::new(); // newest first
    let mut cur = st.follow(st.root().unwrap(), 1);
    while let Some(o) = cur {
        objs.push(o);
        cur = st.follow(o, 2);
    }
    objs.reverse(); // oldest first
    let mut prev = [0u8; 32];
    for (i, &o) in objs.iter().enumerate() {
        let mut r = rec(i as u64, &prev);
        if i == 3 {
            r.payload.iter_mut().for_each(|b| *b = b'x');
        }
        let id = id_of(&prev, &r.payload);
        for (j, c) in pack_payload(&r.payload).iter().enumerate() {
            st.put_cell(o, 12 + j, *c);
        }
        for q in 0..4 {
            let w = |b: &[u8; 32]| i64::from_le_bytes(b[q * 8..q * 8 + 8].try_into().unwrap());
            st.put_cell(o, 3 + q, w(&id));
            st.put_cell(o, 7 + q, w(&prev));
        }
        st.seal(o);
        prev = id;
    }
    assert_eq!(st.pick().unwrap().generation, gen, "rewritten at the same generation");

    let (v, how) = eval_log(&mut st, code(), fk()).unwrap();
    assert_eq!(how, How::Refold("tip"), "the moved tip refuses the memo");
    assert_ne!(v, stale, "the refold differs from the stale OUT");
    assert_eq!(v, log_fold_full(&st).0, "and equals a full fold");
}

/// The tree's OWN redaction (`dowiz_hub::Hub::redact`) keeps every id and rebuilds a FRESH
/// image. The tip does not move -- so the tip check cannot be what protects it -- and the
/// fresh image carries no projection table, so the first read is a full fold.
#[test]
fn a_rebuilt_image_has_no_memo() {
    let (mut st, _) = log_of(10, 1 << 20);
    let (stale, _) = eval_log(&mut st, code(), fk()).unwrap();
    let mut records = EvLog::walk(&st);
    records.reverse();
    records[3].payload.iter_mut().for_each(|b| *b = b'x');
    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    EvLog::init_bytes(&mut fresh).unwrap();
    for r in &records {
        EvLog::append_tip_bytes(&mut fresh, r).unwrap();
    }
    assert_eq!(EvLog::tip(&fresh), EvLog::tip(&st), "the tip did not move");
    let (v, how) = eval_log(&mut fresh, code(), fk()).unwrap();
    assert_eq!(how, How::Full, "no projection table in a rebuilt image");
    assert_ne!(v, stale);
}

/// K-4: a memo written by different code is never used -- the code is IN the key, so the
/// other code's read finds nothing and folds whole -- and it SUPERSEDES the old code's row
/// (a deploy replaces its memo; the table does not grow per deploy). The next read under
/// the new code is a hit; the old code, back again, finds nothing either.
#[test]
fn foreign_code_is_refused() {
    let (mut st, _) = log_of(20, 1 << 20);
    let (v, _) = eval_log(&mut st, code(), fk()).unwrap();
    let other = code() ^ 1;
    assert_eq!(eval_log(&mut st, other, fk()).unwrap(), (v, How::Full));
    let t = projtab(&st).unwrap();
    assert_eq!(st.get(t, 0), 1, "one row: the deploy superseded the old code's memo");
    assert_eq!(eval_log(&mut st, other, fk()).unwrap(), (v, How::Hit));
    assert_eq!(eval_log(&mut st, code(), fk()).unwrap(), (v, How::Full));
}

/// A memo whose bytes do not match their CRC is refolded, not trusted -- and a flipped
/// value bit is exactly that.
#[test]
fn a_corrupt_memo_is_refolded_not_trusted() {
    let (mut st, _) = log_of(20, 1 << 20);
    let (v, _) = eval_log(&mut st, code(), fk()).unwrap();
    let key = proj_key(code(), fk(), KIND_LOG);
    let p = lookup(&st, key).expect("the memo is in the table");
    st.cells[p.out + 2] ^= 1 << 17;
    assert_eq!(eval_log(&mut st, code(), fk()).unwrap(), (v, How::Refold("crc")));
}

/// A plain append (a commit that knows nothing of projections) carries the table forward,
/// and an image that never had one keeps cell 5 at zero.
#[test]
fn the_table_survives_a_plain_commit_and_is_absent_otherwise() {
    let (mut st, mut prev) = log_of(5, 1 << 20);
    assert_eq!(st.cells[st.pick().unwrap().at + SB_PROJTAB], 0, "no projection, no table");
    let _ = eval_log(&mut st, code(), fk()).unwrap();
    let t = st.cells[st.pick().unwrap().at + SB_PROJTAB];
    assert!(t > 0, "the table is named");
    append(&mut st, &mut prev, 5, 1);
    assert_eq!(st.cells[st.pick().unwrap().at + SB_PROJTAB], t, "carried by EvLog's commit");
    assert_eq!(st.cells[st.pick().unwrap().at + SB_ANC], 0, "this writer does not anchor");
}

/// S-3: the KV snapshot is memoised per generation, NOT extended -- a put re-derives it
/// whole, and a read with no put is a hit.
#[test]
fn kv_memo_is_rederived_whole_on_a_put() {
    let p = std::env::temp_dir().join("bebop_proj_kv.store");
    let p = p.to_str().unwrap();
    let mut st = Store::create(p, 1 << 20).unwrap();
    Kv::init(&mut st, p).unwrap();
    let mut kv = Kv::load(&st).unwrap();
    kv.put("a", b"1");
    kv.commit_into(&mut st, p).unwrap();
    let fkv = fn_key("kv_snapshot");
    let (v, how) = eval_kv(&mut st, code(), fkv).unwrap();
    assert_eq!((v, how), (kv.snapshot_root_u64() as i64, How::Full));
    assert_eq!(eval_kv(&mut st, code(), fkv).unwrap(), (v, How::Hit));
    kv.put("b", b"22");
    kv.commit_into(&mut st, p).unwrap();
    let (v2, how2) = eval_kv(&mut st, code(), fkv).unwrap();
    assert_eq!(how2, How::Refold("gen"), "a put re-derives the snapshot whole");
    assert_eq!(v2, Kv::load(&st).unwrap().snapshot_root_u64() as i64);
    let _ = std::fs::remove_file(p);
}

/// The layout digests are `st_digest` of their layout strings: sha256, low 32 bits.
#[test]
fn digests_are_st_digest_of_their_layouts() {
    let lo32 = |s: &str| {
        let h = sha256(s.as_bytes());
        u32::from_be_bytes([h[28], h[29], h[30], h[31]]) as i64
    };
    assert_eq!(DIGEST_PROJTAB, lo32(LAYOUT_PROJTAB));
    assert_eq!(DIGEST_PROJ, lo32(LAYOUT_PROJ));
    assert_eq!(DIGEST_OUT, lo32(LAYOUT_OUT));
}

/// MEASURED, not asserted (`cargo test --release -- --ignored --nocapture measure`):
/// one append + fold_step vs the full fold, at 5,400 and 54,000 events (R §4's shapes).
#[test]
#[ignore]
fn measure_append_step_vs_full() {
    for n in [5400u64, 54_000] {
        let (mut st, mut prev) = log_of(n, if n > 10_000 { 64 << 20 } else { 8 << 20 });
        let _ = eval_log(&mut st, code(), fk()).unwrap();
        let reps = 50u64;
        let t = std::time::Instant::now();
        for i in 0..reps {
            append(&mut st, &mut prev, n + i, 1);
            let (_, how) = eval_log(&mut st, code(), fk()).unwrap();
            assert_eq!(how, How::Step(1));
        }
        let step_ms = t.elapsed().as_secs_f64() * 1e3 / reps as f64;
        let t = std::time::Instant::now();
        let mut h = 0;
        for _ in 0..10 {
            h = log_fold_full(&st).0;
        }
        let full_ms = t.elapsed().as_secs_f64() * 1e3 / 10.0;
        let (hi, _) = eval_log(&mut st, code(), fk()).unwrap();
        assert_eq!(hi, h, "hi == hf");
        let img = st.to_bytes_trimmed();
        let t = std::time::Instant::now();
        for _ in 0..10 {
            let _ = Store::from_bytes(&img);
        }
        let copy_ms = t.elapsed().as_secs_f64() * 1e3 / 10.0;
        let t = std::time::Instant::now();
        let mut got = None;
        for _ in 0..1000 {
            got = read_hit(&View::new(&img), proj_key(code(), fk(), KIND_LOG), code());
        }
        let view_us = t.elapsed().as_secs_f64() * 1e6 / 1000.0;
        assert_eq!(got, Some(h));
        println!(
            "measure n={n} append+step_ms={step_ms:.4} full_fold_ms={full_ms:.3} ratio={:.0}x \
             from_bytes_ms={copy_ms:.2} view_hit_us={view_us:.2} image_bytes={}",
            full_ms / step_ms,
            img.len()
        );
    }
}
