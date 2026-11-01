//! THE PROVIDER: a real ML-DSA-65 (FIPS 204) behind `doc::Signer` / `doc::Verifier`, from
//! dowiz-core (AGPL, feature `mldsa`). Nothing else in this crate names it. The ids are sha256 and
//! the actor id is sha256(public key), the rules of the hub's adapter and of `tools/verify`.
use bebop_store::evlog::doc::{id_message, Ids, Signer, Verifier, ALG_ML_DSA_65};
use bebop_store::evlog::{Record, FLAG_SHA256};
use bebop_store::merkle;
use bebop_store::nodekey::sha256;
use dowiz_core::pq::dsa::{keygen, sign, verify_internal_bytes, MlDsa65Pk, MlDsa65Sk, PUBLICKEYBYTES, SIGNATUREBYTES};

pub const NONCE_LEN: usize = 16;
/// FORMAT §9.4: the mark, the high bit of the byte behind the nonce.
const TOMBSTONE_BIT: u8 = 0x80;

pub struct Sha256Ids;

impl Ids for Sha256Ids {
    fn chained(&self, r: &Record) -> [u8; 32] {
        sha256(&id_message(r.flags, &r.actor_pubkey, r.actor_seq, &r.prev, &r.payload))
    }
    fn redacted(&self, flags: i64, payload: &[u8]) -> bool {
        self.body(flags, payload).first().is_some_and(|k| k & TOMBSTONE_BIT != 0)
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
    fn tree_root(&self, ids: &[[u8; 32]]) -> Option<[u8; 32]> {
        let leaves: Vec<_> = ids.iter().map(|id| merkle::leaf_hash(id)).collect();
        Some(merkle::root(&leaves))
    }
}

/// One device's key. Deterministic signing (FIPS 204 `rnd` = 0^32) keeps a run's bytes repeatable.
pub struct MlDsa {
    pk: MlDsa65Pk,
    sk: MlDsa65Sk,
    id: [u8; 32],
}

impl MlDsa {
    pub fn from_seed(seed: [u8; 32]) -> Self {
        let (pk, sk) = keygen(&seed);
        let id = sha256(&pk.bytes);
        MlDsa { pk, sk, id }
    }
}

impl Signer for MlDsa {
    fn actor_id(&self) -> [u8; 32] {
        self.id
    }
    fn alg(&self) -> i64 {
        ALG_ML_DSA_65
    }
    fn public_key(&self) -> Vec<u8> {
        self.pk.bytes.clone()
    }
    fn sign(&self, msg: &[u8]) -> Vec<u8> {
        sign(&self.sk, msg, &[0u8; 32]).bytes
    }
}

/// Every seal checked.
pub struct MlDsaVerifier;

impl Verifier for MlDsaVerifier {
    fn verify(&self, alg: i64, pk: &[u8], msg: &[u8], sig: &[u8]) -> bool {
        alg == ALG_ML_DSA_65 && pk.len() == PUBLICKEYBYTES && sig.len() == SIGNATUREBYTES && verify_internal_bytes(pk, msg, sig)
    }
    fn actor_of(&self, pk: &[u8]) -> Option<[u8; 32]> {
        Some(sha256(pk))
    }
}

/// A reader with no signature code: it walks seals and proves none (`verify` then counts each one
/// in `seals_bad`, tampered or not), what the file's own embedded reader does. The `verify_counted` row is `verify_signed` with this behind the seam, so the two
/// rows' difference is the price of ML-DSA.
pub struct Counts;

impl Verifier for Counts {
    fn verify(&self, _: i64, _: &[u8], _: &[u8], _: &[u8]) -> bool {
        false
    }
    fn checks_signatures(&self) -> bool {
        false
    }
}
