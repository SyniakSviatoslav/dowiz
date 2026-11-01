//! THE FOUR FIXTURES FMT3 ASKED FOR (W-MG-READ): rules of FORMAT §9.1, §9.2 and §9.4 that no fixture
//! could fail. `BEBOP_RULE_FIXTURES=<dir> cargo test rule_fixtures` writes them beside the others.
//! Each image comes from the library's own `Store` and `EvLog` writers; what no writer here will
//! produce (an unsorted base, a future log version) is laid out through the same allocator.
//!
//! | fixture        | the rule it can fail                                             |
//! |----------------|------------------------------------------------------------------|
//! | cut-undeclared | §9.4 a sealed cut with no TRUNCATE: 75                           |
//! | log-v1         | §9.2 a v1 log: one byte per cell, 15-cell head                   |
//! | kv-unsorted    | §9.1 a base not strictly ascending, empty chain: 2               |
//! | log-v3         | §9.2 a log root naming a version above 2: 12                     |

use super::super::tests::key_of;
use super::super::verify::{verify, Trust};
use super::super::{Doc, DocError};
use super::fixture_ids::{Blind, FixIds};
use super::fixture_tests::{emit, fresh, log_fold as doc_fold, owner, sha_commit};
use crate::{EvLog, Record, Store, DIGEST_EVLOG_ROOT};

const DIGEST_KV_ROOT: i64 = 610082063;
const DIGEST_ARR_I64: i64 = 4290599237;

fn rec(seq: u64, payload: &[u8]) -> Record {
    Record { id: [seq as u8; 32], prev: [0; 32], actor_pubkey: [0; 32], actor_seq: seq, flags: 0, payload: payload.to_vec() }
}

/// `status=<s>` / `n=` + `root=` as `conformance.sh`'s `.expected` files say them.
fn lines(what: &str, body: &str) -> String {
    format!("# {what}\n# Written by doc/spec/fixture_wishes.rs (W-MG-READ), BEBOP_RULE_FIXTURES=<dir>; deterministic.\n{body}\n")
}

fn log_fold(st: &Store) -> (usize, i64) {
    let (mut h, mut n) = (0xcbf2_9ce4_8422_2325u64, 0usize);
    let mut records = EvLog::walk(st);
    records.reverse();
    for r in &records {
        n += 1;
        for b in r.actor_seq.to_le_bytes().iter().chain(&(r.payload.len() as u64).to_le_bytes()).chain(&r.payload) {
            h = (h ^ *b as u64).wrapping_mul(0x100_0000_01b3);
        }
    }
    (n, h as i64)
}

#[test]
fn rule_fixtures_a_sealed_cut_with_no_truncate() {
    let by = owner();
    let build = || {
        let mut st = fresh(&[key_of(&by)]);
        let mut tip = [0u8; 32];
        for i in 1..=5u64 {
            tip = sha_commit(&mut st, &by, tip, i, format!("event-{i}").as_bytes());
        }
        let mut cut = Store::create_bytes(1 << 20).unwrap();
        Doc::cut_into(&st, &mut cut, 2, &FixIds).unwrap();
        cut
    };
    assert_eq!(build().to_bytes_trimmed(), build().to_bytes_trimmed(), "cut-undeclared is not deterministic");
    let st = build();
    assert!(matches!(verify(&st, &FixIds, &Blind, Trust::Genesis), Err(DocError::CutUndeclared { .. })));
    let n = EvLog::walk_until_at(&st, Doc::open(&st).unwrap().log, |_| false).len();
    let what = lines("Three sealed records cut under a SEG and no TRUNCATE on the kept chain: 75.", &format!("doc_n={n}\ndoc_root={}\ndoc_verify=status=75", doc_fold(&st)));
    emit("cut-undeclared", &st.to_bytes_trimmed(), &what);
}

#[test]
fn rule_fixtures_a_v1_log() {
    let build = || {
        let mut st = Store::create_bytes(1 << 16).unwrap();
        EvLog::init_v1_bytes(&mut st).unwrap();
        for (i, p) in [&b"one"[..], b"two", b"three"].iter().enumerate() {
            EvLog::append_bytes(&mut st, &rec(i as u64 + 1, p)).unwrap();
        }
        st
    };
    assert_eq!(build().to_bytes_trimmed(), build().to_bytes_trimmed(), "log-v1 is not deterministic");
    let st = build();
    assert_eq!(EvLog::version(&st), 1);
    let (n, root) = log_fold(&st);
    emit("log-v1", &st.to_bytes_trimmed(), &lines("A v1 log: one payload byte per cell.", &format!("n={n}\nroot={root}")));
}

#[test]
fn rule_fixtures_a_base_out_of_order() {
    let build = || {
        let mut st = Store::create_bytes(1 << 16).unwrap();
        let mut tx = st.begin().unwrap();
        let mut arr = |st: &mut Store, cells: &[i64]| {
            let o = st.alloc(&mut tx, cells.len() as i64, DIGEST_ARR_I64).unwrap();
            cells.iter().enumerate().for_each(|(i, c)| st.put_cell(o, i, *c));
            st.seal(o);
            o
        };
        let kidx = arr(&mut st, &[1, 1, 0, 1]); // "b" then "a"
        let kblob = arr(&mut st, &[i64::from_le_bytes(*b"ab\0\0\0\0\0\0")]);
        let vidx = arr(&mut st, &[0, 1, 1, 1]);
        let vblob = arr(&mut st, &[i64::from_le_bytes(*b"12\0\0\0\0\0\0")]);
        let root = st.alloc(&mut tx, 6, DIGEST_KV_ROOT).unwrap();
        st.put_cell(root, 0, 2);
        for (i, a) in [kidx, kblob, vidx, vblob].into_iter().enumerate() {
            st.link(root, i + 1, a);
        }
        st.put_cell(root, 5, 2);
        st.seal(root);
        st.commit_bytes(&tx, root);
        st
    };
    assert_eq!(build().to_bytes_trimmed(), build().to_bytes_trimmed(), "kv-unsorted is not deterministic");
    emit("kv-unsorted", &build().to_bytes_trimmed(), &lines("A v2 base with keys b, a and no chain: not strictly ascending, 2.", "status=2"));
}

#[test]
fn rule_fixtures_a_log_version_above_2() {
    let build = || {
        let mut st = Store::create_bytes(1 << 16).unwrap();
        EvLog::init_bytes(&mut st).unwrap();
        EvLog::append_bytes(&mut st, &rec(1, b"one")).unwrap();
        let old = st.root().unwrap();
        let mut tx = st.begin().unwrap();
        let root = st.alloc(&mut tx, 8, DIGEST_EVLOG_ROOT).unwrap();
        for i in 0..8 {
            st.put_cell(root, i, st.get(old, i));
        }
        st.link(root, 1, st.follow(old, 1).unwrap());
        st.put_cell(root, 7, 3);
        st.seal(root);
        st.commit_bytes(&tx, root);
        st
    };
    assert_eq!(build().to_bytes_trimmed(), build().to_bytes_trimmed(), "log-v3 is not deterministic");
    let st = build();
    assert_eq!(EvLog::unknown_version(&st), Some(3));
    emit("log-v3", &st.to_bytes_trimmed(), &lines("A log root naming version 3: refused before the walk, 12.", "status=12"));
}
