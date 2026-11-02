// SPDX-License-Identifier: MIT OR Apache-2.0
//! What the signed-file examples share: a device, the id rule, and A STAND-IN FOR A SIGNATURE.
//!
//! `AnyoneCanForge` IS NOT A SIGNATURE. Its "signature" is a hash of public values, so anyone can
//! make one. This library's MIT crates hold no signature code: a real writer brings an ML-DSA-65
//! implementation to the `doc::Signer` and `doc::Verifier` traits, and the examples would run the
//! same way with it. Nothing an example prints about a seal proves anything about who signed.
//!
//! `Sha256Ids` is real. It is the rule the no-install verifiers in `tools/verify` apply: a record's
//! id is `sha256(prev ‖ payload)`, the payload starts with a 16-byte nonce when `FLAG_SHA256` is
//! set, and a nonce of sixteen ASCII zeros marks a redacted record.

use einstore::evlog::doc::{Doc, DocError, Ids, Key, KeyOp, Signer, Verifier};
use einstore::evlog::{Record, FLAG_SHA256};
use einstore::nodekey::sha256;
use einstore::evlog::doc::id_message;
use einstore::{merkle, Store};

pub const NONCE_LEN: usize = 16;
pub const ERASED_NONCE: [u8; NONCE_LEN] = [b'0'; NONCE_LEN];
/// FORMAT §9.4: the mark, the high bit of the byte behind the nonce. The erased nonce is no mark.
pub const TOMBSTONE_BIT: u8 = 0x80;
/// Not an algorithm of the format: `ALG_ML_DSA_65` (65) is the one a real file uses.
pub const ALG_NOT_A_SIGNATURE: i64 = 9000;

pub struct Sha256Ids;

impl Ids for Sha256Ids {
    fn chained(&self, r: &Record) -> [u8; 32] {
        sha256(&id_message(r.flags, &r.actor_pubkey, r.actor_seq, &r.prev, &r.payload))
    }
    fn redacted(&self, flags: i64, payload: &[u8]) -> bool {
        let tail = if flags & FLAG_SHA256 != 0 { payload.get(NONCE_LEN..).unwrap_or(&[]) } else { payload };
        tail.first().is_some_and(|k| k & TOMBSTONE_BIT != 0)
    }
    fn body<'a>(&self, flags: i64, payload: &'a [u8]) -> &'a [u8] {
        if flags & FLAG_SHA256 != 0 {
            payload.get(NONCE_LEN..).unwrap_or(&[])
        } else {
            payload
        }
    }
    fn hash(&self, bytes: &[u8]) -> Option<[u8; 32]> {
        Some(sha256(bytes))
    }
    /// The RFC 9162 tree over the ids, in file order.
    fn tree_root(&self, ids: &[[u8; 32]]) -> Option<[u8; 32]> {
        let leaves: Vec<_> = ids.iter().map(|id| merkle::leaf_hash(id)).collect();
        Some(merkle::root(&leaves))
    }
}

/// See the module header: this is not a signature.
pub struct NotASignature {
    pk: [u8; 32],
}

fn fake_sig(pk: &[u8], msg: &[u8]) -> Vec<u8> {
    sha256(&[&b"not a signature"[..], pk, msg].concat()).to_vec()
}

impl Signer for NotASignature {
    fn actor_id(&self) -> [u8; 32] {
        sha256(&self.pk)
    }
    fn alg(&self) -> i64 {
        ALG_NOT_A_SIGNATURE
    }
    fn public_key(&self) -> Vec<u8> {
        self.pk.to_vec()
    }
    fn sign(&self, msg: &[u8]) -> Vec<u8> {
        fake_sig(&self.pk, msg)
    }
}

/// The verifier half of the stand-in.
pub struct AnyoneCanForge;

impl Verifier for AnyoneCanForge {
    fn verify(&self, alg: i64, pk: &[u8], msg: &[u8], sig: &[u8]) -> bool {
        alg == ALG_NOT_A_SIGNATURE && sig == fake_sig(pk, msg)
    }
}

/// One device: its key and its position in its own chain.
pub struct Device {
    pub signer: NotASignature,
    pub seq: u64,
    pub tip: [u8; 32],
}

impl Device {
    pub fn new(name: u8) -> Device {
        Device { signer: NotASignature { pk: [name; 32] }, seq: 0, tip: [0; 32] }
    }

    pub fn key(&self) -> Key {
        Key { actor_id: self.signer.actor_id(), alg: self.signer.alg(), valid_from: 0, revoked_at: 0, admit_rec: None, pk: self.signer.public_key() }
    }

    /// One commit: one record, one seal over it, in one transaction. A real nonce comes from the
    /// operating system's random source; a counter keeps these examples repeatable.
    pub fn commit(&mut self, st: &mut Store, body: &[u8], op: Option<&KeyOp>) -> Result<i64, DocError> {
        let seq = self.seq + 1;
        let payload = [format!("{seq:016x}").as_bytes(), body].concat();
        let mut rec = Record { id: [0u8; 32], prev: self.tip, actor_pubkey: self.signer.actor_id(), actor_seq: seq, flags: FLAG_SHA256, payload };
        rec.id = Sha256Ids.chained(&rec);
        let id = rec.id;
        let generation = Doc::commit_bytes(st, &rec, &self.signer, 1_700_000_000_000 + seq as i64, op)?;
        self.seq = seq;
        self.tip = id;
        Ok(generation)
    }
}
