//! The DOC's tests with TOY seams -- `ToyIds` (a 4-lane FNV mix), `ToySigner` / `ToyVerifier`
//! (NOT CRYPTO: the "signature" is a mix of the public key and the message, anyone can forge
//! it). They prove the LAYOUT and the COUNTERS; the real ML-DSA-65 kill tests are
//! `dsa_tests.rs`, through the same traits.

use super::keys::{Key, KeyOp};
use super::verify::{verify, Counters, Trust};
use super::write::{stage_doc, Refs};
use super::{Doc, DocError, Ids, Signer, Verifier, ROOT_VERSION_DOC};
use crate::{EvLog, Record, Store};

pub(crate) const ALG_TOY: i64 = 1;

/// FNV-1a 64 over four salted lanes, spread to 32 bytes. Not a hash function anyone should
/// trust; a deterministic 32-byte function of its input is all these tests need.
pub(crate) fn mix(parts: &[&[u8]]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for lane in 0..4u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ lane.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        for p in parts {
            for b in p.iter() {
                h ^= *b as u64;
                h = h.wrapping_mul(0x100_0000_01b3);
            }
            h ^= 0xFF;
            h = h.wrapping_mul(0x100_0000_01b3);
        }
        out[lane as usize * 8..lane as usize * 8 + 8].copy_from_slice(&h.to_le_bytes());
    }
    out
}

pub(crate) struct ToyIds;
impl Ids for ToyIds {
    fn chained(&self, prev: &[u8; 32], payload: &[u8]) -> [u8; 32] {
        mix(&[b"c", prev, payload])
    }
    fn legacy(&self, payload: &[u8]) -> Option<[u8; 32]> {
        Some(mix(&[b"l", payload]))
    }
    fn redacted(&self, payload: &[u8]) -> bool {
        payload.first().is_some_and(|k| k & 0x80 != 0)
    }
}

pub(crate) struct ToySigner {
    pub id: [u8; 32],
}
impl Signer for ToySigner {
    fn actor_id(&self) -> [u8; 32] {
        self.id
    }
    fn alg(&self) -> i64 {
        ALG_TOY
    }
    fn public_key(&self) -> Vec<u8> {
        self.id.to_vec()
    }
    fn sign(&self, msg: &[u8]) -> Vec<u8> {
        mix(&[b"toy", &self.id, msg]).to_vec()
    }
}
pub(crate) struct ToyVerifier;
impl Verifier for ToyVerifier {
    fn verify(&self, alg: i64, pk: &[u8], msg: &[u8], sig: &[u8]) -> bool {
        alg == ALG_TOY && sig == mix(&[b"toy", pk, msg])
    }
}

pub(crate) fn key_of(s: &dyn Signer) -> Key {
    Key { actor_id: s.actor_id(), alg: s.alg(), valid_from: 0, revoked_at: 0, admit_rec: None, pk: s.public_key() }
}

/// One device: its signer and its chain position; `next` builds the next record.
pub(crate) struct Dev<S: Signer> {
    pub s: S,
    pub seq: u64,
    pub tip: [u8; 32],
}
impl<S: Signer> Dev<S> {
    pub fn new(s: S) -> Self {
        Dev { s, seq: 0, tip: [0u8; 32] }
    }
    pub fn next(&mut self, payload: &[u8]) -> Record {
        self.seq += 1;
        let id = ToyIds.chained(&self.tip, payload);
        let r = Record { id, prev: self.tip, actor_pubkey: self.s.actor_id(), actor_seq: self.seq, flags: 0, payload: payload.to_vec() };
        self.tip = id;
        r
    }
    pub fn commit(&mut self, st: &mut Store, payload: &[u8], op: Option<&KeyOp>) -> Result<i64, DocError> {
        let r = self.next(payload);
        let out = Doc::commit_bytes(st, &r, &self.s, 1_700_000_000_000 + self.seq as i64, op);
        if out.is_err() {
            self.seq -= 1;
            self.tip = r.prev;
        }
        out
    }
}

pub(crate) fn doc_with(n: usize) -> (Store, Dev<ToySigner>) {
    let mut st = Store::create_bytes(1 << 20).unwrap();
    let mut owner = Dev::new(ToySigner { id: [0xA1; 32] });
    Doc::init_bytes(&mut st, 0, &[key_of(&owner.s)]).unwrap();
    for i in 0..n {
        owner.commit(&mut st, format!("event-{i}").as_bytes(), None).unwrap();
    }
    (st, owner)
}

/// A FORGER'S EDIT, REPAIRED UNDER THE ID SCHEME: record `k` (oldest first) gets one payload
/// bit changed, its id is recomputed, every later record's `prev` and id are recomputed, the
/// log root's tip moved, every touched object re-sealed. No counter gap is left: the chain
/// is whole and every id chains. Only the seals, which signed the OLD tips, can see it.
pub(crate) fn repair_edit(st: &mut Store, k: usize) {
    let view = Doc::open(st).unwrap();
    let mut objs = EvLog::objs_at(st, view.log);
    objs.reverse();
    let mut recs = EvLog::walk_until_at(st, view.log, |_| false);
    recs.reverse();
    let mut prev = recs[k].prev;
    for i in k..objs.len() {
        let o = objs[i];
        let mut payload = recs[i].payload.clone();
        if i == k {
            payload[0] ^= 1;
            st.put_cell(o, 16, st.get(o, 16) ^ 1);
        }
        let id = ToyIds.chained(&prev, &payload);
        for (j, c) in crate::b32_to_cells(&prev).iter().enumerate() {
            st.put_cell(o, 7 + j, *c);
        }
        for (j, c) in crate::b32_to_cells(&id).iter().enumerate() {
            st.put_cell(o, 3 + j, *c);
        }
        st.seal(o);
        prev = id;
    }
    let log = view.log.unwrap();
    for (j, c) in crate::b32_to_cells(&prev).iter().enumerate() {
        st.put_cell(log, 3 + j, *c);
    }
    st.seal(log);
}

/// KILL TEST 3 (W-MG-M0's finding: `chain_check` cannot see a gap rotation leaves by design,
/// so the SEAL is what binds history): an edit repaired under the id scheme leaves every
/// record `chained` and nothing `broken` -- and is caught by `seals_bad`, because the seals
/// signed the tips the repair had to move. Control: the untouched image has none bad.
#[test]
fn an_edit_repaired_under_the_id_scheme_is_caught_by_seals_bad() {
    let (mut st, _) = doc_with(3);
    assert_eq!(count(&st).seals_bad, 0, "control");
    repair_edit(&mut st, 1);
    let c = count(&st);
    assert_eq!((c.chained, c.broken, c.forks), (3, 0, 0), "the counters see no gap: {c:?}");
    assert_eq!((c.seals_ok, c.seals_bad), (1, 2), "seals 2 and 3 name tips the repair moved: {c:?}");
}

fn count(st: &Store) -> Counters {
    verify(st, &ToyIds, &ToyVerifier, Trust::Genesis).unwrap().counters
}

#[test]
fn three_sealed_commits_verify_and_survive_the_byte_trip() {
    let (st, owner) = doc_with(3);
    let back = Store::from_bytes(&st.to_bytes_trimmed());
    let rep = verify(&back, &ToyIds, &ToyVerifier, Trust::Genesis).unwrap();
    assert_eq!(rep.counters, Counters { records: 3, chained: 3, seals_ok: 3, ..Counters::default() });
    assert!(rep.holds());
    assert_eq!(rep.heads.len(), 1);
    assert_eq!((rep.heads[0].seq, rep.heads[0].tip), (3, owner.tip));
    assert_eq!((rep.keys, rep.admitted), (1, 1));
    let view = Doc::open(&back).unwrap();
    assert_eq!(EvLog::len_at(&back, view.log), 3);
    assert_eq!(EvLog::tip_at(&back, view.log), Some(owner.tip));
}

/// EVERY READER WRITTEN BEFORE THIS LAYOUT REFUSES IT BY NAME: the KV readers read root
/// cell 5 and refuse `> 4` (the bebop-io and conformance halves live beside those readers);
/// the log reader asks `unknown_version`. The twin: an old log image is not a DOC.
#[test]
fn a_doc_root_is_refused_by_version_name() {
    let (st, _) = doc_with(1);
    let root = st.root().unwrap();
    assert_eq!(st.get(root, 5), ROOT_VERSION_DOC, "the KV readers' version cell");
    assert_eq!(st.get(root, 7), ROOT_VERSION_DOC, "the log readers' version cell");
    assert_eq!(EvLog::unknown_version(&st), Some(ROOT_VERSION_DOC));

    let mut old = Store::create_bytes(64 * 1024).unwrap();
    EvLog::init_bytes(&mut old).unwrap();
    assert_eq!(EvLog::unknown_version(&old), None);
    assert_eq!(Doc::open(&old), Err(DocError::NotDoc));
}

/// KILL TEST 1: a changed tip gives `seals_bad`. Two forgers, both re-sealing the crc so the
/// change is not caught as damage: one rewrites the HEADS row, one rewrites the record's id.
/// The control first: untouched, nothing is bad. And the third shape -- a change that does
/// NOT re-seal -- is damage, refused before any counting.
#[test]
fn a_changed_tip_gives_seals_bad() {
    let (clean, _) = doc_with(3);
    assert_eq!(count(&clean).seals_bad, 0, "control");

    let mut st = Store::from_bytes(&clean.to_bytes_trimmed());
    let heads = Doc::open(&st).unwrap().heads.unwrap();
    st.put_cell(heads, 1 + 5, st.get(heads, 1 + 5) ^ 1);
    st.seal(heads);
    let c = count(&st);
    assert_eq!((c.seals_bad, c.seals_ok), (1, 2), "the head's seal no longer names the head: {c:?}");

    let mut st = Store::from_bytes(&clean.to_bytes_trimmed());
    let log = Doc::open(&st).unwrap().log;
    let newest = EvLog::objs_at(&st, log)[0];
    st.put_cell(newest, 3, st.get(newest, 3) ^ 1);
    st.seal(newest);
    let c = count(&st);
    assert_eq!((c.seals_bad, c.seals_ok, c.broken, c.chained), (1, 2, 1, 2), "the seal names a record that is gone: {c:?}");

    let mut st = Store::from_bytes(&clean.to_bytes_trimmed());
    st.put_cell(heads, 1 + 5, st.get(heads, 1 + 5) ^ 1);
    assert!(matches!(verify(&st, &ToyIds, &ToyVerifier, Trust::Genesis), Err(DocError::BadCrc(_))), "unsealed = damage");
}

/// KILL TEST 2: REVOKE after seq k breaks k+1. The owner admits device B by a sealed ADMIT
/// record; B seals 1..3; the owner revokes B after 2: B's third seal is bad, its first two
/// hold, and the writer refuses B's fourth outright. Control: before the REVOKE, all hold.
#[test]
fn revoke_after_k_breaks_k_plus_1() {
    let (mut st, mut owner) = doc_with(0);
    let mut b = Dev::new(ToySigner { id: [0xB2; 32] });
    let admit = KeyOp::Admit(key_of(&b.s));
    owner.commit(&mut st, &admit.body(), Some(&admit)).unwrap();
    for i in 0..3 {
        b.commit(&mut st, format!("b-{i}").as_bytes(), None).unwrap();
    }
    let rep = verify(&st, &ToyIds, &ToyVerifier, Trust::Genesis).unwrap();
    assert_eq!((rep.counters.seals_ok, rep.counters.seals_bad, rep.admitted), (4, 0, 2), "control: {rep:?}");

    let revoke = KeyOp::Revoke { actor_id: b.s.id, after_seq: 2 };
    owner.commit(&mut st, &revoke.body(), Some(&revoke)).unwrap();
    let c = count(&st);
    assert_eq!((c.seals_ok, c.seals_bad, c.records), (4, 1, 5), "B's seal 3 is past the revoke: {c:?}");
    assert_eq!(b.commit(&mut st, b"b-4", None), Err(DocError::Revoked { after_seq: 2, seq: 4 }));
}

/// Trust decides what the seals mean: with no trusted key nothing is admitted and every
/// seal is bad; naming the owner from outside is the genesis answer.
#[test]
fn trust_is_the_verifiers_decision() {
    let (st, owner) = doc_with(2);
    let none = verify(&st, &ToyIds, &ToyVerifier, Trust::Only(&[])).unwrap();
    assert_eq!((none.counters.seals_ok, none.counters.seals_bad, none.admitted), (0, 2, 0));
    let named = verify(&st, &ToyIds, &ToyVerifier, Trust::Only(&[owner.s.id])).unwrap();
    assert_eq!(named.counters, count(&st));
}

/// Two records at one `(actor, seq)` are a fork: named and counted, the seals untouched.
#[test]
fn a_fork_is_counted_by_name() {
    let (mut st, mut owner) = doc_with(2);
    let dup = Record { actor_seq: 2, ..owner.next(b"forked") };
    let view = Doc::open(&st).unwrap();
    let mut tx = st.begin().unwrap();
    let log = EvLog::stage_append_in(&mut st, &mut tx, view.log.unwrap(), &dup, None).unwrap();
    let d = stage_doc(&mut st, &mut tx, view.flags, Refs { log: Some(log), heads: view.heads, keys: view.keys, ..Refs::default() }).unwrap();
    st.commit_bytes(&tx, d);
    let c = count(&st);
    assert_eq!((c.forks, c.records, c.chained, c.seals_ok), (1, 3, 3, 2), "{c:?}");
}

/// A redaction in place (payload rewritten, kind's high bit set, id and prev kept) whose
/// link holds is `redacted`, not broken; the seals still hold because the ids did not move.
#[test]
fn a_declared_redaction_is_counted_as_redacted() {
    let (mut st, _) = doc_with(3);
    let log = Doc::open(&st).unwrap().log;
    let middle = EvLog::objs_at(&st, log)[1];
    st.put_cell(middle, 16, st.get(middle, 16) | 0x80);
    st.seal(middle);
    let c = count(&st);
    assert_eq!((c.redacted, c.chained, c.broken, c.seals_ok), (1, 2, 0, 3), "{c:?}");
}

#[test]
fn the_writer_refuses_a_wrong_seq_and_an_unknown_actor() {
    let (mut st, mut owner) = doc_with(1);
    let skip = Record { actor_seq: 5, ..owner.next(b"x") };
    assert_eq!(Doc::commit_bytes(&mut st, &skip, &owner.s, 0, None), Err(DocError::Seq { want: 2, got: 5 }));
    let stranger = ToySigner { id: [0xC3; 32] };
    let r = Record { actor_pubkey: stranger.id, ..owner.next(b"y") };
    assert_eq!(Doc::commit_bytes(&mut st, &r, &stranger, 0, None), Err(DocError::UnknownActor(stranger.id)));
    assert_eq!(Doc::commit_bytes(&mut st, &r, &owner.s, 0, None), Err(DocError::Actor));
}

/// The conformance fixture `crates/bebop-wasm/fixtures/doc.store`, written only when asked
/// (`BEBOP_DOC_FIXTURE=<path>`): deterministic, three toy-sealed commits.
#[test]
fn write_the_doc_fixture_when_asked() {
    let (st, _) = doc_with(3);
    if let Ok(p) = std::env::var("BEBOP_DOC_FIXTURE") {
        std::fs::write(&p, st.to_bytes_trimmed()).unwrap();
    }
    assert_eq!(count(&st).seals_ok, 3);
}
