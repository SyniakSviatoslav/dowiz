//! THE RULE FIXTURES (W-MG-FIX). A rule nothing can fail is a claim: four format rules passed with
//! a broken reader, because no fixture exercised them. Each image here is written by the library's
//! own writers, each `.expected` by the library's own `verify`, under the seams the MIT reader
//! fills (`Blind`, sha256 ids), so a reader either answers this file or disagrees with the writer.
//!
//! `EIN_RULE_FIXTURES=<dir> cargo test rule_fixtures` writes `<name>.store` + `<name>.expected`.
//! Deterministic: every image is built twice below and held byte-identical.
//!
//! | fixture           | the rule it can fail                                                |
//! |-------------------|---------------------------------------------------------------------|
//! | tomb-sha          | §9.4 a flagged tombstone: nonce erased, kind bit set, FORGOTTEN list |
//! | tomb-legacy       | §9.4 the mark is the kind byte: a legacy record has no nonce to erase|
//! | seg2              | the older segment's TRUNCATE left the file; the chain hash covers it |
//! | forged-tail       | `unsealed` counts a record no seal reaches                           |
//! | keys-disagree     | `keys_disagree`: the table against the REVOKE records                |
//! | seg2-forged       | the chain hash is CHECKED: an older segment's count moved, 75        |

use super::super::keys::{key_rows, Key, KeyOp};
use super::super::seg::read_segs;
use super::super::tests::{key_of, ToySigner, ToyVerifier};
use super::super::verify::{verify, Report, Trust};
use super::super::write::{stage_doc, Refs};
use super::super::{forgotten_body, Doc, DocError, Ids, Verifier};
use super::fixture_ids::{legacy_id, sha_id, Blind, FixIds, ERASED_NONCE, ORDER_KIND, TOMB_BIT};
use crate::{EvLog, Record, Store, FLAG_SHA256};
use crate::doc::tests::sha256;

pub(super) fn owner() -> ToySigner {
    ToySigner { id: [0xA1; 32] }
}

pub(super) fn fresh(keys: &[Key]) -> Store {
    let mut st = Store::create_bytes(1 << 20).unwrap();
    Doc::init_bytes(&mut st, 0, keys).unwrap();
    st
}

/// A sha256-id, nonce-led record of `body`, built and not yet written.
fn sha_rec(by: &ToySigner, tip: [u8; 32], seq: u64, body: &[u8]) -> Record {
    let mut payload = format!("nonce-{seq:0>10}").into_bytes();
    payload.extend_from_slice(body);
    let mut r = Record { id: [0u8; 32], prev: tip, actor_pubkey: by.id, actor_seq: seq, flags: FLAG_SHA256, payload };
    r.id = sha_id(&r);
    r
}

fn commit(st: &mut Store, by: &ToySigner, r: &Record, op: Option<&KeyOp>) -> [u8; 32] {
    Doc::commit_bytes(st, r, by, 1_700_000_000_000 + r.actor_seq as i64, op).unwrap();
    r.id
}

pub(super) fn sha_commit(st: &mut Store, by: &ToySigner, tip: [u8; 32], seq: u64, body: &[u8]) -> [u8; 32] {
    commit(st, by, &sha_rec(by, tip, seq, body), None)
}

fn legacy_commit(st: &mut Store, by: &ToySigner, tip: [u8; 32], seq: u64, payload: &[u8]) -> [u8; 32] {
    let id = legacy_id(&[&tip[..], payload].concat());
    let r = Record { id, prev: tip, actor_pubkey: by.id, actor_seq: seq, flags: 0, payload: payload.to_vec() };
    commit(st, by, &r, None)
}

/// §9.4's tombstone: nonce erased on a flagged record, the kind byte's high bit set, fields gone.
fn tomb_payload(r: &Record) -> Vec<u8> {
    let mut p = if r.flags & FLAG_SHA256 != 0 { ERASED_NONCE.to_vec() } else { Vec::new() };
    p.push(ORDER_KIND | TOMB_BIT);
    p.extend_from_slice(b"{}");
    p
}

/// Three order records, the middle one erased by `redact_into`, then the signed FORGOTTEN. The
/// FORGOTTEN is always chained: a declaration needs a chained id, which is also what moves a
/// legacy device forward.
fn tombstoned(legacy: bool) -> Store {
    let o = owner();
    let mut st = fresh(&[key_of(&o)]);
    let mut tip = [0u8; 32];
    for seq in 1..=3u64 {
        let body = [&[ORDER_KIND][..], format!("order-{seq}").as_bytes()].concat();
        tip = if legacy { legacy_commit(&mut st, &o, tip, seq, &body) } else { sha_commit(&mut st, &o, tip, seq, &body) };
    }
    let mut dst = Store::create_bytes(1 << 20).unwrap();
    let erased = Doc::redact_into(&st, &mut dst, &FixIds, &|r: &Record| (r.actor_seq == 2).then(|| tomb_payload(r))).unwrap();
    assert_eq!(erased.len(), 1);
    // Both forms §9.4 allows: the hub's `D` kind byte in front of the body, and none.
    let body = forgotten_body(&sha256(b"subject"), &erased);
    let body = if legacy { body } else { [&b"D"[..], &body].concat() };
    sha_commit(&mut dst, &o, tip, 4, &body);
    dst
}

/// Two cuts: the first TRUNCATE (seq 6) leaves under the second, so the older segment stands on
/// the chain hash alone.
fn two_cuts() -> Store {
    let o = owner();
    let mut st = fresh(&[key_of(&o)]);
    let mut tip = [0u8; 32];
    for seq in 1..=5u64 {
        tip = sha_commit(&mut st, &o, tip, seq, format!("event-{seq}").as_bytes());
    }
    let mut one = Store::create_bytes(1 << 20).unwrap();
    let seg = Doc::cut_into(&st, &mut one, 2, &FixIds).unwrap();
    tip = sha_commit(&mut one, &o, tip, 6, &seg.truncate_body());
    for seq in 7..=8u64 {
        tip = sha_commit(&mut one, &o, tip, seq, format!("event-{seq}").as_bytes());
    }
    let mut two = Store::create_bytes(1 << 20).unwrap();
    let seg = Doc::cut_into(&one, &mut two, 2, &FixIds).unwrap();
    sha_commit(&mut two, &o, tip, 9, &seg.truncate_body());
    two
}

/// `seg2` with the OLDER segment's `redacted` moved and its crc re-sealed. Its six fields are in no
/// TRUNCATE, so the newest's chain hash is the only thing that can see it.
fn forged_older_segment() -> Store {
    let mut st = two_cuts();
    let newest = Doc::open(&st).unwrap().seg.unwrap();
    let older = read_segs(&st, Some(newest), 8).unwrap()[0].prev.unwrap();
    st.put_cell(older, 9, 1);
    st.seal(older);
    st
}

/// Three sealed commits and a fourth record appended past the last seal, HEADS unmoved: a whole
/// chain of good ids that no signed tip reaches.
fn forged_tail() -> Store {
    let o = owner();
    let mut st = fresh(&[key_of(&o)]);
    let mut tip = [0u8; 32];
    for seq in 1..=3u64 {
        tip = sha_commit(&mut st, &o, tip, seq, format!("event-{seq}").as_bytes());
    }
    assert!(report(&st, &ToyVerifier).holds(), "control: before the forged record the file holds");
    let r = sha_rec(&o, tip, 4, b"forged");
    let view = Doc::open(&st).unwrap();
    let mut tx = st.begin().unwrap();
    let log = EvLog::stage_append_in(&mut st, &mut tx, view.log.unwrap(), &r, None).unwrap();
    let d = stage_doc(&mut st, &mut tx, view.flags, Refs { log: Some(log), heads: view.heads, keys: view.keys, ..Refs::default() }).unwrap();
    st.commit_bytes(&tx, d);
    st
}

/// A signed REVOKE, and the table's `revoked_at` zeroed under it (crc re-sealed): the records and
/// the index tell two stories. `after_seq` 3 covers all of B's records, so nothing else is wrong.
/// One more commit by A follows, because the KEY sits in the REVOKE's commit: left there, the
/// edit trips the last-commit anchor (41) and no reader ever reaches the rule under test.
fn keys_disagree() -> Store {
    let (a, b) = (owner(), ToySigner { id: [0xB2; 32] });
    let mut st = fresh(&[key_of(&a), key_of(&b)]);
    let mut tip = [0u8; 32];
    for seq in 1..=3u64 {
        tip = sha_commit(&mut st, &b, tip, seq, format!("b-{seq}").as_bytes());
    }
    let op = KeyOp::Revoke { actor_id: b.id, after_seq: 3, tip };
    let a_tip = commit(&mut st, &a, &sha_rec(&a, [0u8; 32], 1, &op.body()), Some(&op));
    assert!(report(&st, &ToyVerifier).holds(), "control: the table and the REVOKE agree");
    let rows = key_rows(&st, Doc::open(&st).unwrap().keys);
    let at = rows.iter().find(|r| r.0 == b.id).unwrap().1;
    st.put_cell(at, 6, 0);
    st.seal(at);
    sha_commit(&mut st, &a, a_tip, 2, b"a-2");
    st
}

fn report(st: &Store, v: &dyn Verifier) -> Report {
    verify(st, &FixIds, v, Trust::Genesis).unwrap()
}

/// `doc::verify`'s counter line as the MIT reader prints it.
fn verify_line(st: &Store) -> String {
    let r = report(st, &Blind);
    let c = r.counters;
    format!(
        "records={} chained={} legacy={} redacted={} broken={} forks={} seals={} checked=0 heads={} keys={} genesis={} segments={} cut={} cut_redacted={} declared={} keys_disagree={} unsealed={} merges={} merges_bad={}",
        c.records, c.chained, c.legacy, c.redacted, c.broken, c.forks, c.seals_ok + c.seals_bad, r.heads.len(), r.keys, r.admitted, c.segments, c.cut, c.cut_redacted, c.declared, c.keys_disagree, c.unsealed, c.merges, c.merges_bad
    )
}

/// FORMAT §9.1's LOG fold: FNV-1a 64 over `seq LE8 ‖ len LE8 ‖ payload`, oldest first.
pub(super) fn log_fold(st: &Store) -> i64 {
    let mut recs = EvLog::walk_until_at(st, Doc::open(st).unwrap().log, |_| false);
    recs.reverse();
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for r in &recs {
        let row = [&r.actor_seq.to_le_bytes()[..], &(r.payload.len() as u64).to_le_bytes(), &r.payload].concat();
        for b in row {
            h = (h ^ u64::from(b)).wrapping_mul(0x100_0000_01b3);
        }
    }
    h as i64
}

fn expected(st: &Store, what: &str) -> String {
    let n = EvLog::walk_until_at(st, Doc::open(st).unwrap().log, |_| false).len();
    format!("# {what}\n# Written by doc/spec/fixture_tests.rs (W-MG-FIX), EIN_RULE_FIXTURES=<dir>; deterministic.\ndoc_n={n}\ndoc_root={}\ndoc_verify={}\n", log_fold(st), verify_line(st))
}

pub(super) fn emit(name: &str, store: &[u8], expected: &str) {
    assert!(store.len() < 64 * 1024, "{name} is {} bytes", store.len());
    if let Ok(dir) = std::env::var("EIN_RULE_FIXTURES") {
        std::fs::write(format!("{dir}/{name}.store"), store).unwrap();
        std::fs::write(format!("{dir}/{name}.expected"), expected).unwrap();
    }
}

/// One DOC fixture: the build is run twice and must not move, the real-verifier verdict is held.
pub(super) fn doc_fixture(name: &str, what: &str, holds: bool, build: fn() -> Store) -> Store {
    let (st, again) = (build(), build());
    assert_eq!(st.to_bytes_trimmed(), again.to_bytes_trimmed(), "{name} is not deterministic");
    let rep = report(&st, &ToyVerifier);
    assert_eq!(rep.holds(), holds, "{name}: {:?}", rep.counters);
    emit(name, &st.to_bytes_trimmed(), &expected(&st, what));
    st
}

pub(super) fn counts(st: &Store) -> crate::doc::Counters {
    report(st, &Blind).counters
}

#[test]
fn rule_fixtures_tombstones_of_both_schemes() {
    let st = doc_fixture("tomb-sha", "A live tombstone, FLAG_SHA256: nonce erased, kind bit set; a FORGOTTEN lists it.", true, || tombstoned(false));
    let c = counts(&st);
    assert_eq!((c.records, c.chained, c.legacy, c.redacted, c.broken, c.declared, c.tombs_foreign), (4, 3, 0, 1, 0, 1, 0), "{c:?}");
    let st = doc_fixture("tomb-legacy", "A live tombstone of a legacy record: no nonce, the kind byte's high bit is the mark.", true, || tombstoned(true));
    let c = counts(&st);
    assert_eq!((c.records, c.chained, c.legacy, c.redacted, c.broken, c.declared, c.tombs_foreign), (4, 1, 2, 1, 0, 1, 0), "{c:?}");
}

#[test]
fn rule_fixtures_two_cuts() {
    let st = doc_fixture("seg2", "Two cuts: the older segment's TRUNCATE left the file, the chain hash in the newest covers it.", true, two_cuts);
    let c = counts(&st);
    assert_eq!((c.records, c.chained, c.broken, c.segments, c.cut), (3, 3, 0, 2, 6), "{c:?}");
    let bodies = EvLog::walk_until_at(&st, Doc::open(&st).unwrap().log, |_| false);
    assert_eq!(bodies.iter().filter(|r| FixIds.body(r.flags, &r.payload).starts_with(b"ein-truncate/1")).count(), 1, "only the newest TRUNCATE stays");
}

/// The twin of `rule_fixtures_two_cuts`: the same file, one count moved, refused by name. The
/// status line still reads: 75 is the verify line's answer, as `cut-undeclared` has it.
#[test]
fn rule_fixtures_a_forged_older_segment_is_refused() {
    let st = forged_older_segment();
    assert_eq!(st.to_bytes_trimmed(), forged_older_segment().to_bytes_trimmed(), "seg2-forged is not deterministic");
    let root = read_segs(&st, Doc::open(&st).unwrap().seg, 8).unwrap()[0].root;
    assert_eq!(verify(&st, &FixIds, &Blind, Trust::Genesis).err(), Some(DocError::CutUndeclared { root }));
    let what = "# seg2 with the older segment's redacted moved, crc re-sealed: only the chain hash can see it.\n# Written by doc/spec/fixture_tests.rs (W-MG-FIX), EIN_RULE_FIXTURES=<dir>; deterministic.\n";
    let n = EvLog::walk_until_at(&st, Doc::open(&st).unwrap().log, |_| false).len();
    emit("seg2-forged", &st.to_bytes_trimmed(), &format!("{what}doc_n={n}\ndoc_root={}\ndoc_verify=status=75\n", log_fold(&st)));
}

#[test]
fn rule_fixtures_forged_tail() {
    let st = doc_fixture("forged-tail", "A fourth record past the last seal: every id chains, no signed tip reaches it.", false, forged_tail);
    let c = counts(&st);
    assert_eq!((c.records, c.chained, c.broken, c.unsealed), (4, 4, 0, 1), "{c:?}");
}

#[test]
fn rule_fixtures_keys_disagree() {
    let st = doc_fixture("keys-disagree", "A signed REVOKE of B, the table's revoked_at zeroed and re-sealed under it.", false, keys_disagree);
    let c = counts(&st);
    assert_eq!((c.records, c.chained, c.broken, c.keys_disagree, c.unsealed), (5, 5, 0, 1, 0), "{c:?}");
}
