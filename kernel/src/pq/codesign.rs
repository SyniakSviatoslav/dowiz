//! Code-signing of node update blobs (firmware/OTA) against a pinned root.
//!
//! P3 deliverable: a node update blob is an opaque byte string (firmware image,
//! config bundle, …). It MUST be authenticated by an ML-DSA-65 signature over the
//! blob before it may be `apply`-ed. The verifier trusts exactly ONE pinned root
//! public key — nothing else is accepted, regardless of who presents it.
//!
//! This module is a thin, opinionated layer over the PQ envelope seam
//! (`crate::pq::envelope`): it reuses the same FIPS-204 ML-DSA-65 primitives and
//! the same content-addressed integrity model. It adds:
//!   - a pinned-root abstraction (`PinnedRoot`),
//!   - an `UpdateBlob` that distinguishes *signed* from *unsigned* (RED gate),
//!   - an `apply` stub that records the applied blob hash in a ledger.
//!
//! RNG-free hot path (C10): all entropy is caller-supplied (`seed`, `rnd`).

use crate::pq::envelope::{
    new_identity, open, seal, EnvelopeError, SignedEnvelope, HASH_LEN,
};
use crate::pq::envelope::ENTROPY_LEN;

/// An ML-DSA-65 signing key (secret key bytes). Used only to *produce* updates.
#[derive(Clone)]
pub struct SigningKey {
    pub sk: Vec<u8>,
}

/// The single trusted root. An update blob is accepted *iff* its ML-DSA-65
/// signature verifies under this public key. Tampering with this value is the
/// only way to change trust — there is no fallback path.
#[derive(Clone, PartialEq, Eq)]
pub struct PinnedRoot {
    pub pk: Vec<u8>,
}

/// A signature-wrapped update blob (re-export of the envelope type).
pub type SignedUpdate = SignedEnvelope;

/// An update blob as it arrives at a node: either a properly signed blob, or a
/// raw unsigned blob (which must be refused).
#[derive(Clone)]
pub enum UpdateBlob {
    /// ML-DSA-65 signed blob.
    Signed(SignedUpdate),
    /// Raw, unsigned blob (no signature attached).
    Unsigned(Vec<u8>),
}

/// Reason an update was refused (RED gate) or failed to verify.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CodeSignError {
    /// Blob arrived with no signature at all.
    Unsigned,
    /// Stored content hash ≠ hash(payload). Payload tampered or wrong envelope.
    HashMismatch,
    /// ML-DSA verification rejected (wrong key, tampered sig, or tampered payload).
    BadSignature,
}

impl From<EnvelopeError> for CodeSignError {
    fn from(e: EnvelopeError) -> Self {
        match e {
            EnvelopeError::HashMismatch => CodeSignError::HashMismatch,
            EnvelopeError::BadSignature => CodeSignError::BadSignature,
        }
    }
}

/// Result of a successful `apply`: the authenticated payload plus its hash
/// (already recorded in the ledger by `apply`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedUpdate {
    pub blob_hash: [u8; HASH_LEN],
    pub payload: Vec<u8>,
}

/// Append-only record of hashes that have been successfully applied. The stub
/// `apply` records here; this is the audit trail / replay guard.
#[derive(Debug, Clone, Default)]
pub struct ApplyLedger {
    pub applied: Vec<[u8; HASH_LEN]>,
}

impl ApplyLedger {
    pub fn new() -> Self {
        Self::default()
    }
    /// Record an applied blob hash.
    pub fn record(&mut self, h: [u8; HASH_LEN]) {
        self.applied.push(h);
    }
    /// Has this blob hash been applied before?
    pub fn contains(&self, h: &[u8; HASH_LEN]) -> bool {
        self.applied.contains(h)
    }
}

/// Derive a fresh code-signing identity. Returns the `PinnedRoot` (the public
/// half, to be pinned in the node) and the `SigningKey` (the secret half, kept
/// by the update authority). `seed` is caller entropy (C10).
pub fn codesign_keypair(seed: &[u8; ENTROPY_LEN]) -> (PinnedRoot, SigningKey) {
    let (pk, sk) = new_identity(seed);
    (PinnedRoot { pk }, SigningKey { sk })
}

/// Sign an update blob with the signing key. `rnd` is caller-supplied signing
/// entropy (C10); never reuse it across seals.
pub fn sign_update(
    blob: &[u8],
    key: &SigningKey,
    rnd: &[u8; ENTROPY_LEN],
) -> UpdateBlob {
    UpdateBlob::Signed(seal(blob, &key.sk, rnd))
}

/// Verify `blob` under `root` and, on success, record its hash in `ledger`
/// (the `apply` stub). On any failure the ledger is left unchanged and the
/// error explains why (RED gate: unsigned / tampered / wrong key).
pub fn apply(
    root: &PinnedRoot,
    blob: &UpdateBlob,
    ledger: &mut ApplyLedger,
) -> Result<AppliedUpdate, CodeSignError> {
    let signed = match blob {
        UpdateBlob::Unsigned(_) => return Err(CodeSignError::Unsigned),
        UpdateBlob::Signed(s) => s,
    };
    // open() performs both the content-hash check and the ML-DSA verify.
    let payload = open(signed, &root.pk).map_err(CodeSignError::from)?;
    let blob_hash = crate::pq::envelope::hash32(&payload);
    ledger.record(blob_hash);
    Ok(AppliedUpdate { blob_hash, payload })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed(n: u8) -> [u8; ENTROPY_LEN] {
        [n; ENTROPY_LEN]
    }

    // ── GREEN gate ──────────────────────────────────────────────────────────
    #[test]
    fn green_signed_blob_applies_under_pinned_root() {
        let (root, key) = codesign_keypair(&seed(1));
        let blob = b"\xDE\xAD\xBE\xEF firmware image v2";
        let signed = sign_update(blob, &key, &seed(2));

        let mut ledger = ApplyLedger::new();
        let applied = apply(&root, &signed, &mut ledger).expect("signed blob applies");

        assert_eq!(applied.payload, blob);
        assert_eq!(applied.blob_hash, crate::pq::envelope::hash32(blob));
        assert_eq!(ledger.applied.len(), 1);
        assert!(ledger.contains(&applied.blob_hash));

        // Signature length sanity (no crypto-constant change, just a length check).
        if let UpdateBlob::Signed(s) = &signed {
            assert_eq!(s.sig.len(), crate::pq::envelope::SIG_LEN);
        } else {
            panic!("expected signed blob");
        }
    }

    #[test]
    fn green_different_blobs_distinct_hashes() {
        let (root, key) = codesign_keypair(&seed(3));
        let a = sign_update(b"firmware-A", &key, &seed(4));
        let b = sign_update(b"firmware-B", &key, &seed(5));
        let mut ledger = ApplyLedger::new();
        let pa = apply(&root, &a, &mut ledger).unwrap();
        let pb = apply(&root, &b, &mut ledger).unwrap();
        assert_ne!(pa.blob_hash, pb.blob_hash);
        assert_eq!(ledger.applied.len(), 2);
    }

    // ── RED gate ────────────────────────────────────────────────────────────
    #[test]
    fn red_unsigned_blob_refused() {
        let (root, _key) = codesign_keypair(&seed(6));
        let unsigned = UpdateBlob::Unsigned(b"rogue unsigned blob".to_vec());
        let mut ledger = ApplyLedger::new();
        assert_eq!(
            apply(&root, &unsigned, &mut ledger),
            Err(CodeSignError::Unsigned)
        );
        // Ledger must NOT record a refused update.
        assert!(ledger.applied.is_empty());
    }

    #[test]
    fn red_tampered_signature_refused() {
        let (root, key) = codesign_keypair(&seed(7));
        let signed = sign_update(b"trusted payload", &key, &seed(8));
        // Flip a signature byte to simulate corruption / forgery.
        let mut tampered = signed;
        if let UpdateBlob::Signed(s) = &mut tampered {
            if !s.sig.is_empty() {
                s.sig[0] ^= 0xff;
            }
        }
        let mut ledger = ApplyLedger::new();
        assert_eq!(
            apply(&root, &tampered, &mut ledger),
            Err(CodeSignError::BadSignature)
        );
        assert!(ledger.applied.is_empty());
    }

    #[test]
    fn red_wrong_root_key_refused() {
        // Signing key belongs to a DIFFERENT root than the one pinned in the node.
        let (root, _key) = codesign_keypair(&seed(9));
        let (_other_root, other_key) = codesign_keypair(&seed(10));
        let signed_by_other = sign_update(b"payload", &other_key, &seed(11));

        let mut ledger = ApplyLedger::new();
        // The signed blob is cryptographically valid under other_root's pk — but
        // the node only trusts `root`, so it MUST be refused.
        assert_eq!(
            apply(&root, &signed_by_other, &mut ledger),
            Err(CodeSignError::BadSignature)
        );
        assert!(ledger.applied.is_empty());
    }

    #[test]
    fn red_tampered_payload_refused() {
        let (root, key) = codesign_keypair(&seed(12));
        let mut signed = sign_update(b"firmware v1.0.0", &key, &seed(13));
        // Corrupt the payload after signing (breaks content-hash + signature).
        if let UpdateBlob::Signed(s) = &mut signed {
            if !s.payload.is_empty() {
                s.payload[0] ^= 0xff;
            }
        }
        let mut ledger = ApplyLedger::new();
        assert_eq!(
            apply(&root, &signed, &mut ledger),
            Err(CodeSignError::HashMismatch)
        );
        assert!(ledger.applied.is_empty());
    }
}
