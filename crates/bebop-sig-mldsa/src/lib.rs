// SPDX-License-Identifier: MIT OR Apache-2.0
//! ML-DSA-65 behind `bebop_log::doc::{Signer, Verifier}`, for whoever writes or checks a sealed
//! bebop image. dowiz-hub uses these two types as they stand, so a stranger under MIT runs what
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

use bebop_log::doc::{Key, Signer, Verifier, ALG_ML_DSA_65};
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
