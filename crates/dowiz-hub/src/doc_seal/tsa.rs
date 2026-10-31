//! ANCHOR KIND 1: THE PLATFORM TSA MARK (R-MAINGOAL-A §3, choice 1). The platform signs
//! `"bebop-anchor/1" ‖ kind ‖ over ‖ time_ms` with its own ML-DSA-65 key at every sync --
//! immediate, free, offline-checkable, and it proves EXACTLY "dowiz's key saw this tip by
//! then": a bound against dowiz, not against the world (that is kind 2). An RFC 3161
//! TSTInfo would carry the same four things in ASN.1; this is the same claim in the
//! format's own bytes, so the verifier is 20 lines and not a DER parser.
//!
//! The proof is `pk (1 952) ‖ sig (3 309)` = 5 261 bytes, and the verifier takes the
//! platform keys FROM THE AUDITOR (`PlatformTsa::keys`), never from the proof: a key that
//! travels with the signature it is checked by proves only that someone owns a key. A mark
//! by a key the auditor did not bring is `Unchecked`, with the key's id so it can be
//! brought; a mark that does not verify under a brought key is `Bad`.

use bebop_store::evlog::doc::{Anchor, AnchorCheck, Verdict, KIND_PLATFORM_TSA};
use dowiz_core::pq::dsa;

use super::{actor_id, DeviceKey, MlDsa65};
use bebop_store::evlog::doc::Verifier;

pub const ANCHOR_PREFIX: &[u8] = b"bebop-anchor/1";
pub const MARK_BYTES: usize = dsa::PUBLICKEYBYTES + dsa::SIGNATUREBYTES;

/// The bytes a mark signs.
pub fn message(kind: i64, over: &[u8; 32], time_ms: i64) -> Vec<u8> {
    let mut m = Vec::with_capacity(ANCHOR_PREFIX.len() + 8 + 32 + 8);
    m.extend_from_slice(ANCHOR_PREFIX);
    m.extend_from_slice(&kind.to_le_bytes());
    m.extend_from_slice(over);
    m.extend_from_slice(&time_ms.to_le_bytes());
    m
}

/// The mark: `pk ‖ sig` by the platform key over `message(KIND_PLATFORM_TSA, over, time_ms)`.
/// What the DO writes into an ANCH at sync (`SealedHub::anchor(KIND_PLATFORM_TSA, ..)`).
pub fn mark(platform: &DeviceKey, over: &[u8; 32], time_ms: i64) -> Vec<u8> {
    let mut p = Vec::with_capacity(MARK_BYTES);
    p.extend_from_slice(platform.public_key());
    p.extend_from_slice(&dsa::sign_internal_bytes(&platform.sk, &message(KIND_PLATFORM_TSA, over, time_ms), &[0u8; dsa::RNDBYTES]));
    p
}

/// The checker: the platform public keys the auditor brings (any number; none = nothing
/// can be checked and every mark is `Unchecked`).
pub struct PlatformTsa<'a> {
    pub keys: &'a [Vec<u8>],
}

fn hex8(id: &[u8; 32]) -> String {
    super::hex32(id).chars().take(16).collect()
}

impl AnchorCheck for PlatformTsa<'_> {
    fn check(&self, a: &Anchor) -> Verdict {
        if a.kind != KIND_PLATFORM_TSA {
            return Verdict::Unchecked(format!("kind {} is not a platform TSA mark", a.kind));
        }
        if a.proof.len() != MARK_BYTES {
            return Verdict::Bad(format!("a TSA mark is {MARK_BYTES} bytes; this one is {}", a.proof.len()));
        }
        let (pk, sig) = a.proof.split_at(dsa::PUBLICKEYBYTES);
        let id = actor_id(pk);
        if self.keys.is_empty() {
            return Verdict::Unchecked(format!("no platform key was given; the mark names key {}", hex8(&id)));
        }
        if !self.keys.iter().any(|k| k.as_slice() == pk) {
            return Verdict::Unchecked(format!("the mark's key {} is not one of the platform keys given", hex8(&id)));
        }
        if MlDsa65.verify(bebop_store::evlog::doc::ALG_ML_DSA_65, pk, &message(a.kind, &a.over, a.time_ms), sig) {
            Verdict::Ok { not_after_ms: Some(a.time_ms), not_before_ms: None, what: format!("ML-DSA-65 by platform key {}", hex8(&id)) }
        } else {
            Verdict::Bad(format!("the ML-DSA-65 signature by platform key {} does not verify over this over/time", hex8(&id)))
        }
    }
}
