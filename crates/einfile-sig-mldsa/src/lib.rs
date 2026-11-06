// SPDX-License-Identifier: MIT OR Apache-2.0
//! ML-DSA-65 behind `einfile_log::doc::{Signer, Verifier}`, for whoever writes or checks a sealed
//! einfile image. dowiz-hub uses these two types as they stand, so a stranger under MIT runs what
//! the hub runs.
//!
//! The signature is EXTERNAL PURE `ML-DSA.Sign` of FIPS 204 with the empty context,
//! `M' = 0x00 0x00 ‖ msg` (FORMAT §9.4): what any stock library calls plain signing. The domain
//! is separated by the message itself (`ein-seal/1`, `ein-anchor/1`). `rnd = 0^32` is this
//! writer's choice, not the format's: a verifier cannot tell.
//!
//! The DECART for `fips204` is in this crate's Cargo.toml.
#![no_std]
#![deny(unsafe_code)]

extern crate alloc;

use alloc::boxed::Box;
use alloc::vec::Vec;

use einfile_log::doc::{id_message, Ids, Key, Signer, Verifier, ALG_ML_DSA_65};
use einfile_log::{Record, FLAG_SHA256, NONCE_LEN};
use fips204::ml_dsa_65::{self, PrivateKey, PublicKey, KG};
use fips204::traits::{KeyGen as _, SerDes as _, Signer as _, Verifier as _};
use sha2::{Digest as _, Sha256};

/// The public key, 1 952 bytes.
pub const PK_BYTES: usize = ml_dsa_65::PK_LEN;
/// A signature, 3 309 bytes.
pub const SIG_BYTES: usize = ml_dsa_65::SIG_LEN;
/// The seed `KeyGen` takes, and all a device keeps.
pub const SEED_BYTES: usize = 32;

/// The context of every signature in the format. Not a parameter: a second value would be a
/// second format.
const CTX: &[u8] = b"";
/// `rnd = 0^32`. A device has no entropy to spend on a signature it can make without one.
const RND: [u8; 32] = [0; 32];

/// `actor_id` of a public key: sha256 over its bytes (R-MAINGOAL-A §1.1).
pub fn actor_id(pk: &[u8]) -> [u8; 32] {
    Sha256::digest(pk).into()
}

/// ML-DSA-65 as the DOC's `Verifier`. An algorithm number other than 65, or a key or a signature
/// of the wrong length, or a key that does not parse, is `false`: never a guess at another scheme.
pub struct MlDsa65;

impl Verifier for MlDsa65 {
    fn verify(&self, alg: i64, pk: &[u8], msg: &[u8], sig: &[u8]) -> bool {
        if alg != ALG_ML_DSA_65 {
            return false;
        }
        let (Ok(pk), Ok(sig)) = (<[u8; PK_BYTES]>::try_from(pk), <&[u8; SIG_BYTES]>::try_from(sig)) else {
            return false;
        };
        PublicKey::try_from_bytes(pk).is_ok_and(|pk| pk.verify(msg, sig, CTX))
    }
    fn actor_of(&self, pk: &[u8]) -> Option<[u8; 32]> {
        Some(actor_id(pk))
    }
}

/// One device's ML-DSA-65 key, derived from a 32-byte seed (FIPS 204 `KeyGen`). The secret is
/// boxed so that moving the key copies no secret bytes; `fips204` wipes it on drop.
pub struct DeviceKey {
    pk: Vec<u8>,
    sk: Box<PrivateKey>,
    id: [u8; 32],
}

impl DeviceKey {
    pub fn from_seed(seed: &[u8; SEED_BYTES]) -> Self {
        let (pk, sk) = KG::keygen_from_seed(seed);
        let pk = pk.into_bytes().to_vec();
        let id = actor_id(&pk);
        DeviceKey { pk, sk: Box::new(sk), id }
    }
    pub fn public_key(&self) -> &[u8] {
        &self.pk
    }
    pub fn id(&self) -> [u8; 32] {
        self.id
    }
    /// The KEYS row for this key, as a genesis key (`admit_rec` = none).
    pub fn key(&self) -> Key {
        Key { actor_id: self.id, alg: ALG_ML_DSA_65, valid_from: 0, revoked_at: 0, admit_rec: None, pk: self.pk.clone() }
    }
}

impl Signer for DeviceKey {
    fn actor_id(&self) -> [u8; 32] {
        self.id
    }
    fn alg(&self) -> i64 {
        ALG_ML_DSA_65
    }
    fn public_key(&self) -> Vec<u8> {
        self.pk.clone()
    }
    /// Deterministic, so two signings of one message are one signature. A panic here is 814
    /// rejected attempts on a key `KeyGen` made (FIPS 204 Appendix C), which `Signer` has no
    /// error for; a seal with no signature would be the silent version of the same fault.
    fn sign(&self, msg: &[u8]) -> Vec<u8> {
        self.sk.try_sign_with_seed(&RND, msg, CTX).expect("ML-DSA-65 signing failed").to_vec()
    }
}

/// FORMAT §9.4, rule 3: a tombstone is marked by the high bit of the kind byte, the first byte
/// behind the nonce.
const TOMBSTONE_BIT: u8 = 0x80;
/// RFC 9162 §2.1.1: the domain bytes of a leaf and of an interior node.
const LEAF: u8 = 0x00;
const NODE: u8 = 0x01;

/// The ready-made id scheme: what a user of this crate needs to seal, verify, cut and merge with
/// nothing of their own. It is the scheme dowiz-hub and `bench` write under, read off FORMAT §9.4:
///
/// * a record's id is `sha256(id_message(..))`;
/// * with `FLAG_SHA256` the payload starts with a 16-byte nonce and the body follows it;
/// * a tombstone is the high bit of the body's first byte;
/// * a cut segment is named by the RFC 9162 tree over the cut ids, leaf `sha256(0x00 ‖ id)`.
///
/// No legacy scheme: a record whose id is not its sha256 is `broken`. Bring your own `Ids` when your
/// payload framing differs, for instance when only some record kinds carry a key-op body.
///
/// ```
/// use einfile_image::Store;
/// use einfile_log::doc::{verify, Doc, Ids as _, Signer as _, Trust};
/// use einfile_log::{Record, FLAG_SHA256};
/// use einfile_sig_mldsa::{DeviceKey, MlDsa65, Sha256Ids};
///
/// let key = DeviceKey::from_seed(&[7u8; 32]); // keep the seed: it is the whole secret
/// let mut st = Store::create_bytes(1 << 20).unwrap();
/// Doc::init_bytes(&mut st, 0, &[key.key()]).unwrap();
///
/// // one record: a 16-byte nonce, then the body
/// let mut r = Record { id: [0; 32], prev: [0; 32], actor_pubkey: key.actor_id(), actor_seq: 1,
///                      flags: FLAG_SHA256, payload: b"0123456789abcdefhello".to_vec() };
/// r.id = Sha256Ids.chained(&r);
/// Doc::commit_bytes(&mut st, &r, &key, 1_700_000_000_000, None).unwrap();
///
/// let report = verify(&st, &Sha256Ids, &MlDsa65, Trust::Genesis).unwrap();
/// assert_eq!(report.counters.seals_ok, 1);
/// assert!(report.holds());
/// ```
pub struct Sha256Ids;

impl Ids for Sha256Ids {
    fn chained(&self, r: &Record) -> [u8; 32] {
        Sha256::digest(id_message(r.flags, &r.actor_pubkey, r.actor_seq, &r.prev, &r.payload)).into()
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
        Some(Sha256::digest(bytes).into())
    }
    fn tree_root(&self, ids: &[[u8; 32]]) -> Option<[u8; 32]> {
        let leaves: Vec<[u8; 32]> = ids.iter().map(|id| Sha256::new().chain_update([LEAF]).chain_update(id).finalize().into()).collect();
        Some(mth(&leaves))
    }
}

/// RFC 9162 §2.1.1, the Merkle Tree Hash over leaf hashes: the empty tree is `sha256("")`, and
/// a tree of n > 1 splits at the largest power of two below n.
fn mth(leaves: &[[u8; 32]]) -> [u8; 32] {
    match leaves.len() {
        0 => Sha256::digest([]).into(),
        1 => leaves[0],
        n => {
            let mut k = 1;
            while k << 1 < n {
                k <<= 1;
            }
            Sha256::new().chain_update([NODE]).chain_update(mth(&leaves[..k])).chain_update(mth(&leaves[k..])).finalize().into()
        }
    }
}
