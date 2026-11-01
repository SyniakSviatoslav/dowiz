//! W-MG-REVOKE kill tests, through the toy seams of `doc/tests.rs`: the revocation comes from
//! the signed records, the table is an index, and a REVOKE outlives a cut, a copy and an
//! archive. The `limit_` tests pin what the file alone cannot decide: `holds` on them is the
//! limit, written down, and whoever closes one turns its assert around.

use crate::doc::keys::{key_rows, narrower, Key, KeyOp, REVOKED_ALL};
use crate::doc::seal::read_heads;
use crate::doc::spec::SpecIn;
use crate::doc::tests::{key_of, mix, tombs_body, Dev, ToyIds, ToySigner, ToyVerifier};
use crate::doc::verify::{verify, Counters, Report, Trust};
use crate::doc::write::{stage_doc, Refs};
use crate::doc::{forgotten_body, Doc, DocError, Ids, Verifier};
use crate::{EvLog, Record, Store};

type D = Dev<ToySigner>;

fn rep(st: &Store) -> Report {
    verify(st, &ToyIds, &ToyVerifier, Trust::Genesis).unwrap()
}

fn cnt(st: &Store) -> Counters {
    rep(st).counters
}

/// A DOC whose genesis keys are one per byte of `ids`.
fn genesis<const N: usize>(ids: [u8; N]) -> (Store, [D; N]) {
    let mut st = Store::create_bytes(1 << 20).unwrap();
    let devs = ids.map(|b| Dev::new(ToySigner { id: [b; 32] }));
    let keys: Vec<Key> = devs.iter().map(|d| key_of(&d.s)).collect();
    Doc::init_bytes(&mut st, 0, &keys).unwrap();
    (st, devs)
}

fn revoke(who: &D, after_seq: i64) -> KeyOp {
    KeyOp::Revoke { actor_id: who.s.id, after_seq }
}

/// One sealed commit carrying `op`, the table moved with it.
fn op(st: &mut Store, by: &mut D, op: &KeyOp) -> Result<i64, DocError> {
    by.commit(st, &op.body(), Some(op))
}

/// One sealed commit carrying `op`'s body with the table left alone.
fn body(st: &mut Store, by: &mut D, op: &KeyOp) {
    by.commit(st, &op.body(), None).unwrap();
}

/// THE FORGER: one cell of a KEY, its crc re-sealed. Nothing else in the file moves.
fn forge_table(st: &mut Store, who: &D, revoked_at: i64) {
    let rows = key_rows(st, Doc::open(st).unwrap().keys);
    let at = rows.iter().find(|r| r.0 == who.s.id).unwrap().1;
    st.put_cell(at, 6, revoked_at);
    st.seal(at);
}

/// THE RED: before this lane `verify` read the table alone, so this forgery read as a whole
/// file, `seals_bad` 0 and `holds`. The REVOKE record still breaks B's seals past it, the
/// forger's next commit (the writer reads the table, so it goes through) is bad as well, and
/// the disagreement is named.
#[test]
fn a_zeroed_revoked_at_does_not_bring_a_key_back() {
    let (mut st, [mut a, mut b]) = genesis([0xA1, 0xB2]);
    for i in 0..3 {
        b.commit(&mut st, format!("b-{i}").as_bytes(), None).unwrap();
    }
    op(&mut st, &mut a, &revoke(&b, 1)).unwrap();
    let honest = cnt(&st);
    assert_eq!((honest.seals_ok, honest.seals_bad, honest.keys_disagree), (2, 2, 0), "control: {honest:?}");

    forge_table(&mut st, &b, 0);
    let c = cnt(&st);
    assert_eq!((c.seals_ok, c.seals_bad, c.keys_disagree), (2, 2, 1), "the table decides nothing: {c:?}");
    assert!(!rep(&st).holds());

    b.commit(&mut st, b"after-the-forgery", None).unwrap();
    let c = cnt(&st);
    assert_eq!((c.seals_ok, c.seals_bad, c.keys_disagree), (2, 3, 1), "and the forger's next seal: {c:?}");
}

/// The other way: a table that revokes with no record behind it narrows nothing, every seal
/// stays ok, and the file does not hold, because the index and the records disagree.
#[test]
fn a_table_that_revokes_without_a_record_is_a_disagreement() {
    let (mut st, [mut a, mut b]) = genesis([0xA1, 0xB2]);
    a.commit(&mut st, b"a-1", None).unwrap();
    b.commit(&mut st, b"b-1", None).unwrap();
    b.commit(&mut st, b"b-2", None).unwrap();
    assert!(rep(&st).holds(), "control");
    forge_table(&mut st, &b, 1);
    let c = cnt(&st);
    assert_eq!((c.seals_ok, c.seals_bad, c.keys_disagree), (3, 0, 1), "{c:?}");
    assert!(!rep(&st).holds());
}

/// Two REVOKEs of one key: the earliest wins in the fold and in the writer's table, in either
/// order, so a later REVOKE never widens a key. `narrower` is that rule, 0 being "none yet".
#[test]
fn the_earliest_after_seq_wins() {
    assert_eq!((narrower(0, 3), narrower(2, 5), narrower(5, 2)), (3, 2, 2));
    assert_eq!((narrower(0, 0), narrower(0, -4), narrower(REVOKED_ALL, 7), narrower(3, 0)), (REVOKED_ALL, REVOKED_ALL, REVOKED_ALL, REVOKED_ALL));
    for (one, two) in [(1, 3), (3, 1)] {
        let (mut st, [mut a, mut b]) = genesis([0xA1, 0xB2]);
        for i in 0..3 {
            b.commit(&mut st, format!("b-{i}").as_bytes(), None).unwrap();
        }
        op(&mut st, &mut a, &revoke(&b, one)).unwrap();
        op(&mut st, &mut a, &revoke(&b, two)).unwrap();
        let c = cnt(&st);
        assert_eq!((c.seals_ok, c.seals_bad, c.keys_disagree), (3, 2, 0), "{one} then {two}: {c:?}");
    }
}

/// `after_seq` 0: the table has no k for it (0 = not revoked), so `REVOKED_ALL`, and a device
/// that never sealed can be revoked before it does. Until this lane the writer stored 0 and
/// the REVOKE did nothing at all.
#[test]
fn revoke_after_zero_leaves_no_seal_valid() {
    let (mut st, [mut a, mut b]) = genesis([0xA1, 0xB2]);
    op(&mut st, &mut a, &revoke(&b, 0)).unwrap();
    let c = cnt(&st);
    assert_eq!((c.seals_ok, c.seals_bad, c.keys_disagree), (1, 0, 0), "{c:?}");
    assert_eq!(b.commit(&mut st, b"b-1", None), Err(DocError::Revoked { after_seq: REVOKED_ALL, seq: 1 }));
}

/// A seq past `i64::MAX` wrapped to negative and passed every `revoked_at`: the record's seq is
/// the key holder's to write, so that was a way past any revocation.
#[test]
fn a_seq_past_i64_max_does_not_wrap_past_a_revocation() {
    let k = Key { actor_id: [1; 32], alg: 1, valid_from: 0, revoked_at: 2, admit_rec: None, pk: Vec::new() };
    assert!(k.valid_at(2));
    assert!(!k.valid_at(3));
    assert!(!k.valid_at(1 << 63), "seq 2^63 as i64 is negative");
    let all = Key { revoked_at: REVOKED_ALL, ..k.clone() };
    assert!(!all.valid_at(1) && !all.valid_at(u64::MAX));
    assert!(Key { revoked_at: 0, ..k }.valid_at(u64::MAX), "not revoked");
}

/// What is NOT effective, each beside a file that holds without it: a REVOKE no seal covers,
/// one whose id does not chain, one by a key this verifier does not admit.
#[test]
fn a_revoke_is_effective_by_the_rule_of_declare_rs() {
    let (mut st, [mut a, mut b]) = genesis([0xA1, 0xB2]);
    b.commit(&mut st, b"b-1", None).unwrap();
    a.commit(&mut st, b"a-1", None).unwrap();

    // Unsealed: appended, no seal, HEADS unmoved.
    let r = a.next(&revoke(&b, 0).body());
    let view = Doc::open(&st).unwrap();
    let mut tx = st.begin().unwrap();
    let log = EvLog::stage_append_in(&mut st, &mut tx, view.log.unwrap(), &r, None).unwrap();
    let d = stage_doc(&mut st, &mut tx, view.flags, Refs { log: Some(log), heads: view.heads, keys: view.keys, ..Refs::default() }).unwrap();
    st.commit_bytes(&tx, d);
    let c = cnt(&st);
    assert_eq!((c.records, c.seals_ok, c.seals_bad, c.keys_disagree, c.unsealed), (3, 2, 0, 0, 1), "unsealed declares nothing: {c:?}");
    assert!(!rep(&st).holds(), "and no record nobody signed for holds (W-MG-SEAL): {c:?}");

    // Sealed, its id not chained (a legacy id): the seal is good and the REVOKE declares nothing.
    let (mut st, [mut a, mut b]) = genesis([0xA1, 0xB2]);
    b.commit(&mut st, b"b-1", None).unwrap();
    let payload = revoke(&b, 0).body();
    let mut r = a.next(&payload);
    r.id = mix(&[b"l", &payload]);
    Doc::commit_bytes(&mut st, &r, &a.s, 1, None).unwrap();
    let c = cnt(&st);
    assert_eq!((c.legacy, c.seals_ok, c.seals_bad, c.keys_disagree), (1, 2, 0, 0), "an unchained record declares nothing: {c:?}");

    // By a key the verifier does not admit: B's REVOKE of A, read under A's trust alone.
    let (mut st, [mut a, mut b]) = genesis([0xA1, 0xB2]);
    a.commit(&mut st, b"a-1", None).unwrap();
    body(&mut st, &mut b, &revoke(&a, 0));
    let only = [a.s.id];
    let c = verify(&st, &ToyIds, &ToyVerifier, Trust::Only(&only)).unwrap().counters;
    assert_eq!((c.seals_ok, c.seals_bad, c.keys_disagree), (1, 1, 0), "an unadmitted key revokes nobody: {c:?}");
}

/// ANY key valid at its seq may revoke any key, the owner included: there are no roles. B,
/// an admitted device with no special standing, ends the owner, and the owner's seals past
/// the REVOKE are bad. The rule stated as a test, so changing it is a decision.
#[test]
fn any_valid_key_may_revoke_any_key() {
    let (mut st, [mut a, mut b]) = genesis([0xA1, 0xB2]);
    a.commit(&mut st, b"a-1", None).unwrap();
    a.commit(&mut st, b"a-2", None).unwrap();
    op(&mut st, &mut b, &revoke(&a, 1)).unwrap();
    let c = cnt(&st);
    assert_eq!((c.seals_ok, c.seals_bad, c.keys_disagree, c.revokes_unsettled), (2, 1, 0, 0), "{c:?}");
}

/// A REVOKED KEY'S OWN LATE REVOKE DECLARES NOTHING, though it was written before the REVOKE
/// that ended the key: it sits past `after_seq`. C's second seal is bad while only B's REVOKE
/// stands and ok once B's is void; the table the writer left says otherwise, and is counted.
#[test]
fn a_revoked_keys_late_revoke_declares_nothing() {
    let (mut st, [mut a, mut b, mut c]) = genesis([0xA1, 0xB2, 0xC3]);
    c.commit(&mut st, b"c-1", None).unwrap();
    c.commit(&mut st, b"c-2", None).unwrap();
    b.commit(&mut st, b"b-1", None).unwrap();
    op(&mut st, &mut b, &revoke(&c, 1)).unwrap();
    let n = cnt(&st);
    assert_eq!((n.seals_ok, n.seals_bad), (3, 1), "control: B's REVOKE stands: {n:?}");
    op(&mut st, &mut a, &revoke(&b, 1)).unwrap();
    let n = cnt(&st);
    assert_eq!((n.seals_ok, n.seals_bad, n.keys_disagree, n.revokes_unsettled), (4, 1, 1, 0), "only B's REVOKE seal is bad: {n:?}");
}

/// THE FIXPOINT: a REVOKE takes the seal that carried an ADMIT, the ADMIT takes the key, and
/// that key's REVOKE of a fourth falls with it, so D's seal comes back. No single pass over
/// the log gets there, and the file order is not what gets it there.
#[test]
fn a_revoke_feeds_admission_and_the_revokes_it_carried() {
    let (mut st, [mut a, mut d]) = genesis([0xA1, 0xD4]);
    let mut b = Dev::new(ToySigner { id: [0xB2; 32] });
    let mut c = Dev::new(ToySigner { id: [0xC3; 32] });
    d.commit(&mut st, b"d-1", None).unwrap();
    op(&mut st, &mut a, &KeyOp::Admit(key_of(&b.s))).unwrap();
    b.commit(&mut st, b"b-1", None).unwrap();
    op(&mut st, &mut b, &KeyOp::Admit(key_of(&c.s))).unwrap();
    body(&mut st, &mut c, &revoke(&d, 0));
    let r = rep(&st);
    assert_eq!((r.counters.seals_ok, r.counters.seals_bad, r.admitted), (4, 1, 4), "control: D's seal is past C's REVOKE: {r:?}");

    op(&mut st, &mut a, &revoke(&b, 1)).unwrap();
    let r = rep(&st);
    assert_eq!((r.counters.seals_ok, r.counters.seals_bad, r.admitted), (4, 2, 3), "B's second seal and C's are bad, D's holds: {r:?}");
    assert_eq!((r.counters.keys_disagree, r.counters.revokes_unsettled), (0, 0), "{r:?}");
}

/// TWO KEYS REVOKED EACH OTHER back past their REVOKEs: no reading settles it (each is void
/// exactly when the other stands). Both stay in force, the narrowest reading, both are
/// counted, the file does not hold, and neither file order wins: the order across
/// devices is signed by nobody.
#[test]
fn a_mutual_revoke_is_unsettled_in_either_file_order() {
    let build = |a_first: bool| {
        let (mut st, [mut a, mut b]) = genesis([0xA1, 0xB2]);
        a.commit(&mut st, b"a-1", None).unwrap();
        b.commit(&mut st, b"b-1", None).unwrap();
        let (ra, rb) = (revoke(&b, 0), revoke(&a, 0));
        if a_first {
            body(&mut st, &mut a, &ra);
            body(&mut st, &mut b, &rb);
        } else {
            body(&mut st, &mut b, &rb);
            body(&mut st, &mut a, &ra);
        }
        st
    };
    let (one, two) = (build(true), build(false));
    let c = cnt(&one);
    assert_eq!((c.seals_ok, c.seals_bad, c.revokes_unsettled), (0, 4, 2), "{c:?}");
    assert!(!rep(&one).holds());
    assert_eq!(cnt(&two), c, "the file's order across devices decides nothing");
}

/// A key's REVOKE of itself past its own seal is the same contradiction with one party: the
/// narrowing stands, a device may declare itself stolen, and it is counted. Forward (`after` =
/// its own seq) nothing contradicts: the device retires and the file holds.
#[test]
fn a_key_revoking_itself() {
    let (mut st, [mut a]) = genesis([0xA1]);
    a.commit(&mut st, b"a-1", None).unwrap();
    let back = revoke(&a, 1);
    op(&mut st, &mut a, &back).unwrap();
    let c = cnt(&st);
    assert_eq!((c.seals_ok, c.seals_bad, c.revokes_unsettled, c.keys_disagree), (1, 1, 1, 0), "{c:?}");

    let (mut st, [mut a]) = genesis([0xA1]);
    a.commit(&mut st, b"a-1", None).unwrap();
    let retire = revoke(&a, 2);
    op(&mut st, &mut a, &retire).unwrap();
    assert!(rep(&st).holds(), "{:?}", cnt(&st));
    assert_eq!(a.commit(&mut st, b"a-3", None), Err(DocError::Revoked { after_seq: 2, seq: 3 }));
}

/// A REVOKE is final: ADMIT does not bring the key back, or the table would say 0 over a
/// record that still says otherwise.
#[test]
fn a_revoked_key_is_not_admitted_again() {
    let (mut st, [mut a, b]) = genesis([0xA1, 0xB2]);
    op(&mut st, &mut a, &revoke(&b, 1)).unwrap();
    let again = KeyOp::Admit(key_of(&b.s));
    assert_eq!(op(&mut st, &mut a, &again), Err(DocError::Readmit(b.s.id)));
    assert!(rep(&st).holds(), "the refusal wrote nothing: {:?}", cnt(&st));
}

/// THE CUT: a REVOKE under it is refused by name; up to it is allowed, and the revocation
/// stands on the cut file, against a zeroed table too.
#[test]
fn a_cut_keeps_the_revoke() {
    let (mut st, [mut a, mut b]) = genesis([0xA1, 0xB2]);
    b.commit(&mut st, b"b-1", None).unwrap();
    op(&mut st, &mut a, &revoke(&b, 1)).unwrap();
    a.commit(&mut st, b"a-2", None).unwrap();
    a.commit(&mut st, b"a-3", None).unwrap();

    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    assert_eq!(Doc::cut_into(&st, &mut fresh, 2, &ToyIds), Err(DocError::TakesRevoke(b.s.id)));
    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    let seg = Doc::cut_into(&st, &mut fresh, 3, &ToyIds).unwrap();
    a.commit(&mut fresh, &seg.truncate_body(), None).unwrap();
    assert!(rep(&fresh).holds(), "{:?}", cnt(&fresh));
    assert_eq!(b.commit(&mut fresh, b"b-2", None), Err(DocError::Revoked { after_seq: 1, seq: 2 }));

    forge_table(&mut fresh, &b, 0);
    b.commit(&mut fresh, b"b-2", None).unwrap();
    let c = cnt(&fresh);
    assert_eq!((c.seals_bad, c.keys_disagree), (1, 1), "the fold reads the kept record: {c:?}");
}

/// A COPY AND AN ARCHIVE are verbatim, so the verdict travels: neither launders a forged table
/// into a whole file. A redaction that would tombstone a REVOKE is refused and commits nothing.
#[test]
fn a_copy_an_archive_and_a_redaction_keep_the_revoke() {
    let (mut st, [mut a, mut b]) = genesis([0xA1, 0xB2]);
    b.commit(&mut st, b"b-1", None).unwrap();
    b.commit(&mut st, b"b-2", None).unwrap();
    op(&mut st, &mut a, &revoke(&b, 1)).unwrap();
    forge_table(&mut st, &b, 0);
    let want = cnt(&st);
    assert_eq!((want.seals_bad, want.keys_disagree), (1, 1), "control: {want:?}");

    let mut copy = Store::create_bytes(1 << 20).unwrap();
    Doc::copy_bytes(&st, &mut copy).unwrap();
    assert_eq!(cnt(&copy), want);

    let spec = SpecIn { wasm: b"\0asm", wasm_sha256: [0; 32], text: b"text", abi: 1 };
    let mut archive = Store::create_bytes(1 << 20).unwrap();
    Doc::archive_bytes(&st, &mut archive, &spec).unwrap();
    assert_eq!(cnt(&archive), want);

    let is_revoke = revoke(&b, 1).body();
    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    let rule = |r: &Record| (r.payload == is_revoke).then(|| vec![0x80]);
    assert_eq!(Doc::redact_into(&st, &mut fresh, &ToyIds, &rule), Err(DocError::TakesRevoke(b.s.id)));
    assert!(fresh.root().is_none());
}

/// A reader with no signature algorithm judges no seal: a REVOKE counts on its chain alone,
/// `keys_disagree` says what the records CLAIM against the table, and it never says the
/// file holds.
#[test]
fn a_reader_without_signatures_reports_the_claim() {
    struct Blind;
    impl Verifier for Blind {
        fn verify(&self, _alg: i64, _pk: &[u8], _msg: &[u8], _sig: &[u8]) -> bool {
            false
        }
        fn checks_signatures(&self) -> bool {
            false
        }
    }
    let (mut st, [mut a, mut b]) = genesis([0xA1, 0xB2]);
    b.commit(&mut st, b"b-1", None).unwrap();
    op(&mut st, &mut a, &revoke(&b, 1)).unwrap();
    let c = verify(&st, &ToyIds, &Blind, Trust::Genesis).unwrap().counters;
    assert_eq!((c.seals_ok, c.seals_bad, c.keys_disagree), (0, 2, 0), "{c:?}");
    forge_table(&mut st, &b, 0);
    let r = verify(&st, &ToyIds, &Blind, Trust::Genesis).unwrap();
    assert_eq!((r.counters.seals_ok, r.counters.keys_disagree), (0, 1), "{:?}", r.counters);
    assert!(!r.holds());
}

/// The signature memo keeps the answer, not a pass: a forged signature is bad in every round.
#[test]
fn a_forged_signature_stays_bad_across_the_rounds() {
    let (mut st, mut a, mut b) = root_and_device();
    b.commit(&mut st, b"b-1", None).unwrap();
    op(&mut st, &mut a, &revoke(&b, 5)).unwrap();
    assert_eq!(under(&st, &a).counters.seals_bad, 0, "control");
    let seal = read_heads(&st, Doc::open(&st).unwrap().heads).iter().find(|h| h.actor_id == b.s.id).unwrap().seal.unwrap();
    st.put_cell(seal, 15, st.get(seal, 15) ^ 1);
    st.seal(seal);
    let c = under(&st, &a).counters;
    assert_eq!((c.seals_ok, c.seals_bad), (2, 1), "{c:?}");
}

/// LIMIT 1, ROLLBACK: one file cannot know a newer one exists. The bytes as they stood before
/// the REVOKE take a thief's next commit and hold. Forks of this kind are found by comparing
/// files (M2) or against an anchor.
#[test]
fn limit_the_file_before_the_revoke_knows_nothing_of_it() {
    let (mut st, [mut a, mut b]) = genesis([0xA1, 0xB2]);
    b.commit(&mut st, b"b-1", None).unwrap();
    let before = st.to_bytes_trimmed();
    op(&mut st, &mut a, &revoke(&b, 1)).unwrap();
    assert_eq!(b.commit(&mut st, b"b-2", None), Err(DocError::Revoked { after_seq: 1, seq: 2 }));
    let mut old = Store::from_bytes(&before);
    b.commit(&mut old, b"b-2", None).unwrap();
    assert!(rep(&old).holds(), "the limit moved: {:?}", cnt(&old));
}

/// One root (A), one admitted device (B), read under A's trust from outside the file.
fn root_and_device() -> (Store, D, D) {
    let (mut st, [mut a]) = genesis([0xA1]);
    let b = Dev::new(ToySigner { id: [0xB2; 32] });
    op(&mut st, &mut a, &KeyOp::Admit(key_of(&b.s))).unwrap();
    (st, a, b)
}

fn under(st: &Store, root: &D) -> Report {
    let only = [root.s.id];
    verify(st, &ToyIds, &ToyVerifier, Trust::Only(&only)).unwrap()
}

/// The tombstone a forger writes: the newest record's kind bit, its crc re-sealed.
fn tombstone(st: &mut Store, newest: usize) {
    let o = EvLog::objs_at(st, Doc::open(st).unwrap().log)[newest];
    st.put_cell(o, 16, st.get(o, 16) | 0x80);
    st.seal(o);
}

/// A TOMBSTONE OVER THE REVOKE: `redact_into` refuses it, a forger writes the bytes, and a
/// tombstone cannot say what it was (an erased id confirms no guess). Before `foreign_tombs`
/// the thief's key, unrevoked by the erasure, signed the FORGOTTEN that balanced law 9 and the
/// file held. Now that FORGOTTEN does not cover another author's record, even listing it.
#[test]
fn a_thief_does_not_erase_the_revoke_of_its_own_key() {
    let (mut st, mut a, mut b) = root_and_device();
    b.commit(&mut st, b"b-1", None).unwrap();
    op(&mut st, &mut a, &revoke(&b, 1)).unwrap();
    forge_table(&mut st, &b, 0);
    tombstone(&mut st, 0);
    let body = tombs_body(&st);
    b.commit(&mut st, &body, None).unwrap();
    let r = under(&st, &a);
    assert_eq!((r.counters.redacted, r.counters.declared, r.counters.seals_bad, r.counters.keys_disagree), (1, 1, 0, 0), "law 9 balances and every seal is ok: {:?}", r.counters);
    assert_eq!(r.counters.tombs_foreign, 1, "{:?}", r.counters);
    assert!(!r.holds());
}

/// The twins: a device declares its own tombstone, a root declares anyone's, and a device
/// does not declare a root's or another device's.
#[test]
fn a_tombstone_is_declared_by_its_author_or_a_root() {
    let (mut st, a, mut b) = root_and_device();
    b.commit(&mut st, b"b-1", None).unwrap();
    b.commit(&mut st, b"b-2", None).unwrap();
    tombstone(&mut st, 1);
    assert_eq!(under(&st, &a).counters.tombs_foreign, 1, "undeclared");
    let body = tombs_body(&st);
    b.commit(&mut st, &body, None).unwrap();
    assert!(under(&st, &a).holds(), "the author declares: {:?}", under(&st, &a).counters);

    let (mut st, mut a, mut b) = root_and_device();
    b.commit(&mut st, b"b-1", None).unwrap();
    b.commit(&mut st, b"b-2", None).unwrap();
    tombstone(&mut st, 1);
    let body = tombs_body(&st);
    a.commit(&mut st, &body, None).unwrap();
    assert!(under(&st, &a).holds(), "a root declares: {:?}", under(&st, &a).counters);

    let (mut st, mut a, mut b) = root_and_device();
    a.commit(&mut st, b"a-2", None).unwrap();
    a.commit(&mut st, b"a-3", None).unwrap();
    tombstone(&mut st, 1);
    let body = tombs_body(&st);
    b.commit(&mut st, &body, None).unwrap();
    let r = under(&st, &a);
    assert_eq!((r.counters.redacted, r.counters.declared, r.counters.tombs_foreign), (1, 1, 1), "{:?}", r.counters);
    assert!(!r.holds(), "a device does not declare a root's record");
}

/// LIMIT, A STOLEN ROOT TOMBSTONES: two roots have no rank, B is a root, so its FORGOTTEN covers
/// A's REVOKE. A witness holding A's tip sees it; nothing in the file does.
#[test]
fn limit_a_stolen_root_erases_a_revoke() {
    let (mut st, [mut a, mut b]) = genesis([0xA1, 0xB2]);
    b.commit(&mut st, b"b-1", None).unwrap();
    op(&mut st, &mut a, &revoke(&b, 1)).unwrap();
    forge_table(&mut st, &b, 0);
    tombstone(&mut st, 0);
    let body = tombs_body(&st);
    b.commit(&mut st, &body, None).unwrap();
    let r = rep(&st);
    assert_eq!((r.counters.redacted, r.counters.declared, r.counters.seals_bad, r.counters.keys_disagree, r.counters.tombs_foreign), (1, 1, 0, 0, 0), "{:?}", r.counters);
    assert!(r.holds(), "the limit moved: {:?}", r.counters);
}

/// An `Ids` that shows no body stands in for a forger's own cut writer: `cut_into` then sees
/// no REVOKE, as the forger's writer would not look.
struct Bodiless;
impl Ids for Bodiless {
    fn chained(&self, r: &Record) -> [u8; 32] {
        ToyIds.chained(r)
    }
    fn hash(&self, bytes: &[u8]) -> Option<[u8; 32]> {
        ToyIds.hash(bytes)
    }
    fn body<'a>(&self, _flags: i64, _payload: &'a [u8]) -> &'a [u8] {
        &[]
    }
    fn tree_root(&self, ids: &[[u8; 32]]) -> Option<[u8; 32]> {
        ToyIds.tree_root(ids)
    }
}

/// A THIEF'S OWN CUT, AN ADMITTED KEY: a REVOKE sits after its target's ADMIT, so a cut through
/// the REVOKE takes the ADMIT, and the honest `CutsAdmit` is not what stops him: he zeroes
/// `ref ADMIT_REC` first and writes his own cut. To a verifier with its own roots the key is
/// then admitted by nothing, its TRUNCATE declares nothing, and the file is refused by name.
/// Under `Trust::Genesis` that zero made him a root: the table names the roots there.
#[test]
fn a_thief_with_an_admitted_key_cannot_cut_its_revoke_away() {
    let (mut st, mut a, mut b) = root_and_device();
    b.commit(&mut st, b"b-1", None).unwrap();
    op(&mut st, &mut a, &revoke(&b, 1)).unwrap();
    a.commit(&mut st, b"a-3", None).unwrap();
    let rows = key_rows(&st, Doc::open(&st).unwrap().keys);
    let at = rows.iter().find(|r| r.0 == b.s.id).unwrap().1;
    st.put_cell(at, 7, 0);
    st.seal(at);
    forge_table(&mut st, &b, 0);
    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    let seg = Doc::cut_into(&st, &mut fresh, 1, &Bodiless).unwrap();
    b.commit(&mut fresh, &seg.truncate_body(), None).unwrap();

    let only = [a.s.id];
    assert_eq!(verify(&fresh, &ToyIds, &ToyVerifier, Trust::Only(&only)).map(|_| ()), Err(DocError::CutUndeclared { root: seg.root }));
    assert!(rep(&fresh).holds(), "Trust::Genesis reads the forged table: the limit moved: {:?}", cnt(&fresh));
}

/// LIMIT, A STOLEN ROOT CUTS: no ADMIT to lose, and it signs its own TRUNCATE. The archive
/// still holds the record, under the root that TRUNCATE names: the audit reads it there.
#[test]
fn limit_a_stolen_root_cuts_a_revoke_away() {
    let (mut st, [mut a, mut b]) = genesis([0xA1, 0xB2]);
    b.commit(&mut st, b"b-1", None).unwrap();
    op(&mut st, &mut a, &revoke(&b, 1)).unwrap();
    a.commit(&mut st, b"a-2", None).unwrap();
    let mut fresh = Store::create_bytes(1 << 20).unwrap();
    let seg = Doc::cut_into(&st, &mut fresh, 1, &Bodiless).unwrap();
    forge_table(&mut fresh, &b, 0);
    b.commit(&mut fresh, &seg.truncate_body(), None).unwrap();
    let r = rep(&fresh);
    assert!(r.holds(), "the limit moved: {:?}", r.counters);
    assert_eq!(r.counters.keys_disagree, 0, "{:?}", r.counters);

    let mut archived = EvLog::walk_until_at(&st, Doc::open(&st).unwrap().log, |_| false);
    archived.reverse();
    let ids: Vec<[u8; 32]> = archived[..2].iter().map(|r| r.id).collect();
    assert_eq!(ToyIds.tree_root(&ids), Some(seg.root));
    assert!(archived[1].payload.starts_with(crate::doc::keys::REVOKE_MARK), "the audit that recomputes the root reads it");
}
