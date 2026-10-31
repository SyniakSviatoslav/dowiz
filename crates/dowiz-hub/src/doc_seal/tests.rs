//! THE KILL TESTS ON A REAL HUB IMAGE (W-MG-M1B): a `Hub` the order log writes today -- one
//! pre-M0 FNV record and five sha256 ones -- sealed under a DOC with a real ML-DSA-65 device
//! key; two hub commits each seal the tip; every old loader refuses the sealed image BY
//! NAME; one changed byte under a seal is `broken` (re-sealed) or refused (not); the image
//! grows and verifies the same; the three anchor kinds with their adapters, each with its
//! control and its RED.

use super::ots::{self, Attestation, BlockHeaders, Calendar, Op, Ots, Proof, Reached, Ts, TAG_BITCOIN, TAG_PENDING};
use super::roughtime::{self, Roughtime, RoughtimeServer, NONCE_BYTES};
use super::tsa::{self, PlatformTsa};
use super::{DeviceKey, HubIds, SealedHub, KIND_KEYOP};
use crate::logimage::LogImage;
use crate::{EventKind, Hub, HubError};
use bebop_store::evlog::doc::{AnchorCounts, Ids, Key, KeyOp, Verdict, KIND_OTS, KIND_PLATFORM_TSA, KIND_ROUGHTIME};
use bebop_store::evlog::{EvLog, Record};
use bebop_store::nodekey::sha256;
use bebop_store::Store;

pub(super) const JSON: &str = r#"{"status":"new","total":{"amount":1500,"currency":"ALL"}}"#;

pub(super) fn device(seed: u8) -> DeviceKey {
    DeviceKey::from_seed(&[seed; 32])
}

/// A REAL hub image: one legacy (FNV-chained, flags 0, pre-M0) record first, then five
/// records the order log writes today. `Hub::load` + `chain_check` say legacy 1, chained 5.
pub(super) fn real_hub() -> Vec<u8> {
    let mut hub = Hub::create_sized(64 * 1024).unwrap();
    let prev = EvLog::tip(&hub.store).unwrap_or([0u8; 32]);
    let mut payload = vec![EventKind::Placed as u8, 3];
    payload.extend_from_slice(b"old");
    payload.extend_from_slice(JSON.as_bytes());
    let id = crate::chain::legacy_id_chained(&prev, &payload);
    EvLog::append_tip_bytes(&mut hub.store, &Record { id, prev, actor_pubkey: [0u8; 32], actor_seq: 0, flags: 0, payload }).unwrap();
    for i in 1..=5u64 {
        hub.append(EventKind::Placed, &format!("o{i}"), JSON, i, [0u8; 32]).unwrap();
    }
    let bytes = hub.to_bytes_trimmed();
    let c = Hub::load(&bytes).unwrap().chain_check();
    assert_eq!((c.records, c.legacy, c.chained, c.broken), (6, 1, 5, 0), "the control hub: {c:?}");
    bytes
}

/// The seal on real hub bytes, then two hub commits, each sealing its tip; every reader
/// written before refuses the sealed image by name.
#[test]
fn a_real_hub_is_sealed_and_its_commits_seal_the_tip() {
    let key = device(7);
    let mut sh = SealedHub::seal_hub(&real_hub(), &key).unwrap();
    let rep = sh.verify().unwrap();
    let c = rep.counters;
    assert_eq!((c.records, c.legacy, c.chained, c.broken, c.seals_ok), (6, 1, 5, 0, 0), "sealed, no commit yet: {c:?}");
    assert!(!rep.holds(), "nothing is sealed until a device commits");

    sh.append(EventKind::Placed, "o6", JSON, &key, 1_730_000_000_000).unwrap();
    sh.append(EventKind::Advanced, "o6", JSON, &key, 1_730_000_001_000).unwrap();
    let bytes = sh.to_bytes_trimmed();
    let sh = SealedHub::load(&bytes).unwrap();
    let rep = sh.verify().unwrap();
    let c = rep.counters;
    assert_eq!((c.records, c.legacy, c.chained, c.broken, c.seals_ok, c.seals_bad, c.forks), (8, 1, 7, 0, 2, 0, 0), "{c:?}");
    assert!(rep.holds());
    assert_eq!(rep.heads.len(), 1);
    assert_eq!((rep.heads[0].actor_id, rep.heads[0].seq, Some(rep.heads[0].tip)), (key.id(), 2, sh.tip()));
    assert_eq!(sh.events().len(), 8, "the order events read back through the DOC");
    assert_eq!(sh.events()[0].order_id, "o6");
    let cc = sh.chain_check();
    assert_eq!((cc.legacy, cc.chained, cc.broken), (1, 7, 0), "M0's walk over the DOC's log: {cc:?}");

    // EVERY READER WRITTEN BEFORE REFUSES BY NAME.
    assert!(matches!(Hub::load(&bytes), Err(HubError::UnknownVersion(5))), "Hub::load");
    assert!(matches!(LogImage::load(&bytes), Err(HubError::UnknownVersion(5))), "LogImage::load");
    assert!(matches!(SealedHub::seal_hub(&bytes, &key), Err(HubError::UnknownVersion(5))), "sealing twice");
}

/// KILL TEST: one changed byte of a record under a seal. Not re-sealed: the load refuses
/// (BadCrc). Re-sealed (a forger): `broken` 1, and the report no longer holds. Then the
/// forger moves the HEADS tip to hide it: `seals_bad`.
#[test]
fn a_changed_byte_under_a_seal_is_broken_or_refused() {
    let key = device(7);
    let mut sh = SealedHub::seal_hub(&real_hub(), &key).unwrap();
    sh.append(EventKind::Placed, "o6", JSON, &key, 1).unwrap();
    sh.append(EventKind::Paid, "o6", JSON, &key, 2).unwrap();
    assert!(sh.verify().unwrap().holds(), "control");
    let clean = sh.to_bytes_trimmed();

    let mut st = Store::from_bytes(&clean);
    let log = bebop_store::evlog::doc::Doc::open(&st).unwrap().log;
    let newest = EvLog::objs_at(&st, log)[0];
    // A v2 record: 12 header cells, 4 actor cells (12..16), the 16-byte nonce (16..18), the
    // body from cell 18 -- bit 0 of its first byte is bit 0 of the kind (Paid 3 -> Advanced 2,
    // still an event, which is the point: the edit decodes, only the id does not hold).
    st.put_cell(newest, 18, st.get(newest, 18) ^ 1);
    assert!(matches!(SealedHub::load(&st.to_bytes()), Err(HubError::BadCrc(_))), "damage is refused at load");

    st.seal(newest);
    let forged = SealedHub::load(&st.to_bytes()).unwrap();
    let rep = forged.verify().unwrap();
    assert_eq!((rep.counters.broken, rep.counters.seals_ok), (1, 2), "a re-sealed edit is broken: {rep:?}");
    assert!(!rep.holds());

    // The forger also repairs the id, so the record chains -- and then the seal, which signed
    // the old tip, is what catches it.
    let mut st = Store::from_bytes(&clean);
    let r = EvLog::walk_until_at(&st, log, |_| true).remove(0);
    let mut payload = r.payload.clone();
    payload[16] ^= 1;
    st.put_cell(newest, 18, st.get(newest, 18) ^ 1);
    let id = HubIds.chained(&r.prev, &payload);
    for (j, c) in bebop_store::evlog::doc::id_cells(&id).iter().enumerate() {
        st.put_cell(newest, 3 + j, *c);
    }
    st.seal(newest);
    let log_root = log.unwrap();
    for (j, c) in bebop_store::evlog::doc::id_cells(&id).iter().enumerate() {
        st.put_cell(log_root, 3 + j, *c);
    }
    st.seal(log_root);
    let rep = SealedHub::load(&st.to_bytes()).unwrap().verify().unwrap();
    assert_eq!((rep.counters.broken, rep.counters.chained), (0, 7), "the repair leaves every id chained: {rep:?}");
    assert_eq!((rep.counters.seals_ok, rep.counters.seals_bad), (1, 1), "and the seal over the moved tip is bad: {rep:?}");
}

/// The image grows on a full arena (`Doc::copy_bytes`) and the counters, heads and events
/// are the same after; a commit by a key the file does not admit is refused by name.
#[test]
fn a_sealed_hub_grows_and_refuses_a_stranger() {
    let key = device(7);
    let mut sh = SealedHub::create_sized(&key, (bebop_store::ARENA + 2_500) * 8).unwrap();
    let before = sh.to_bytes().len();
    for i in 0..6 {
        sh.append(EventKind::Placed, &format!("g{i}"), JSON, &key, i).unwrap();
    }
    assert!(sh.to_bytes().len() > before, "the image doubled: {} -> {}", before, sh.to_bytes().len());
    let rep = sh.verify().unwrap();
    assert_eq!((rep.counters.records, rep.counters.seals_ok, rep.counters.seals_bad), (6, 6, 0), "{rep:?}");
    assert!(rep.holds());
    assert_eq!(sh.events().len(), 6);

    let stranger = device(9);
    let e = sh.append(EventKind::Placed, "x", JSON, &stranger, 99).unwrap_err();
    assert!(format!("{e:?}").contains("not in KEYS"), "{e:?}");
}

/// ADMIT a second device through the owner's sealed chain: its commits verify, the key-op
/// record is not an order event, REVOKE after seq 1 makes its second seal bad.
#[test]
fn a_second_device_is_admitted_and_revoked() {
    let owner = device(7);
    let b = device(9);
    let mut sh = SealedHub::create(&owner).unwrap();
    let admit = KeyOp::Admit(Key { valid_from: 1, ..b.key() });
    sh.commit_keyop(&admit, &owner, 1).unwrap();
    sh.append(EventKind::Placed, "b1", JSON, &b, 2).unwrap();
    sh.append(EventKind::Placed, "b2", JSON, &b, 3).unwrap();
    let rep = sh.verify().unwrap();
    assert_eq!((rep.counters.records, rep.counters.seals_ok, rep.keys, rep.admitted), (3, 3, 2, 2), "{rep:?}");
    assert_eq!(sh.events().len(), 2, "the ADMIT record is not an order event");
    assert_eq!(sh.len(), 3);
    let keyop_kind = EvLog::walk_until_at(&Store::from_bytes(&sh.to_bytes_trimmed()), None, |_| true).len();
    assert_eq!(keyop_kind, 0, "a DOC root is not a log root: the raw walk sees nothing");
    assert_eq!(KIND_KEYOP, b'K');

    let revoke = KeyOp::Revoke { actor_id: b.id(), after_seq: 1 };
    sh.commit_keyop(&revoke, &owner, 4).unwrap();
    let rep = sh.verify().unwrap();
    assert_eq!((rep.counters.seals_ok, rep.counters.seals_bad), (3, 1), "B's seal 2 is past the revoke: {rep:?}");
    let e = sh.append(EventKind::Placed, "b3", JSON, &b, 5).unwrap_err();
    assert!(format!("{e:?}").contains("revoked"), "{e:?}");
}

// ── anchors ──────────────────────────────────────────────────────────────────────────────

/// KIND 1, the platform TSA mark: ok under the platform key the auditor brings; unchecked
/// with no key or another key; bad when a byte of the mark or the time moved.
#[test]
fn a_platform_tsa_mark_checks_under_the_auditors_key() {
    let key = device(7);
    let platform = device(42);
    let mut sh = SealedHub::create(&key).unwrap();
    sh.append(EventKind::Placed, "o1", JSON, &key, 1).unwrap();
    let tip = sh.tip().unwrap();
    sh.anchor(KIND_PLATFORM_TSA, &tip, 1_730_000_000_000, &tsa::mark(&platform, &tip, 1_730_000_000_000)).unwrap();
    let bytes = sh.to_bytes_trimmed();
    let sh = SealedHub::load(&bytes).unwrap();
    assert!(sh.verify().unwrap().holds(), "anchoring moved nothing");

    let keys = [platform.public_key().to_vec()];
    let checked = sh.anchors(&PlatformTsa { keys: &keys }).unwrap();
    assert_eq!(AnchorCounts::of(&checked), AnchorCounts { anchors: 1, ok: 1, bad: 0, unchecked: 0, cut: 0 }, "{checked:?}");
    assert!(matches!(&checked[0].verdict, Verdict::Ok { not_after_ms: Some(1_730_000_000_000), .. }), "{:?}", checked[0].verdict);

    let none = sh.anchors(&PlatformTsa { keys: &[] }).unwrap();
    assert!(matches!(&none[0].verdict, Verdict::Unchecked(w) if w.contains("no platform key")), "{:?}", none[0].verdict);
    let other = [device(43).public_key().to_vec()];
    let other = sh.anchors(&PlatformTsa { keys: &other }).unwrap();
    assert!(matches!(&other[0].verdict, Verdict::Unchecked(w) if w.contains("not one of the platform keys")), "{:?}", other[0].verdict);

    // RED: one flipped bit of the signature, re-sealed; and the time moved under the mark.
    let mut st = Store::from_bytes(&bytes);
    let a = bebop_store::evlog::doc::Doc::open(&st).unwrap().anch.unwrap();
    st.put_cell(a, 9 + 300, st.get(a, 9 + 300) ^ 1);
    st.seal(a);
    let forged = SealedHub::load(&st.to_bytes()).unwrap().anchors(&PlatformTsa { keys: &keys }).unwrap();
    assert!(matches!(&forged[0].verdict, Verdict::Bad(_)), "{:?}", forged[0].verdict);
    let mut st = Store::from_bytes(&bytes);
    st.put_cell(a, 6, 1_730_000_000_001);
    st.seal(a);
    let moved = SealedHub::load(&st.to_bytes()).unwrap().anchors(&PlatformTsa { keys: &keys }).unwrap();
    assert!(matches!(&moved[0].verdict, Verdict::Bad(_)), "{:?}", moved[0].verdict);
}

/// The published `.ots` encoding, BY HAND: a pending attestation and a Bitcoin branch at
/// one node. The parser reads it; the serializer writes it back byte for byte.
#[test]
fn an_ots_proof_parses_from_the_published_encoding() {
    let d = [0x11u8; 32];
    let uri = "https://a.pool.opentimestamps.org";
    let mut file = Vec::new();
    file.extend_from_slice(&ots::MAGIC);
    file.push(0x01);
    file.push(ots::OP_SHA256);
    file.extend_from_slice(&d);
    file.extend_from_slice(&[0xf0, 0x02, 0xaa, 0xbb]); // append aabb
    file.push(0xff); // two items at this node: the pending attestation, then the sha256 branch
    file.push(0x00);
    file.extend_from_slice(&TAG_PENDING);
    file.push(0x22); // payload: varbytes(uri) = 0x21 ‖ 33 bytes
    file.push(0x21);
    file.extend_from_slice(uri.as_bytes());
    file.push(ots::OP_SHA256);
    file.push(0x00);
    file.extend_from_slice(&TAG_BITCOIN);
    file.extend_from_slice(&[0x02, 0xac, 0x02]); // payload: varint 300
    let p = ots::parse(&file).unwrap();
    let want = Proof {
        digest: d.to_vec(),
        ts: Ts {
            attestations: vec![],
            ops: vec![(
                Op::Append(vec![0xaa, 0xbb]),
                Ts { attestations: vec![Attestation::Pending { uri: uri.to_string() }], ops: vec![(Op::Sha256, Ts { attestations: vec![Attestation::Bitcoin { height: 300 }], ops: vec![] })] },
            )],
        },
    };
    assert_eq!(p, want);
    assert_eq!(ots::serialize(&p).unwrap(), file, "byte for byte");
    let mut reached = Vec::new();
    ots::reach(&p.ts, &p.digest, &mut reached);
    let mut m = d.to_vec();
    m.extend_from_slice(&[0xaa, 0xbb]);
    assert_eq!(reached[0], Reached::At { attestation: Attestation::Pending { uri: uri.to_string() }, msg: m.clone() });
    assert_eq!(reached[1], Reached::At { attestation: Attestation::Bitcoin { height: 300 }, msg: sha256(&m).to_vec() });

    let mut cut = file.clone();
    cut.truncate(file.len() - 1);
    assert!(ots::parse(&cut).unwrap_err().contains("truncated"));
    let mut tail = file.clone();
    tail.push(0);
    assert!(ots::parse(&tail).unwrap_err().contains("trailing"));
}

/// A calendar in memory: `submit` answers `append nonce ‖ sha256 ‖ pending`; `upgrade`
/// answers `append pad ‖ sha256 ‖ bitcoin 900_000` once `committed` is set. The headers in
/// memory hold the Merkle root the upgrade reaches.
struct FakeCalendar {
    committed: bool,
}
impl Calendar for FakeCalendar {
    fn submit(&self, calendar: &str, _digest: &[u8; 32]) -> Result<Vec<u8>, String> {
        let ts = Ts {
            attestations: vec![],
            ops: vec![(Op::Append(vec![0x01, 0x02, 0x03, 0x04]), Ts { attestations: vec![], ops: vec![(Op::Sha256, Ts { attestations: vec![Attestation::Pending { uri: calendar.to_string() }], ops: vec![] })] })],
        };
        let mut out = Vec::new();
        let p = Proof { digest: vec![0; 32], ts };
        let all = ots::serialize(&p)?;
        out.extend_from_slice(&all[ots::MAGIC.len() + 2 + 32..]);
        Ok(out)
    }
    fn upgrade(&self, _uri: &str, _msg: &[u8]) -> Result<Option<Vec<u8>>, String> {
        if !self.committed {
            return Ok(None);
        }
        let ts = Ts { attestations: vec![], ops: vec![(Op::Prepend(vec![0xee]), Ts { attestations: vec![], ops: vec![(Op::Sha256, Ts { attestations: vec![Attestation::Bitcoin { height: 900_000 }], ops: vec![] })] })] };
        let all = ots::serialize(&Proof { digest: vec![0; 32], ts })?;
        Ok(Some(all[ots::MAGIC.len() + 2 + 32..].to_vec()))
    }
}
struct FakeHeaders {
    root: [u8; 32],
}
impl BlockHeaders for FakeHeaders {
    fn merkle_root(&self, height: u64) -> Option<[u8; 32]> {
        (height == 900_000).then_some(self.root)
    }
    fn time_ms(&self, height: u64) -> Option<i64> {
        (height == 900_000).then_some(1_740_000_000_000)
    }
}

/// KIND 2, OpenTimestamps on the nightly witness root (= the tip): stamped through the
/// calendar seam -> pending -> `Unchecked` naming the calendar; upgraded -> Bitcoin block
/// 900000 -> `Ok` against the auditor's headers, `Unchecked` without headers, `Bad` against
/// a header whose root differs, `Bad` when the proof is over another digest.
#[test]
fn an_ots_anchor_is_pending_then_checks_against_block_headers() {
    let key = device(7);
    let mut sh = SealedHub::create(&key).unwrap();
    sh.append(EventKind::Placed, "o1", JSON, &key, 1).unwrap();
    let tip = sh.tip().unwrap();
    let pending = ots::stamp(&FakeCalendar { committed: false }, ots::CALENDARS[0], &tip).unwrap();
    assert_eq!(ots::upgrade(&FakeCalendar { committed: false }, &pending).unwrap(), None, "nothing to upgrade yet");
    sh.anchor(KIND_OTS, &tip, 1_730_000_000_000, &pending).unwrap();
    let got = sh.anchors(&Ots { headers: None }).unwrap();
    assert!(matches!(&got[0].verdict, Verdict::Unchecked(w) if w.contains("pending at https://a.pool.opentimestamps.org")), "{:?}", got[0].verdict);

    let upgraded = ots::upgrade(&FakeCalendar { committed: true }, &pending).unwrap().expect("upgraded");
    // the root the ops reach: sha256(0xee ‖ sha256(tip ‖ 01020304))
    let mut m = tip.to_vec();
    m.extend_from_slice(&[1, 2, 3, 4]);
    let mut m2 = vec![0xee];
    m2.extend_from_slice(&sha256(&m));
    let root = sha256(&m2);
    sh.anchor(KIND_OTS, &tip, 1_740_000_000_000, &upgraded).unwrap();
    let bytes = sh.to_bytes_trimmed();
    let sh = SealedHub::load(&bytes).unwrap();
    let headers = FakeHeaders { root };
    let got = sh.anchors(&Ots { headers: Some(&headers) }).unwrap();
    assert_eq!(got.len(), 2, "newest first: the upgraded, then the pending");
    assert_eq!(got[0].verdict, Verdict::Ok { not_after_ms: Some(1_740_000_000_000), not_before_ms: None, what: "the ops reach the Merkle root of Bitcoin block 900000".to_string() });
    assert!(matches!(&got[1].verdict, Verdict::Unchecked(_)));
    let blind = sh.anchors(&Ots { headers: None }).unwrap();
    assert!(matches!(&blind[0].verdict, Verdict::Unchecked(w) if w.contains("attests Bitcoin block 900000; no block headers")), "{:?}", blind[0].verdict);
    let wrong = FakeHeaders { root: [0xAB; 32] };
    let wrong = sh.anchors(&Ots { headers: Some(&wrong) }).unwrap();
    assert!(matches!(&wrong[0].verdict, Verdict::Bad(w) if w.contains("do not reach")), "{:?}", wrong[0].verdict);

    // RED: the same proof filed over another record id.
    let mut sh2 = SealedHub::load(&bytes).unwrap();
    sh2.append(EventKind::Placed, "o2", JSON, &key, 2).unwrap();
    let other = sh2.tip().unwrap();
    sh2.anchor(KIND_OTS, &other, 1, &upgraded).unwrap();
    let got = sh2.anchors(&Ots { headers: Some(&headers) }).unwrap();
    assert!(matches!(&got[0].verdict, Verdict::Bad(w) if w.contains("different digest")), "{:?}", got[0].verdict);
}

/// A Roughtime message in memory: tags ascending, values 4-byte aligned.
fn rt_map(entries: &[(u32, Vec<u8>)]) -> Vec<u8> {
    let n = entries.len() as u32;
    let mut out = n.to_le_bytes().to_vec();
    let mut off = 0u32;
    for (i, (_, v)) in entries.iter().enumerate() {
        if i > 0 {
            out.extend_from_slice(&off.to_le_bytes());
        }
        off += v.len() as u32;
    }
    for (t, _) in entries {
        out.extend_from_slice(&t.to_le_bytes());
    }
    for (_, v) in entries {
        out.extend_from_slice(v);
    }
    out
}

struct FakeRoughtime;
impl RoughtimeServer for FakeRoughtime {
    fn query(&self, _nonce: &[u8; NONCE_BYTES]) -> Result<Vec<u8>, String> {
        let srep = rt_map(&[(roughtime::TAG_RADI, 1_000_000u32.to_le_bytes().to_vec()), (roughtime::TAG_MIDP, 1_730_000_000_000_000u64.to_le_bytes().to_vec())]);
        Ok(rt_map(&[(roughtime::TAG_SREP, srep)]))
    }
}

/// KIND 3, Roughtime: the response is parsed (MIDP, RADI) and the verdict is `Unchecked`
/// naming what was not checked -- never `Ok` in a build with no Ed25519; a nonce not
/// derived from `over` is `Bad`.
#[test]
fn a_roughtime_anchor_is_parsed_and_never_passes() {
    let key = device(7);
    let mut sh = SealedHub::create(&key).unwrap();
    sh.append(EventKind::Placed, "o1", JSON, &key, 1).unwrap();
    let tip = sh.tip().unwrap();
    let proof = roughtime::stamp(&FakeRoughtime, &tip).unwrap();
    assert_eq!(&proof[..32], &tip, "the nonce starts with the tip");
    sh.anchor(KIND_ROUGHTIME, &tip, 1_730_000_000_000, &proof).unwrap();
    let got = sh.anchors(&Roughtime).unwrap();
    match &got[0].verdict {
        Verdict::Unchecked(w) => {
            assert!(w.contains("MIDP 1730000000000000 µs, RADI 1000000 µs"), "{w}");
            assert!(w.contains("no earlier than 1729999999000 ms"), "{w}");
            assert!(w.contains("NOT checked"), "{w}");
        }
        v => panic!("never ok: {v:?}"),
    }
    assert_eq!(AnchorCounts::of(&got), AnchorCounts { anchors: 1, ok: 0, bad: 0, unchecked: 1, cut: 0 });

    let mut forged = proof.clone();
    forged[40] ^= 1;
    let mut sh2 = SealedHub::load(&sh.to_bytes_trimmed()).unwrap();
    sh2.anchor(KIND_ROUGHTIME, &tip, 2, &forged).unwrap();
    let got = sh2.anchors(&Roughtime).unwrap();
    assert!(matches!(&got[0].verdict, Verdict::Bad(w) if w.contains("nonce")), "{:?}", got[0].verdict);
}
