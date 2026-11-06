//! THE PROVIDER: a real ML-DSA-65 (FIPS 204) behind `doc::Signer` / `doc::Verifier`, from
//! einfile-sig-mldsa (feature `mldsa`), the provider dowiz-hub runs. Nothing else in this crate names it. The ids are sha256 and
//! the actor id is sha256(public key), the rules of the hub's adapter and of `tools/verify`.
use einstore::evlog::doc::{id_message, Ids, Signer, Verifier};
use einstore::evlog::{Record, FLAG_SHA256};
use einstore::merkle;
use einstore::nodekey::sha256;
use einfile_sig_mldsa::{DeviceKey, MlDsa65};

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
pub struct MlDsa(DeviceKey);

impl MlDsa {
    pub fn from_seed(seed: [u8; 32]) -> Self {
        MlDsa(DeviceKey::from_seed(&seed))
    }
}

impl Signer for MlDsa {
    fn actor_id(&self) -> [u8; 32] {
        self.0.actor_id()
    }
    fn alg(&self) -> i64 {
        self.0.alg()
    }
    fn public_key(&self) -> Vec<u8> {
        self.0.public_key().to_vec()
    }
    fn sign(&self, msg: &[u8]) -> Vec<u8> {
        self.0.sign(msg)
    }
}

/// Every seal checked.
pub struct MlDsaVerifier;

impl Verifier for MlDsaVerifier {
    fn verify(&self, alg: i64, pk: &[u8], msg: &[u8], sig: &[u8]) -> bool {
        MlDsa65.verify(alg, pk, msg, sig)
    }
    fn actor_of(&self, pk: &[u8]) -> Option<[u8; 32]> {
        MlDsa65.actor_of(pk)
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
