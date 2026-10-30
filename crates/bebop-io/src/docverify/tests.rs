//! `verify_doc` from bytes, and the native KV readers of this crate REFUSING a DOC root BY
//! NAME (W-MG-M1): `Kv::load_checked` and `Db::from_bytes` both answer UnknownVersion(5).

use super::verify_doc;
use crate::db::{Db, Options};
use crate::refusal::Refusal;
use bebop_image::Store;
use bebop_kv::verify::KvError;
use bebop_kv::Kv;
use bebop_log::doc::{Doc, Ids, Key, Signer, Trust, Verifier};
use bebop_log::Record;

/// A deterministic 32-byte function of its input: NOT a hash, NOT a signature.
fn mix(parts: &[&[u8]]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for lane in 0..4u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ lane.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        for p in parts {
            for b in p.iter() {
                h = (h ^ *b as u64).wrapping_mul(0x100_0000_01b3);
            }
            h = (h ^ 0xFF).wrapping_mul(0x100_0000_01b3);
        }
        out[lane as usize * 8..lane as usize * 8 + 8].copy_from_slice(&h.to_le_bytes());
    }
    out
}
struct Toy;
impl Ids for Toy {
    fn chained(&self, prev: &[u8; 32], payload: &[u8]) -> [u8; 32] {
        mix(&[b"c", prev, payload])
    }
}
impl Signer for Toy {
    fn actor_id(&self) -> [u8; 32] {
        [0xA1; 32]
    }
    fn alg(&self) -> i64 {
        1
    }
    fn public_key(&self) -> Vec<u8> {
        vec![0xA1; 32]
    }
    fn sign(&self, msg: &[u8]) -> Vec<u8> {
        mix(&[b"toy", &self.public_key(), msg]).to_vec()
    }
}
impl Verifier for Toy {
    fn verify(&self, alg: i64, pk: &[u8], msg: &[u8], sig: &[u8]) -> bool {
        alg == 1 && sig == mix(&[b"toy", pk, msg])
    }
}

fn doc_bytes(n: u64) -> Vec<u8> {
    let mut st = Store::create_bytes(1 << 20).unwrap();
    let key = Key { actor_id: Toy.actor_id(), alg: 1, valid_from: 0, revoked_at: 0, admit_rec: None, pk: Toy.public_key() };
    Doc::init_bytes(&mut st, 0, &[key]).unwrap();
    let mut tip = [0u8; 32];
    for seq in 1..=n {
        let payload = format!("event-{seq}").into_bytes();
        let id = Toy.chained(&tip, &payload);
        let r = Record { id, prev: tip, actor_pubkey: Toy.actor_id(), actor_seq: seq, flags: 0, payload };
        Doc::commit_bytes(&mut st, &r, &Toy, 0, None).unwrap();
        tip = id;
    }
    st.to_bytes_trimmed()
}

#[test]
fn verify_doc_counts_from_bytes_under_the_last_commit_rule() {
    let bytes = doc_bytes(3);
    let rep = verify_doc(&bytes, &Toy, &Toy, Trust::Genesis).unwrap();
    let c = rep.doc.counters;
    assert_eq!((c.records, c.chained, c.seals_ok, c.seals_bad, c.forks), (3, 3, 3, 0, 0), "{rep:?}");
    assert_eq!(rep.generation, 4, "init + three commits");
    assert!(matches!(rep.via, crate::sim::Via::Anchor { .. }), "the Rust writer anchors: {:?}", rep.via);

    // Damage under the anchor is the anchor's refusal, before any counting.
    let mut torn = bytes.clone();
    let last = torn.len() - 8;
    torn[last] ^= 1;
    assert!(matches!(verify_doc(&torn, &Toy, &Toy, Trust::Genesis), Err(Refusal::LastCommit(_)) | Err(Refusal::BadCrc(_))));
}

/// THE NATIVE KV READERS REFUSE A DOC BY NAME, the same word the other four say
/// (`tools/gates/conformance.sh`, fixture `doc`): root cell 5 = 5 > 4.
#[test]
fn the_kv_readers_refuse_a_doc_root_by_version_name() {
    let bytes = doc_bytes(1);
    let st = Store::from_bytes(&bytes);
    assert!(matches!(Kv::load_checked(&st), Err(KvError::UnknownVersion { v: 5 })));
    assert!(matches!(Db::from_bytes(&bytes, Options::detached()), Err(Refusal::UnknownVersion(5))));
    // Twin: a KV image is not a DOC.
    let kv = Db::create_detached(Options::detached()).unwrap().snapshot_bytes();
    assert!(matches!(verify_doc(&kv, &Toy, &Toy, Trust::Genesis), Err(Refusal::Corrupt("the root is not a DOC root"))));
}
