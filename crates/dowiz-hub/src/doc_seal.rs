//! THE SEAL IN THE HUB (W-MG-M1B, R-MAINGOAL-A §1.2/§2d/§2e, phases M1 and M6): the three
//! seams of `einstore::evlog::doc` filled with this crate's own answers --
//!
//!   * `HubIds`   -- how a record's id is derived: `chain_id` (sha256 over
//!                   `doc::id_message`), the two FNV schemes of old images as LEGACY, the redaction
//!                   mark of `forget.rs`, and the body behind M0's nonce and the kind byte;
//!   * `MlDsa65`  -- the verifier: ML-DSA-65 (FIPS 204) from the `einfile-sig-mldsa` crate, alg 65,
//!                   pk 1 952 B, sig 3 309 B, external pure with the empty context (FORMAT
//!                   §9.4). NEVER `pq::hybrid_signing` (memory `pq-hybrid-signing-leaks-key.md`:
//!                   one signature recovers its secret);
//!   * `DeviceKey` -- the signer, from the same crate: one ML-DSA-65 key per device,
//!                   `actor_id = sha256(pk)`, deterministic signing (`rnd = 0^32`). The price
//!                   root is a different object on a different interface
//!                   (`workers/api/src/hubdo/publish/root.rs`: `Sign_internal`);
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

use einstore::evlog::doc::Ids;
use einstore::evlog::{Record, FLAG_SHA256};
use einstore::merkle;

use crate::chain::{legacy_scheme, record_id, NONCE_LEN};
use crate::forget::REDACTED_BIT;

mod history;
pub mod live;
pub mod merge;
pub mod ots;
pub mod roughtime;
mod sealed;
mod sides;
mod stock;
pub mod tsa;
pub use history::KIND_DECL;
pub use sealed::{SealedHub, KIND_KEYOP, MIN_SEALED_BYTES};
pub use stock::KIND_STOCK;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod cut_tests;
#[cfg(test)]
mod sig_tests;
// `actor_id` is sha256 over a public key (R-MAINGOAL-A §1.1): the convention belongs to the
// signature crate, `einfile-log` never derives it.
pub use einfile_sig_mldsa::{actor_id, DeviceKey, MlDsa65};

/// How this crate's records are named: M0's scheme, the legacy schemes, the redaction mark,
/// the framing. Every answer is `chain::scheme`'s, called -- nothing re-derived here.
pub struct HubIds;

impl Ids for HubIds {
    fn chained(&self, r: &Record) -> [u8; 32] {
        record_id(r)
    }
    /// A pre-M0 id: FNV over `prev ‖ payload` (2026-09-20 to M0) or over the payload alone.
    /// A record FLAGGED sha256 whose id only matches FNV is not legacy -- it was edited and
    /// repaired under the wrong scheme (`verify_id`'s rule) -- and `verify` counts it broken.
    fn legacy(&self, flags: i64, prev: &[u8; 32], payload: &[u8], id: &[u8; 32]) -> bool {
        flags & FLAG_SHA256 == 0 && legacy_scheme(prev, payload, id).is_some()
    }
    fn redacted(&self, flags: i64, payload: &[u8]) -> bool {
        after_nonce(flags, payload).first().is_some_and(|k| k & REDACTED_BIT != 0)
    }
    /// Behind the nonce (when `FLAG_SHA256`) and the kind byte: where a key-op body starts. ONLY
    /// kinds `K` and `D` have one: any other record answers empty, or an order id that spells a
    /// mark would read as a declaration (`atk_holes` hole 7).
    fn body<'a>(&self, flags: i64, payload: &'a [u8]) -> &'a [u8] {
        match after_nonce(flags, payload).split_first() {
            Some((&KIND_KEYOP | &KIND_DECL, body)) => body,
            _ => &[],
        }
    }
    /// The root a cut segment is named by (W-MG-M3's `Doc::cut_into`): RFC 9162 over the ids
    /// in file order, `einstore::merkle` -- the same tree `rotate.rs` publishes for an
    /// archive, so a segment's root and an archive's root over the same ids are one value.
    fn hash(&self, bytes: &[u8]) -> Option<[u8; 32]> {
        Some(einstore::nodekey::sha256(bytes))
    }
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

/// `Hub`-style hex of an id (`chain::hex32`), for the CLI and the tests.
pub fn hex32(b: &[u8; 32]) -> String {
    crate::hex32(b)
}

/// THE THREE SENTENCES of R-MAINGOAL-A §2d, as the code stands: what the proof says, and the one
/// line after them saying what it does not. §2d planned `ref TIME` on the seal and other kind
/// numbers; a seal's TIME is 0, a lower bound comes from a Roughtime anchor, and `tests.rs` holds
/// the numbers here to the `KIND_` constants. A verifier prints these and nothing stronger.
pub const SENTENCES: [&str; 4] = [
    "(1) ці байти підписав власник ключа K у ланцюгу з номером seq, і після печатки їх не змінювали;",
    "(2) якщо є перевірений якір виду 1 (TSA-мітка платформи) або 2 (OpenTimestamps) — байти існували **не пізніше** T_anchor;",
    "(3) якщо є перевірений якір виду 3 (Roughtime) — **не раніше** T_roughtime − radius.",
    "Доказ **не** каже, що подія сталась, що кількість правдива, що `claimed_ms` правильний.",
];
