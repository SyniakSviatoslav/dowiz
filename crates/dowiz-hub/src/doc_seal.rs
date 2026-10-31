//! THE SEAL IN THE HUB (W-MG-M1B, R-MAINGOAL-A §1.2/§2d/§2e, phases M1 and M6): the three
//! seams of `bebop_store::evlog::doc` filled with this crate's own answers --
//!
//!   * `HubIds`   -- how a record's id is derived: M0's `chain_id` (sha256 over `prev ‖
//!                   payload`), the two FNV schemes of old images as LEGACY, the redaction
//!                   mark of `forget.rs`, and the body behind M0's nonce and the kind byte;
//!   * `MlDsa65`  -- the verifier: ML-DSA-65 (FIPS 204) from `dowiz_core::pq::dsa`, alg 65,
//!                   pk 1 952 B, sig 3 309 B. NEVER `pq::hybrid_signing` (memory
//!                   `pq-hybrid-signing-leaks-key.md`: one signature recovers its secret);
//!   * `DeviceKey` -- the signer: one ML-DSA-65 key per device, `actor_id = sha256(pk)`,
//!                   deterministic signing (`rnd = 0^32`) as the published price root is signed
//!                   (`workers/api/src/hubdo/publish/root.rs`);
//!
//! and `SealedHub`, A HUB IMAGE UNDER A DOC ROOT. "A hub commit seals its tip": `append` is
//! ONE record and ONE seal in one transaction (`Doc::commit_bytes`), so no generation holds
//! a record its device did not sign for. `seal_hub` takes a REAL hub image (the order log
//! `Hub` writes today) and lays its chain, verbatim, under a DOC whose genesis key is the
//! device's; from then on only `SealedHub` reads it, because every reader written before --
//! `Hub::load`, `LogImage::load`, the five conformance readers -- refuses a root-version 5
//! BY NAME (`HubError::UnknownVersion`, status 12). That is the format's rule, not a gap:
//! the readers learn the DOC in M5, and until then nothing decodes what it does not know.
//!
//! WHY NOT THE `Hub` ITSELF. `Hub` reads its log at the image's root through some eighty
//! `EvLog::*(&self.store)` sites in files other lanes own (`forget.rs`, `rotate.rs`); a DOC
//! names its log BY REF, so each of them would need the `_at(root)` twin. This type is the
//! opt-in path that proves the seal on real hub bytes without touching them.
//!
//! What the seal proves (§2d, printed verbatim by `bin/doc_verify.rs`): the holder of key K
//! signed these bytes at seq s of its chain and nothing under the tip changed since. NOT
//! that the event happened, that a quantity is true, or that `claimed_ms` is right. The
//! anchors (`tsa.rs`, `ots.rs`, `roughtime.rs`) are the time half.

use bebop_store::evlog::doc::{Ids, Key, Signer, Verifier, ALG_ML_DSA_65};
use bebop_store::evlog::FLAG_SHA256;
use bebop_store::merkle;
use dowiz_core::pq::dsa;

use crate::chain::{chain_id, verify_id, IdScheme, NONCE_LEN};
use crate::forget::REDACTED_BIT;

mod history;
pub mod live;
pub mod ots;
pub mod roughtime;
mod sealed;
pub mod tsa;
pub use history::KIND_DECL;
pub use sealed::{SealedHub, KIND_KEYOP, MIN_SEALED_BYTES};
#[cfg(test)]
mod tests;
#[cfg(test)]
mod cut_tests;

/// `actor_id` of a public key: sha256 over its bytes (R-MAINGOAL-A §1.1; the convention is
/// this crate's, the MIT crate never derives it).
pub fn actor_id(pk: &[u8]) -> [u8; 32] {
    bebop_store::nodekey::sha256(pk)
}

/// How this crate's records are named: M0's scheme, the legacy schemes, the redaction mark,
/// the framing. Every answer is `chain::scheme`'s, called -- nothing re-derived here.
pub struct HubIds;

impl Ids for HubIds {
    fn chained(&self, prev: &[u8; 32], payload: &[u8]) -> [u8; 32] {
        chain_id(prev, payload)
    }
    /// A pre-M0 id: FNV over `prev ‖ payload` (2026-09-20 to M0) or over the payload alone.
    /// A record FLAGGED sha256 whose id only matches FNV is not legacy -- it was edited and
    /// repaired under the wrong scheme (`verify_id`'s rule) -- and `verify` counts it broken.
    fn legacy(&self, flags: i64, prev: &[u8; 32], payload: &[u8], id: &[u8; 32]) -> bool {
        matches!(verify_id(flags, prev, payload, id), Some(IdScheme::LegacyChained | IdScheme::LegacyPayload))
    }
    fn redacted(&self, flags: i64, payload: &[u8]) -> bool {
        after_nonce(flags, payload).first().is_some_and(|k| k & REDACTED_BIT != 0)
    }
    /// Behind the nonce (when `FLAG_SHA256`) and the kind byte: where a key-op body starts.
    fn body<'a>(&self, flags: i64, payload: &'a [u8]) -> &'a [u8] {
        after_nonce(flags, payload).get(1..).unwrap_or(&[])
    }
    /// The root a cut segment is named by (W-MG-M3's `Doc::cut_into`): RFC 9162 over the ids
    /// in file order, `bebop_store::merkle` -- the same tree `rotate.rs` publishes for an
    /// archive, so a segment's root and an archive's root over the same ids are one value.
    fn tree_root(&self, ids: &[[u8; 32]]) -> Option<[u8; 32]> {
        let leaves: Vec<[u8; 32]> = ids.iter().map(|id| merkle::leaf_hash(id)).collect();
        Some(merkle::root(&leaves))
    }
}

/// `chain::body_of` on a flags cell and a payload rather than a `Record`.
fn after_nonce(flags: i64, payload: &[u8]) -> &[u8] {
    if flags & FLAG_SHA256 != 0 {
        payload.get(NONCE_LEN..).unwrap_or(&[])
    } else {
        payload
    }
}

/// ML-DSA-65 as the DOC's `Verifier`. An algorithm number other than 65, or a key or a
/// signature of the wrong length, is `false`: never a guess at another scheme.
pub struct MlDsa65;

impl Verifier for MlDsa65 {
    fn verify(&self, alg: i64, pk: &[u8], msg: &[u8], sig: &[u8]) -> bool {
        alg == ALG_ML_DSA_65 && pk.len() == dsa::PUBLICKEYBYTES && sig.len() == dsa::SIGNATUREBYTES && dsa::verify_internal_bytes(pk, msg, sig)
    }
}

/// One device's ML-DSA-65 key, derived from a 32-byte seed (FIPS 204 `KeyGen`); the seed is
/// what a device keeps, as `VENUE_SIGN_SK` is a seed. The platform's own key for the TSA
/// mark is the same type (`tsa::mark`).
pub struct DeviceKey {
    pk: Vec<u8>,
    sk: Vec<u8>,
    id: [u8; 32],
}

impl DeviceKey {
    pub fn from_seed(seed: &[u8; dsa::SEEDBYTES]) -> Self {
        let (pk, sk) = dsa::keygen_bytes(seed);
        let id = actor_id(&pk);
        DeviceKey { pk, sk, id }
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
    /// Deterministic: `rnd = 0^32`. A device has no entropy to spend on a signature it can
    /// make without one, and two signings of one message are then one signature.
    fn sign(&self, msg: &[u8]) -> Vec<u8> {
        dsa::sign_internal_bytes(&self.sk, msg, &[0u8; dsa::RNDBYTES])
    }
}


/// `Hub`-style hex of an id (`chain::hex32`), for the CLI and the tests.
pub fn hex32(b: &[u8; 32]) -> String {
    crate::hex32(b)
}

/// THE THREE SENTENCES of R-MAINGOAL-A §2d, verbatim: what the proof says, and the one line
/// after them saying what it does not. A verifier prints these and nothing stronger.
pub const SENTENCES: [&str; 4] = [
    "(1) ці байти підписав власник ключа K у ланцюгу з номером seq, і після печатки їх не змінювали;",
    "(2) якщо є якір виду 2/3/4 — байти існували **не пізніше** T_anchor;",
    "(3) якщо печатка має `ref TIME` (Roughtime-відповідь на nonce = попередня печатка) — **не раніше** T_roughtime − radius.",
    "Доказ **не** каже, що подія сталась, що кількість правдива, що `claimed_ms` правильний.",
];
