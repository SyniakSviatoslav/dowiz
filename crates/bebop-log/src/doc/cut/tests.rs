//! W-MG-M3/M4/M3b kill tests (`doc/cut/tests.rs`; the 300-line gate excludes `tests.rs` by name), through the toy seams of `doc/tests.rs`: a cut is declared or the
//! file is refused by name; what is kept is kept byte for byte; the archive recomputes the
//! root; a revoked key declares nothing; law 9 balances across a cut.

use crate::doc::anch::{check_anchors, AnchorCounts, NoAdapters, Verdict, KIND_PLATFORM_TSA};
use crate::doc::keys::KeyOp;
use crate::doc::seg::{forgotten_body, parse_truncate, SCHEME_CHAINED, SCHEME_LEGACY};
use crate::doc::tests::{doc_with, key_of, mix, toy_mark, Dev, ToyIds, ToySigner, ToyTsa, ToyVerifier};
use crate::doc::verify::{verify, Report, Trust};
use crate::doc::write::{stage_doc, Refs};
use crate::doc::{Doc, DocError, Ids, Verifier};
use crate::{EvLog, Record, Store};

fn rep(st: &Store) -> Result<Report, DocError> {
    verify(st, &ToyIds, &ToyVerifier, Trust::Genesis)
}

fn oldest_first(st: &Store) -> Vec<Record> {
    let mut v = EvLog::walk_until_at(st, Doc::open(st).unwrap().log, |_| false);
    v.reverse();
    v
}

/// Redact the record at oldest-first position `k` in place: kind's high bit, id kept.
fn redact_in_place(st: &mut Store, k: usize) {
    let mut objs = EvLog::objs_at(st, Doc::open(st).unwrap().log);
    objs.reverse();
    let o = objs[k];
    st.put_cell(o, 16, st.get(o, 16) | 0x80);
    st.seal(o);
}

/// THE CUT, DECLARED: six sealed commits, the oldest four cut under a root, the TRUNCATE
/// committed by the owner. `verify` holds, the kept records are the old ones byte for byte,
/// and the archive (the old store) recomputes the segment's root from its ids.
#[test]
fn a_cut_declared_by_a_signed_truncate_verifies_and_the_archive_recomputes_the_root() {
    let (old, mut owner) = doc_with(6);
    let before = oldest_first(&old);
    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    let seg = Doc::cut_into(&old, &mut fresh, 2, &ToyIds).unwrap();
    assert_eq!((seg.first, seg.last, seg.count(), seg.prev_root), (0, 3, 4, [0u8; 32]));
    owner.commit(&mut fresh, &seg.truncate_body(), None).unwrap();

    let r = rep(&fresh).unwrap();
    let c = r.counters;
    assert!(r.holds(), "{c:?}");
    assert_eq!((c.records, c.chained, c.seals_ok, c.seals_bad, c.broken), (3, 3, 3, 0, 0), "{c:?}");
    assert_eq!((c.segments, c.cut, c.cut_redacted, c.declared), (1, 4, 0, 0), "{c:?}");
    assert_eq!((r.heads.len(), r.heads[0].seq, r.heads[0].tip), (1, 7, owner.tip));

    let after = oldest_first(&fresh);
    assert_eq!(&after[..2], &before[4..], "the kept records are the old ones, byte for byte");
    assert_eq!(parse_truncate(&after[2].payload).map(|t| t.root), Some(seg.root), "the TRUNCATE names the root");

    // The archive audit: the old file still verifies, and its first four ids climb to the root.
    assert!(rep(&old).unwrap().holds());
    let ids: Vec<[u8; 32]> = before[..4].iter().map(|r| r.id).collect();
    assert_eq!(ToyIds.tree_root(&ids), Some(seg.root));
    assert_ne!(ToyIds.tree_root(&ids[..3]), Some(seg.root), "one record fewer is another root");

    // And the bytes survive the trip.
    let back = Store::from_bytes(&fresh.to_bytes_trimmed());
    assert_eq!(rep(&back).unwrap().counters, c);
}

/// KILL (M3): the same cut WITHOUT the TRUNCATE -- a sealed segment dropped -- is refused by
/// name, with the root it would have needed. The twin is the test above.
#[test]
fn a_dropped_segment_without_a_truncate_is_refused_by_name() {
    let (old, _) = doc_with(6);
    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    let seg = Doc::cut_into(&old, &mut fresh, 2, &ToyIds).unwrap();
    assert_eq!(rep(&fresh), Err(DocError::CutUndeclared { root: seg.root }));
}

/// A second cut chains the segments (`prev_root` = the first root) and may take the first
/// TRUNCATE record with it: the newest segment is signed directly, the older one through
/// the newest one's `prev_root`. Everything leaves the live log; the file still verifies.
#[test]
fn a_second_cut_chains_the_segments_and_may_take_the_first_truncate() {
    let (old, mut owner) = doc_with(6);
    let mut mid = Store::create_bytes(1 << 20).unwrap();
    let s1 = Doc::cut_into(&old, &mut mid, 2, &ToyIds).unwrap();
    owner.commit(&mut mid, &s1.truncate_body(), None).unwrap();
    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    let s2 = Doc::cut_into(&mid, &mut fresh, 0, &ToyIds).unwrap();
    assert_eq!((s2.first, s2.last, s2.prev_root), (4, 6, s1.root));
    assert_eq!((s2.frontier.len(), s2.frontier[0].seq, s2.frontier[0].tip), (1, 7, owner.tip), "the frontier is the TRUNCATE itself");
    owner.commit(&mut fresh, &s2.truncate_body(), None).unwrap();
    let r = rep(&fresh).unwrap();
    assert!(r.holds(), "{:?}", r.counters);
    assert_eq!((r.counters.records, r.counters.segments, r.counters.cut, r.counters.seals_ok), (1, 2, 7, 1), "{:?}", r.counters);

    // A rewritten older segment is NOT caught here (its root is only the newer SEG's unsigned
    // `prev_root`), which `declare.rs` says; the newest one is: one bit of its root and the
    // TRUNCATE no longer names it.
    let mut st = Store::from_bytes(&fresh.to_bytes_trimmed());
    let seg_at = Doc::open(&st).unwrap().seg.unwrap();
    st.put_cell(seg_at, 3, st.get(seg_at, 3) ^ 1);
    st.seal(seg_at);
    assert!(matches!(rep(&st), Err(DocError::CutUndeclared { .. })));
}

/// What a cut refuses, each beside the cut it allows: through an ADMIT a key depends on,
/// nothing to cut, no tree to name the root, a DOC that names a TREE.
#[test]
fn a_cut_refuses_what_it_cannot_carry() {
    let (mut st, mut owner) = doc_with(2);
    let mut b = Dev::new(ToySigner { id: [0xB2; 32] });
    let admit = KeyOp::Admit(key_of(&b.s));
    owner.commit(&mut st, &admit.body(), Some(&admit)).unwrap();
    b.commit(&mut st, b"b-1", None).unwrap();
    b.commit(&mut st, b"b-2", None).unwrap();
    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    assert_eq!(Doc::cut_into(&st, &mut fresh, 2, &ToyIds), Err(DocError::CutsAdmit(b.s.id)));
    assert_eq!(Doc::cut_into(&st, &mut fresh, 5, &ToyIds), Err(DocError::NothingToCut));
    struct NoTree;
    impl Ids for NoTree {
        fn chained(&self, prev: &[u8; 32], payload: &[u8]) -> [u8; 32] {
            ToyIds.chained(prev, payload)
        }
    }
    assert_eq!(Doc::cut_into(&st, &mut fresh, 3, &NoTree), Err(DocError::NoTree));
    // The twin: up to the ADMIT is allowed, and B's seals read their prev_tip as before.
    let seg = Doc::cut_into(&st, &mut fresh, 3, &ToyIds).unwrap();
    owner.commit(&mut fresh, &seg.truncate_body(), None).unwrap();
    let r = rep(&fresh).unwrap();
    assert!(r.holds(), "{:?}", r.counters);
    assert_eq!((r.counters.records, r.counters.seals_ok, r.admitted), (4, 4, 2), "{:?}", r.counters);

    // A DOC naming a TREE is the tree layer's to move; the counters never looked at it.
    let (mut st, _) = doc_with(1);
    let view = Doc::open(&st).unwrap();
    let plain = rep(&st).unwrap().counters;
    let mut tx = st.begin().unwrap();
    let tree = st.alloc(&mut tx, 1, 7).unwrap();
    st.put_cell(tree, 0, 0);
    st.seal(tree);
    let d = stage_doc(&mut st, &mut tx, view.flags, Refs { log: view.log, tree: Some(tree), heads: view.heads, keys: view.keys, ..Refs::default() }).unwrap();
    st.commit_bytes(&tx, d);
    assert_eq!(rep(&st).unwrap().counters, plain, "the log's counters do not depend on the tree");
    assert_eq!(Doc::cut_into(&st, &mut fresh, 0, &ToyIds), Err(DocError::Carries("TREE")));
}

/// KILL (M4): a FORGOTTEN declares only while its key is admitted and valid at that seq. B
/// redacts its own record and declares it: `redacted == declared`, nothing broken, holds.
/// Then the owner revokes B after seq 2 -- B's declaration was seq 3 -- and the file no
/// longer holds: `declared` falls to 0 while `redacted` stays 1 (counted, not broken). The
/// writer side: a revoked B cannot commit another declaration at all.
#[test]
fn a_forgotten_by_a_revoked_key_declares_nothing() {
    let (mut st, mut owner) = doc_with(0);
    let mut b = Dev::new(ToySigner { id: [0xB2; 32] });
    let admit = KeyOp::Admit(key_of(&b.s));
    owner.commit(&mut st, &admit.body(), Some(&admit)).unwrap();
    b.commit(&mut st, b"contact: +355 69 000 0001", None).unwrap();
    b.commit(&mut st, b"b-2", None).unwrap();
    let undeclared = rep(&st).unwrap();
    assert!(undeclared.holds(), "control: {:?}", undeclared.counters);
    redact_in_place(&mut st, 1);
    let r = rep(&st).unwrap();
    assert_eq!((r.counters.redacted, r.counters.broken, r.counters.declared), (1, 0, 0), "{:?}", r.counters);
    assert!(!r.holds(), "an undeclared tombstone does not hold");

    b.commit(&mut st, &forgotten_body(1, &[0x5B; 32]), None).unwrap();
    let r = rep(&st).unwrap();
    assert_eq!((r.counters.redacted, r.counters.broken, r.counters.declared, r.counters.seals_ok), (1, 0, 1, 4), "{:?}", r.counters);
    assert!(r.holds(), "declared: {:?}", r.counters);

    let revoke = KeyOp::Revoke { actor_id: b.s.id, after_seq: 2 };
    owner.commit(&mut st, &revoke.body(), Some(&revoke)).unwrap();
    let r = rep(&st).unwrap();
    assert_eq!((r.counters.redacted, r.counters.broken, r.counters.declared, r.counters.seals_bad), (1, 0, 0, 1), "{:?}", r.counters);
    assert!(!r.holds());
    assert_eq!(b.commit(&mut st, &forgotten_body(1, &[0x5B; 32]), None), Err(DocError::Revoked { after_seq: 2, seq: 4 }));
}

/// LAW 9 ACROSS A CUT: the tombstone leaves under the segment, its declaration stays live;
/// `redacted + cut_redacted == declared` still. The segment's counts are the TRUNCATE's
/// signed claim: one changed count and the TRUNCATE names nothing.
#[test]
fn law_9_balances_across_a_cut_and_the_counts_are_signed() {
    let mut st = Store::create_bytes(1 << 20).unwrap();
    let mut owner = Dev::new(ToySigner { id: [0xA1; 32] });
    let mut b = Dev::new(ToySigner { id: [0xB2; 32] });
    Doc::init_bytes(&mut st, 0, &[key_of(&owner.s), key_of(&b.s)]).unwrap();
    b.commit(&mut st, b"contact: +355 69 000 0002", None).unwrap();
    b.commit(&mut st, b"b-2", None).unwrap();
    redact_in_place(&mut st, 0);
    b.commit(&mut st, &forgotten_body(1, &[0x5B; 32]), None).unwrap();
    owner.commit(&mut st, b"owner-1", None).unwrap();
    assert!(rep(&st).unwrap().holds());
    // THE HOLDS CONJUNCT ITSELF (M3b): a second tombstone, undeclared, in the part that stays.
    // Nothing is broken and every seal is ok; only law 9 fails, and `holds` must say so.
    redact_in_place(&mut st, 3);
    let r = rep(&st).unwrap();
    assert_eq!((r.counters.redacted, r.counters.declared, r.counters.broken, r.counters.seals_bad), (2, 1, 0, 0), "{:?}", r.counters);
    assert!(!r.holds(), "redacted 2, declared 1: law 9 alone must fail holds");

    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    let seg = Doc::cut_into(&st, &mut fresh, 2, &ToyIds).unwrap();
    assert_eq!((seg.redacted, seg.declared, seg.frontier.len()), (1, 0, 1));
    owner.commit(&mut fresh, &seg.truncate_body(), None).unwrap();
    let r = rep(&fresh).unwrap();
    assert_eq!((r.counters.redacted, r.counters.cut_redacted, r.counters.declared, r.counters.broken), (1, 1, 1, 0), "{:?}", r.counters);
    assert!(!r.holds(), "the cut tombstone is declared, the live one is not: {:?}", r.counters);
    b.commit(&mut fresh, &forgotten_body(1, &[0x5C; 32]), None).unwrap();
    let r = rep(&fresh).unwrap();
    assert_eq!((r.counters.redacted, r.counters.cut_redacted, r.counters.declared, r.counters.broken), (1, 1, 2, 0), "{:?}", r.counters);
    assert!(r.holds(), "{:?}", r.counters);

    let seg_at = Doc::open(&fresh).unwrap().seg.unwrap();
    fresh.put_cell(seg_at, 9, 0);
    fresh.seal(seg_at);
    assert_eq!(rep(&fresh), Err(DocError::CutUndeclared { root: seg.root }), "a changed count is not what the TRUNCATE signed");
}

/// SCALE: two thousand sealed commits, nineteen hundred cut. The kept hundred read back
/// byte for byte, the archive recomputes the root, both files hold. (The plan's number is
/// 1e5; the toy signer makes this a store-size question, not a time one, and 2 000 is what
/// a 4 MB store holds with its dead roots.)
#[test]
fn a_cut_of_two_thousand_keeps_the_rest_byte_for_byte() {
    let mut old = Store::create_bytes(4 << 20).unwrap();
    let mut owner = Dev::new(ToySigner { id: [0xA1; 32] });
    Doc::init_bytes(&mut old, 0, &[key_of(&owner.s)]).unwrap();
    for i in 0..2000 {
        owner.commit(&mut old, format!("order-{i}:{}", i * 7919 % 1000).as_bytes(), None).unwrap();
    }
    let before = oldest_first(&old);
    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    let seg = Doc::cut_into(&old, &mut fresh, 100, &ToyIds).unwrap();
    owner.commit(&mut fresh, &seg.truncate_body(), None).unwrap();
    let r = rep(&fresh).unwrap();
    assert!(r.holds(), "{:?}", r.counters);
    assert_eq!((r.counters.records, r.counters.cut, r.counters.seals_ok), (101, 1900, 101), "{:?}", r.counters);
    let after = oldest_first(&fresh);
    assert_eq!(&after[..100], &before[1900..]);
    let ids: Vec<[u8; 32]> = before[..1900].iter().map(|r| r.id).collect();
    assert_eq!(ToyIds.tree_root(&ids), Some(seg.root));
    assert!(rep(&old).unwrap().holds());
    println!("M3 cut: old {} B, fresh {} B", old.to_bytes_trimmed().len(), fresh.to_bytes_trimmed().len());
}

/// A cut is history: an anchor on the cut DOC and a copy of it (what a grow does) must carry the
/// segment, or a file that dropped records would read as one that never had them.
#[test]
fn an_anchor_and_a_copy_keep_the_segments_of_a_cut_doc() {
    let (old, mut owner) = doc_with(6);
    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    let seg = Doc::cut_into(&old, &mut fresh, 2, &ToyIds).unwrap();
    owner.commit(&mut fresh, &seg.truncate_body(), None).unwrap();
    let want = rep(&fresh).unwrap().counters;
    assert_eq!((want.segments, want.cut), (1, 4), "{want:?}");

    let over = oldest_first(&fresh)[0].id;
    Doc::anchor_bytes(&mut fresh, 1, &over, 7, b"proof").unwrap();
    let anchored = rep(&fresh).unwrap();
    assert!(anchored.holds(), "{:?}", anchored.counters);
    assert_eq!((anchored.counters.segments, anchored.counters.cut), (1, 4), "the anchor dropped the segment");

    let mut copy = Store::create_bytes(1 << 20).unwrap();
    Doc::copy_bytes(&fresh, &mut copy).unwrap();
    let copied = rep(&copy).unwrap();
    assert!(copied.holds(), "{:?}", copied.counters);
    assert_eq!((copied.counters.segments, copied.counters.cut, copied.counters.records), (1, 4, want.records), "the copy dropped the segment");
}

/// AN ANCHORED FILE IS STILL CUTTABLE (M3b, the first main-goal property must not exclude the
/// second). Two anchors: one over a record the cut takes, one over a kept record. The cut
/// carries both. The kept one checks as before. The cut one is `under_cut`: not bad (its id
/// may well have been in the log), not ok (the file cannot show it), `Unchecked` naming the
/// segments -- and the ARCHIVE places it: its `over` is among the cut ids whose tree is the
/// segment's root. A forged proof over a cut id is still bad.
#[test]
fn a_cut_carries_the_anchors_and_an_anchor_over_a_cut_record_is_the_archives_to_place() {
    let (mut old, mut owner) = doc_with(4);
    let early = oldest_first(&old)[1].id;
    Doc::anchor_bytes(&mut old, KIND_PLATFORM_TSA, &early, 1_000, &toy_mark(&early, 1_000)).unwrap();
    owner.commit(&mut old, b"event-4", None).unwrap();
    let late = owner.tip;
    Doc::anchor_bytes(&mut old, KIND_PLATFORM_TSA, &late, 2_000, &toy_mark(&late, 2_000)).unwrap();
    let before = oldest_first(&old);
    assert_eq!(AnchorCounts::of(&check_anchors(&old, &ToyTsa).unwrap()), AnchorCounts { anchors: 2, ok: 2, bad: 0, unchecked: 0, cut: 0 }, "control: both check in the uncut file");

    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    let seg = Doc::cut_into(&old, &mut fresh, 2, &ToyIds).unwrap();
    owner.commit(&mut fresh, &seg.truncate_body(), None).unwrap();
    assert!(rep(&fresh).unwrap().holds());
    let checked = check_anchors(&fresh, &ToyTsa).unwrap();
    assert_eq!(checked.len(), 2, "the cut dropped an anchor: {checked:?}");
    assert_eq!((checked[0].anchor.over, checked[0].names_record, checked[0].under_cut), (late, true, false));
    assert!(matches!(checked[0].verdict, Verdict::Ok { .. }), "{:?}", checked[0].verdict);
    assert_eq!((checked[1].anchor.over, checked[1].names_record, checked[1].under_cut), (early, false, true));
    assert!(matches!(&checked[1].verdict, Verdict::Unchecked(w) if w.contains("1 segment")), "{:?}", checked[1].verdict);
    assert_eq!(AnchorCounts::of(&checked), AnchorCounts { anchors: 2, ok: 1, bad: 0, unchecked: 1, cut: 1 }, "{checked:?}");
    let blind = check_anchors(&fresh, &NoAdapters).unwrap();
    assert_eq!(AnchorCounts::of(&blind), AnchorCounts { anchors: 2, ok: 0, bad: 0, unchecked: 2, cut: 1 });

    // The archive places the early anchor: `over` is a cut id, and the cut ids climb to the root.
    let ids: Vec<[u8; 32]> = before[..3].iter().map(|r| r.id).collect();
    assert!(ids.contains(&early));
    assert_eq!(ToyIds.tree_root(&ids), Some(seg.root));

    // RED twin: one bit of the early anchor's proof, re-sealed -- a forgery over a cut id is bad.
    let mut st = Store::from_bytes(&fresh.to_bytes_trimmed());
    let a = checked[1].anchor.at;
    st.put_cell(a, 9, st.get(a, 9) ^ 1);
    st.seal(a);
    let forged = check_anchors(&st, &ToyTsa).unwrap();
    assert!(matches!(&forged[1].verdict, Verdict::Bad(_)), "{:?}", forged[1].verdict);
    assert_eq!(AnchorCounts::of(&forged), AnchorCounts { anchors: 2, ok: 1, bad: 1, unchecked: 0, cut: 1 });
}

/// M0'S FORWARD-ONCE RULE SURVIVES A CUT (M3b). Four chained records, all four cut: the frontier
/// row says the device had moved to chained ids. A record with a LEGACY id committed after
/// the cut is broken, exactly as it is in the uncut file (the control). The twin: a device
/// that never chained -- three legacy records, two cut -- carries `SCHEME_LEGACY`, and its
/// legacy record after the cut counts legacy.
#[test]
fn a_legacy_id_after_a_cut_of_chained_records_is_broken() {
    fn legacy_commit(st: &mut Store, d: &mut Dev<ToySigner>, payload: &[u8]) {
        d.seq += 1;
        let r = Record { id: mix(&[b"l", payload]), prev: d.tip, actor_pubkey: d.s.id, actor_seq: d.seq, flags: 0, payload: payload.to_vec() };
        Doc::commit_bytes(st, &r, &d.s, 1_700_000_000_000 + d.seq as i64, None).unwrap();
        d.tip = r.id;
    }
    // CONTROL: in the uncut file the legacy id after four chained ones is broken.
    let (mut st, mut owner) = doc_with(4);
    legacy_commit(&mut st, &mut owner, b"legacy-after");
    let c = rep(&st).unwrap().counters;
    assert_eq!((c.chained, c.legacy, c.broken), (4, 0, 1), "control: {c:?}");

    // THE KILL: every record of the device leaves (keep 0), and the FIRST record after the cut
    // carries a legacy id -- before the TRUNCATE, so no kept chained record re-arms the rule;
    // only the frontier row's `scheme` can. Broken, as in the control.
    let (old, mut owner) = doc_with(4);
    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    let seg = Doc::cut_into(&old, &mut fresh, 0, &ToyIds).unwrap();
    assert_eq!((seg.frontier.len(), seg.frontier[0].scheme, seg.frontier[0].seq), (1, SCHEME_CHAINED, 4));
    legacy_commit(&mut fresh, &mut owner, b"legacy-after");
    owner.commit(&mut fresh, &seg.truncate_body(), None).unwrap();
    let c = rep(&fresh).unwrap().counters;
    assert_eq!((c.records, c.chained, c.legacy, c.broken, c.seals_ok), (2, 1, 0, 1, 2), "the cut must not reset M0's rule: {c:?}");

    // THE TWIN: a device that never moved. Its legacy record after the cut comes BEFORE the
    // TRUNCATE (which is a chained record and moves the device for good, in any file).
    let mut st = Store::create_bytes(1 << 20).unwrap();
    let mut owner = Dev::new(ToySigner { id: [0xA1; 32] });
    Doc::init_bytes(&mut st, 0, &[key_of(&owner.s)]).unwrap();
    for p in [b"l-1", b"l-2", b"l-3"] {
        legacy_commit(&mut st, &mut owner, p);
    }
    assert_eq!(rep(&st).unwrap().counters.legacy, 3);
    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    let seg = Doc::cut_into(&st, &mut fresh, 1, &ToyIds).unwrap();
    assert_eq!(seg.frontier[0].scheme, SCHEME_LEGACY);
    legacy_commit(&mut fresh, &mut owner, b"l-4");
    assert_eq!(rep(&fresh).map(|_| ()), Err(DocError::CutUndeclared { root: seg.root }), "undeclared until the TRUNCATE");
    owner.commit(&mut fresh, &seg.truncate_body(), None).unwrap();
    let c = rep(&fresh).unwrap().counters;
    assert_eq!((c.records, c.chained, c.legacy, c.broken), (3, 1, 2, 0), "a legacy device stays legacy: {c:?}");
}

/// A reader with no signature algorithm (the MIT reader) must still open an honest cut file and
/// must still refuse a dropped segment; it just never says the file holds.
#[test]
fn a_reader_without_signatures_reads_a_cut_and_refuses_a_dropped_segment() {
    struct Blind;
    impl Verifier for Blind {
        fn verify(&self, _alg: i64, _pk: &[u8], _msg: &[u8], _sig: &[u8]) -> bool {
            false
        }
        fn checks_signatures(&self) -> bool {
            false
        }
    }
    let (old, mut owner) = doc_with(6);
    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    let seg = Doc::cut_into(&old, &mut fresh, 2, &ToyIds).unwrap();
    assert_eq!(verify(&fresh, &ToyIds, &Blind, Trust::Genesis).map(|_| ()), Err(DocError::CutUndeclared { root: seg.root }));
    owner.commit(&mut fresh, &seg.truncate_body(), None).unwrap();
    let r = verify(&fresh, &ToyIds, &Blind, Trust::Genesis).unwrap();
    assert_eq!((r.counters.segments, r.counters.cut, r.counters.seals_ok), (1, 4, 0), "{:?}", r.counters);
    assert!(!r.holds(), "a count is not a proof");
}

/// REDACTION IN PLACE FOR A DOC (M3b, `Doc::redact_into`, what `SealedHub::forget` calls): the
/// copy with two payloads emptied keeps every id, `prev`, seal and anchor; `verify` counts the
/// two tombstones and does not hold until a FORGOTTEN declares them; a redaction that matches
/// nothing commits nothing.
#[test]
fn a_redacting_copy_keeps_the_ids_and_is_declared_by_a_forgotten() {
    let (mut old, mut owner) = doc_with(4);
    let tip = owner.tip;
    Doc::anchor_bytes(&mut old, KIND_PLATFORM_TSA, &tip, 1_000, &toy_mark(&tip, 1_000)).unwrap();
    let before = oldest_first(&old);
    let person = |r: &Record| -> Option<Vec<u8>> {
        (r.payload == b"event-1" || r.payload == b"event-2").then(|| {
            let mut p = r.payload.clone();
            p[0] |= 0x80;
            p.truncate(1);
            p
        })
    };
    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    assert_eq!(Doc::redact_into(&old, &mut fresh, &person).unwrap(), 2);
    let after = oldest_first(&fresh);
    assert_eq!(after.iter().map(|r| (r.id, r.prev, r.actor_seq)).collect::<Vec<_>>(), before.iter().map(|r| (r.id, r.prev, r.actor_seq)).collect::<Vec<_>>(), "every id and link kept");
    assert_eq!((after[1].payload.as_slice(), after[3].payload.as_slice()), (&[b'e' | 0x80][..], &b"event-3"[..]), "the person's payloads emptied, the rest verbatim");
    let r = rep(&fresh).unwrap();
    assert_eq!((r.counters.records, r.counters.chained, r.counters.redacted, r.counters.declared, r.counters.broken, r.counters.seals_ok), (4, 2, 2, 0, 0, 4), "{:?}", r.counters);
    assert!(!r.holds(), "two tombstones, nothing declared");
    assert_eq!(AnchorCounts::of(&check_anchors(&fresh, &ToyTsa).unwrap()), AnchorCounts { anchors: 1, ok: 1, bad: 0, unchecked: 0, cut: 0 }, "the anchor rode along");
    owner.commit(&mut fresh, &forgotten_body(2, &[0x5B; 32]), None).unwrap();
    let r = rep(&fresh).unwrap();
    assert_eq!((r.counters.redacted, r.counters.declared), (2, 2));
    assert!(r.holds(), "{:?}", r.counters);

    // Nothing to redact: nothing is committed, the fresh store has no root.
    let mut empty = Store::create_bytes(1 << 20).unwrap();
    assert_eq!(Doc::redact_into(&old, &mut empty, &|_| None).unwrap(), 0);
    assert!(matches!(Doc::open(&empty), Err(DocError::NotDoc)));
}
