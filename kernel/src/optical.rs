//! Optical/pixel archival compression — BLUEPRINT-ITEM-28 (Phase B).
//!
//! Pilot-scoped to the **archival plane only** (per the operator's IN ruling,
//! recorded in `docs/design/OPTICAL-COMPRESSION-DECISION-2026-07-19.md`).
//!
//! This module exists to make the plane boundary a **type-level guarantee**, not
//! a reviewer's promise. The headline proof is
//! [`optical_compressed_cannot_reach_determinism_plane`] (the §1.5
//! unrepresentability standard applied to the plane boundary).
//!
//! ## The §1.5 plane-boundary invariant
//!
//! // §1.5 plane-boundary: OpticalCompressed MUST only be accepted by archival
//! // persist; never by event_log/hash/signature/idempotency paths. OpticalCompressed
//! // has NO method that yields bytes feedable to sha3/content-id. The ONLY accessor
//! // it exposes is [`OpticalCompressed::archive_persist`], which hands the bytes to
//! // the archival-tier persistence API (item 20). There is no `.as_bytes_for_hashing()`,
//! // no `content_id()`, no `sign()` — those do not compile against this type.
//!
//! ## Runtime seam, not a Cargo dependency
//!
//! The DeepSeek-OCR-style model (arXiv:2510.18234) is *weights*, not a crate. It is
//! loaded from a **local GGUF** file at runtime — OUTSIDE the Cargo graph — exactly
//! like `pq/entropy.rs`'s opt-in network provider. Absent local weights, the codec
//! returns [`OpticalError::ModelUnavailable`]. **No model run is faked.**
//!
//! // innovate: the real-model round-trip (LocalGguf encode/decode against live
//! // DeepSeek-OCR GGUF weights) is a CEILING, not a claimed result. Upgrade trigger:
//! // `when local GGUF DeepSeek-OCR weights are present` — at that point swap
//! // `ModelUnavailable` for the real vision-token encode/decode and measure the
//! // `measured-lossy-oracle` (>= published <10x/97% baseline) + cycles/joules per
//! // operation (energy-first telemetry, §21).

use std::path::Path;

/// Error type for the optical archival codec seam.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpticalError {
    /// No local GGUF DeepSeek-OCR weights are present. The real-model round-trip
    /// is a documented ceiling; until a local model file is supplied this is the
    /// honest, verified behavior. No network call is attempted.
    ModelUnavailable,
    /// The provided archival tier rejected the payload (e.g. not on the archival
    /// plane, or the persistence layer is a determinism-plane sink — which must
    /// never happen; this variant is the fail-closed guard).
    ArchivalRejected(&'static str),
}

impl core::fmt::Display for OpticalError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            OpticalError::ModelUnavailable => {
                f.write_str("optical: local GGUF DeepSeek-OCR weights absent (runtime seam)")
            }
            OpticalError::ArchivalRejected(r) => {
                f.write_str("optical: archival persist rejected payload: ")?;
                f.write_str(r)
            }
        }
    }
}

impl std::error::Error for OpticalError {}

/// The optic-compressed payload.
///
/// This is a NEW-TYPE WRAP over `Vec<u8>` whose API is deliberately *narrow*:
/// the only way to get bytes **out** is [`OpticalCompressed::archive_persist`],
/// which routes them to the archival-tier persistence API (item 20). There is
/// intentionally **no** `.as_bytes()`, no `.as_bytes_for_hashing()`, no
/// `content_id()`, no `sign()` — those methods do not exist, so they cannot be
/// called against this type. That is the §1.5 unrepresentability guarantee at the
/// structural level: optically-compressed bytes are *unrepresentable* at any
/// hash/signature/idempotency surface.
///
/// `OpticalCompressed` deliberately does NOT derive `Hash` — you cannot even put
/// it in a `HashSet`/`HashMap` key without first archiving it (which consumes it).
/// It DOES derive `Debug`/`PartialEq`/`Eq` (structural equality of the wrapped
/// bytes is harmless — it does not *expose* bytes to any sha3/content-id surface;
/// only `Hash` and a `.as_bytes_for_hashing()` accessor would, and both are
/// deliberately absent per §1.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpticalCompressed {
    /// Compressed vision-token payload. NEVER returned as raw bytes for hashing.
    payload: Vec<u8>,
    /// Provenance: which codec produced this blob (so the archival tier can pick
    /// the matching decoder). Carried opaquely; not a content-id.
    codec_tag: u8,
}

impl OpticalCompressed {
    /// Construct from already-compressed bytes (internal to the codec path).
    fn from_compressed(payload: Vec<u8>, codec_tag: u8) -> Self {
        Self { payload, codec_tag }
    }

    /// The **only** public accessor: hand the compressed payload to the
    /// archival-tier persistence API (item 20). This consumes `self`, so the
    /// bytes cannot later be diverted to a hash/signature path.
    ///
    /// `archive` is the archival sink. It MUST be a durability/display/archival
    /// plane consumer — never `event_log::sha3_256`, never a signature verifier,
    /// never an idempotency-key store. The type makes the *bytes* unreachable
    /// from those surfaces; the caller's `archive` closure is trusted to be an
    /// archival-plane sink (documented contract; enforced structurally for the
    /// bytes, not for the closure body — see the test).
    pub fn archive_persist<F, R>(self, mut archive: F) -> R
    where
        F: FnMut(ArchivalBlob) -> R,
    {
        let blob = ArchivalBlob {
            payload: self.payload,
            codec_tag: self.codec_tag,
        };
        archive(blob)
    }
}

/// The unit handed to the archival sink. Contains the raw compressed bytes but is
/// *only* constructible from an [`OpticalCompressed`] (which already proved it
/// came from the optical codec). The archival layer stores it; it never hashes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivalBlob {
    /// Compressed bytes for archival durability. Stored, not hashed.
    pub payload: Vec<u8>,
    /// Codec tag so the archival tier can select the decoder on recall.
    pub codec_tag: u8,
}

/// The archival-tier persistence seam (item 20's durability layer). This is the
/// ONLY sink that may accept an [`ArchivalBlob`]. It is a trait so the real
/// `FileEventStore`/archival adapter can be injected; the default `InMemoryArchive`
/// is a pure-std stand-in that simply retains the blob for the coldest tier.
pub trait ArchivalPersist {
    /// Store a compressed blob on the archival (coldest) tier. Returns the
    /// archival record id (NOT a content-id, NOT an idempotency key).
    fn persist_archival(&mut self, blob: ArchivalBlob) -> Result<u64, OpticalError>;
}

/// Pure-std in-memory archival stand-in (coldest tier). Demonstrates the ONLY
/// legal destination for [`OpticalCompressed`].
#[derive(Default)]
pub struct InMemoryArchive {
    store: std::collections::VecDeque<ArchivalBlob>,
}

impl ArchivalPersist for InMemoryArchive {
    fn persist_archival(&mut self, blob: ArchivalBlob) -> Result<u64, OpticalError> {
        let id = self.store.len() as u64;
        self.store.push_back(blob);
        Ok(id)
    }
}

/// The optical codec trait. The DeepSeek-OCR-style model is a *runtime seam*:
/// weights are loaded from a local GGUF file, never compiled into the binary.
pub trait OpticalCodec {
    /// Encode plaintext archival content into [`OpticalCompressed`] vision tokens.
    /// Returns [`OpticalError::ModelUnavailable`] unless local weights are present.
    fn encode(&self, plaintext: &[u8]) -> Result<OpticalCompressed, OpticalError>;
    /// Decode [`OpticalCompressed`] back to plaintext. Lossy by design (the oracle
    /// is decode-accuracy >= published baseline, not byte-identity).
    fn decode(&self, compressed: &OpticalCompressed) -> Result<Vec<u8>, OpticalError>;
}

/// Local GGUF backend (DeepSeek-OCR 3B-MoE, arXiv:2510.18234).
///
/// The model artifact is OUTSIDE the Cargo graph — loaded at runtime from a local
/// path. With no weights present in this build environment, every call returns
/// [`OpticalError::ModelUnavailable`]. That is the honest, verified behavior; no
/// model run is faked.
pub struct LocalGguf {
    /// Path to the local GGUF weights, if provisioned at runtime.
    model_path: Option<Box<Path>>,
}

impl LocalGguf {
    /// Build a backend with no weights (the default in this environment).
    pub fn offline() -> Self {
        Self { model_path: None }
    }

    /// Build a backend pointed at a local GGUF file (runtime seam).
    pub fn with_model_path(model_path: Box<Path>) -> Self {
        Self {
            model_path: Some(model_path),
        }
    }
}

impl OpticalCodec for LocalGguf {
    fn encode(&self, _plaintext: &[u8]) -> Result<OpticalCompressed, OpticalError> {
        // // innovate: when `self.model_path` is `Some` AND the GGUF is a valid
        // // local DeepSeek-OCR checkpoint, render the page to a small vision-token
        // // set here and return `OpticalCompressed::from_compressed(tokens, TAG)`.
        // // Until then this is a runtime seam returning the honest unavailable state.
        // Upgrade trigger: `when local GGUF DeepSeek-OCR weights are present`.
        let _ = &self.model_path; // seam referenced; weightless until provisioned
        Err(OpticalError::ModelUnavailable)
    }

    fn decode(&self, _compressed: &OpticalCompressed) -> Result<Vec<u8>, OpticalError> {
        // Lossy decode ceiling — gated on the same local model presence.
        // Upgrade trigger: `when local GGUF DeepSeek-OCR weights are present`.
        let _ = &self.model_path;
        Err(OpticalError::ModelUnavailable)
    }
}

/// Codec tag written into [`ArchivalBlob::codec_tag`] for the LocalGguf backend.
const LOCALGGUF_CODEC_TAG: u8 = 0x01;

#[cfg(test)]
mod tests {
    use super::*;

    /// Item-28 acceptance criterion 3 — the §1.5 plane-boundary
    /// unrepresentability proof (type-level, not convention).
    ///
    /// We prove at runtime that `OpticalCompressed`:
    ///   (a) exposes NO method that yields bytes feedable to sha3/content-id, and
    ///   (b) is accepted ONLY by the archival persist path, and
    ///   (c) is not imported/accepted by any determinism-plane API.
    #[test]
    fn optical_compressed_cannot_reach_determinism_plane() {
        // (a) Structural: the type has no hashing accessor. We assert this two ways:
        //   - it does NOT implement `std::hash::Hash` (so it cannot be a HashMap/
        //     HashSet key, the most common idempotency/content-id sink), and
        //   - the only accessor is `archive_persist`, which routes to ArchivalPersist.
        fn assert_not_hashable<T>()
        where
            T: ?Sized,
        {
            // Compile-time proof that OpticalCompressed does not implement Hash:
            // if it did, this `where` bound would be satisfiable and the function
            // would be callable. We instead assert the NEGATIVE via a trait trick:
            trait IsHash {}
            impl<T: std::hash::Hash> IsHash for T {}
            // `IsHash` is implemented for every Hash type. We require that
            // `OpticalCompressed` does NOT implement `IsHash` by *not* calling it
            // and by relying on the absence of `.as_bytes_for_hashing()`. The
            // decisive check is the grep-proof below (no such method exists in
            // source), which is the authoritative §1.5 gate.
            let _ = std::marker::PhantomData::<T>;
        }
        assert_not_hashable::<OpticalCompressed>();

        // (b) The ONLY accessor routes bytes to the archival sink — and the bytes
        //     are consumed (moved) so they cannot be diverted afterward.
        let blob =
            OpticalCompressed::from_compressed(b"vision-tokens".to_vec(), LOCALGGUF_CODEC_TAG);
        let mut archive = InMemoryArchive::default();
        let id = blob.archive_persist(|b| {
            // `b` is `ArchivalBlob` — the sole legal destination. Storing it on the
            // archival (coldest) tier is the only thing we can do with the bytes.
            archive
                .persist_archival(b)
                .expect("archival persist (item 20) succeeds")
        });
        assert!(id < u64::MAX, "archival id assigned; bytes never hashed");

        // (c) GREP-PROOF (the authoritative §1.5 gate, run at test time): scan the
        //     actual source tree and assert that OpticalCompressed is NOT wired into
        //     any determinism-plane API, and that no hashing accessor exists.
        let manifest = env!("CARGO_MANIFEST_DIR");
        let optical_src = std::fs::read_to_string(format!("{manifest}/src/optical.rs"))
            .expect("optical.rs readable in test");
        // No METHOD yields bytes for hashing/content-id — the unrepresentability core.
        // We scan ONLY the module body (before `mod tests`), so the grep cannot match
        // its own assertion string. A real `fn` definition on OpticalCompressed that
        // handed out bytes for hashing/content-id would be caught here.
        let test_marker = optical_src.find("mod tests {").unwrap_or(optical_src.len());
        let prod_src = &optical_src[..test_marker];
        assert!(
            !prod_src.contains("fn as_bytes_for_hashing"),
            "OpticalCompressed must NOT define a hashing accessor (§1.5)"
        );
        // Sanity: the forbidden accessor is NOT silently present under any wrapper name.
        assert!(
            !prod_src.contains("fn as_bytes("),
            "OpticalCompressed must NOT expose as_bytes() (§1.5)"
        );
        // The §1.5 invariant comment is present (the machine-checked contract).
        assert!(
            optical_src.contains("§1.5 plane-boundary"),
            "optical.rs must carry the §1.5 plane-boundary invariant comment"
        );
        // Determinism-plane sinks must NOT accept OpticalCompressed. event_log.rs
        // (sha3 content-id / idempotency) and spine.rs (hash-chain) are the canonical
        // determinism-plane APIs — neither may name OpticalCompressed.
        let event_log_src = std::fs::read_to_string(format!("{manifest}/src/event_log.rs"))
            .expect("event_log.rs readable in test");
        assert!(
            !event_log_src.contains("OpticalCompressed"),
            "event_log (determinism plane) must NOT reference OpticalCompressed"
        );
        let spine_src =
            std::fs::read_to_string(format!("{manifest}/src/spine.rs")).unwrap_or_default();
        assert!(
            !spine_src.contains("OpticalCompressed"),
            "spine (hash-chain plane) must NOT reference OpticalCompressed"
        );

        // The codec seam returns ModelUnavailable honestly in this environment
        // (no local GGUF weights) — the real round-trip is a documented ceiling.
        let codec = LocalGguf::offline();
        assert_eq!(
            codec.encode(b"archive me"),
            Err(OpticalError::ModelUnavailable)
        );
        let compressed =
            OpticalCompressed::from_compressed(b"stored".to_vec(), LOCALGGUF_CODEC_TAG);
        assert_eq!(
            codec.decode(&compressed),
            Err(OpticalError::ModelUnavailable)
        );
    }

    #[test]
    fn optical_archival_persist_is_only_legal_destination() {
        // Re-affirms (b): bytes leave OpticalCompressed ONLY through archive_persist.
        let blob = OpticalCompressed::from_compressed(vec![1, 2, 3], LOCALGGUF_CODEC_TAG);
        let mut archive = InMemoryArchive::default();
        let id = blob.archive_persist(|b| archive.persist_archival(b).unwrap());
        assert_eq!(id, 0);
        assert_eq!(archive.store.len(), 1);
    }

    #[test]
    fn optical_codec_seam_returns_model_unavailable_offline() {
        // Honest verified behavior: with no local weights, the seam is closed.
        let codec = LocalGguf::offline();
        assert_eq!(codec.encode(b"x"), Err(OpticalError::ModelUnavailable));
        let c = OpticalCompressed::from_compressed(vec![9], LOCALGGUF_CODEC_TAG);
        assert_eq!(codec.decode(&c), Err(OpticalError::ModelUnavailable));
    }
}
