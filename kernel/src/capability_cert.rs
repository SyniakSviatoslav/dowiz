//! `capability_cert` — BLUEPRINT-P59 capability-cert chain & crypto-agility.
//!
//! A biscuit-style, **hybrid-signed** (Ed25519 ⊕ ML-DSA-65 via the
//! `SignatureVerifier` seam), **algorithm-agile** capability-cert chain built over
//! the kernel's existing identity primitives. This module is the P59 deliverable:
//! self-signed hub/owner roots by default (§17.7), owner → hub single-hop
//! delegation with `may_delegate = false` (§2.4 / §16.48), suite-negotiation with
//! downgrade binding (TLS model), SSH-style overlap rotation for fleet migration,
//! and owner-signed revocation blobs gossiped via the existing `RevocationSet::merge`
//! anti-entropy union (§5).
//!
//! # Placement note (deviation from the blueprint's illustrative path)
//!
//! The blueprint text suggests `kernel/src/pq/cert_chain.rs`. That path is gated
//! behind the `pq` Cargo feature, but P59's acceptance gate is `cargo test -p kernel`
//! with **default features** (no `pq`). The "existing HybridSigner" the charter
//! references is *not* a struct named `HybridSigner` — it is the `SignatureVerifier`
//! seam + in-tree `RefSigner` reference in `ports/agent/cap.rs` (default-built),
//! which already provides a real `sign_classical`/`sign_pq`/`verify_classical`/
//! `verify_pq` AND-verify floor (`HybridPolicy::RequireBoth`). This module therefore
//! lives at `kernel/src/capability_cert.rs` (default-built) and rides that seam, so
//! the full chain verifies under a real RequireBoth floor even in the default build.
//! Production injects real bebop2 Ed25519 + ML-DSA-65 at the seam without touching
//! this file. All behaviors mandated by the 20-point contract (D1–D13) are met here.
//!
//! # Crypto-agility (adoption, not invention)
//!
//! Each block carries `AlgSuite` (an internal `u16` registry that *maps* to the
//! composite-sigs OID `1.3.6.1.5.5.7.6.48`, `draft-ietf-lamps-pq-composite-sigs`
//! v19). The suite tag is bound into the signed bytes (`DOMAIN_SUITE_PREFIX`) so a
//! signature made under one suite can never replay as another. Unknown code points
//! are rejected fail-closed. Adding a suite = one code-point registration, never a
//! wire-format fork.

use std::collections::HashMap;

use crate::event_log::sha3_256;
use crate::ports::agent::cap::{
    revocation_hash, AnchorRoster, Capability, NodeId, RefSigner, RevocationSet, SignatureVerifier,
};
use crate::ports::agent::scope::{Action, Resource, Scope};

// ── algorithm-suite registry (composite-sigs adoption) ──────────────────────────

/// Algorithm-suite identifier carried in every signed block. Adoption of the
/// composite-sigs registry (`draft-ietf-lamps-pq-composite-sigs` v19), NOT a bespoke
/// scheme. The `u16` is an INTERNAL enum that *maps* to the OID — the mapping is the
/// only place the pre-RFC OID lives, so an OID shift is a one-line remap (R3 risk #6),
/// never a cert-format migration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum AlgSuite {
    /// v1 — the CURRENT dowiz hybrid. Maps to `id-MLDSA65-Ed25519-SHA512`.
    MlDsa65Ed25519 = 0x0001,
    /// Example/reserved second code point — registered ONLY to exercise suite
    /// negotiation + downgrade-binding (M6). It shares the in-tree `SignatureVerifier`
    /// seam (a production v2 would bind a distinct primitive + OID). Adding a suite is
    /// a ONE-LINE code-point registration (never a wire-format fork).
    MlDsa65SlhDsa = 0x0002,
}

/// The standardized OID string for suite v1 (composite-sigs registry). THE single
/// remap point — `AlgSuite::oid` references it in exactly one `match` arm.
pub const OID_MLDSA65_ED25519_SHA512: &str = "1.3.6.1.5.5.7.6.48";
/// The composite-sigs signature label (transcript/domain use) for suite v1.
pub const LABEL_MLDSA65_ED25519_SHA512: &[u8] = b"COMPSIG-MLDSA65-Ed25519-SHA512";

/// Placeholder OID for the reserved example suite v2 (negotiation exercise only).
pub const OID_MLDSA65_SLHDSA_SHA512: &str = "1.3.6.1.5.5.7.6.48.example";
/// Placeholder label for the reserved example suite v2.
pub const LABEL_MLDSA65_SLHDSA_SHA512: &[u8] = b"COMPSIG-MLDSA65-SLHDSA-SHA512";

impl AlgSuite {
    /// The standardized OID for this suite. ONE mapping table — the single remap point.
    pub fn oid(self) -> &'static str {
        match self {
            Self::MlDsa65Ed25519 => OID_MLDSA65_ED25519_SHA512,
            Self::MlDsa65SlhDsa => OID_MLDSA65_SLHDSA_SHA512,
        }
    }
    /// Fail-closed decode: an unknown `u16` is `None`, mirroring `scope.rs`'s strict
    /// decode. An unregistered suite on the wire is therefore unrepresentable-valid.
    pub fn from_u16(v: u16) -> Option<Self> {
        match v {
            0x0001 => Some(Self::MlDsa65Ed25519),
            0x0002 => Some(Self::MlDsa65SlhDsa),
            _ => None,
        }
    }
    /// The internal wire code point.
    pub fn to_u16(self) -> u16 {
        self as u16
    }
}

/// Per-block domain tag that binds the SUITE into the signed bytes, so a signature
/// made under one suite can never be replayed as another (cross-suite confusion —
/// R3 risk #3). Prepended to the canonical block bytes before signing/verifying.
pub const DOMAIN_SUITE_PREFIX: &[u8; 16] = b"dowiz.pq.suite\x01\x01";

/// Bind the suite tag into `msg`: `DOMAIN_SUITE_PREFIX || (alg_suite as u16 le) || msg`.
fn suite_bound(alg_suite: AlgSuite, msg: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(DOMAIN_SUITE_PREFIX.len() + 2 + msg.len());
    out.extend_from_slice(DOMAIN_SUITE_PREFIX);
    out.extend_from_slice(&alg_suite.to_u16().to_le_bytes());
    out.extend_from_slice(msg);
    out
}

// ── hybrid signature (RequireBoth, no OR code point) ────────────────────────────

/// A hybrid signature pair over one block. RequireBoth: verification is AND
/// (composite-sigs + B4/SSR-2020 lesson). A missing OR non-verifying half is total
/// failure — never a soft pass.
///
/// The suite is stored as a raw `u16` (not the enum) so an unknown code point that
/// arrives on the wire is representable and rejected by [`HybridSig::suite`] (the
/// fail-closed decode). This is what makes `red_unknown_suite_rejected` and
/// `red_suite_swap_breaks_sig` meaningful.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridSig {
    /// Raw algorithm-suite code point (see note above).
    pub alg_suite_raw: u16,
    /// Classical (Ed25519) signature leg.
    pub classical: Vec<u8>,
    /// Post-quantum (ML-DSA-65) signature leg.
    pub pq: Vec<u8>,
}

impl HybridSig {
    /// The registered suite, or `None` if the raw code point is unknown (fail-closed).
    pub fn suite(&self) -> Option<AlgSuite> {
        AlgSuite::from_u16(self.alg_suite_raw)
    }

    /// Produce a RequireBoth hybrid signature over `msg` under both secrets.
    pub fn sign<V: SignatureVerifier>(
        verifier: &V,
        alg_suite: AlgSuite,
        classical_secret: &[u8; 32],
        pq_secret: &[u8; 32],
        msg: &[u8],
    ) -> Self {
        let bound = suite_bound(alg_suite, msg);
        HybridSig {
            alg_suite_raw: alg_suite.to_u16(),
            classical: verifier.sign_classical(classical_secret, &bound),
            pq: verifier.sign_pq(pq_secret, &bound),
        }
    }

    /// RequireBoth verify. Unknown suite ⇒ `false` (fail-closed). A missing OR
    /// non-verifying leg ⇒ `false`. No OR shortcut exists.
    pub fn verify<V: SignatureVerifier>(
        &self,
        verifier: &V,
        classical_pub: &[u8; 32],
        pq_pub: &[u8],
        msg: &[u8],
    ) -> bool {
        let suite = match self.suite() {
            Some(s) => s,
            // Unknown suite on the wire ⇒ unconditionally rejected.
            None => return false,
        };
        let bound = suite_bound(suite, msg);
        let ok_cls = verifier.verify_classical(classical_pub, &bound, &self.classical);
        let ok_pq = verifier.verify_pq(pq_pub, &bound, &self.pq);
        ok_cls && ok_pq
    }
}

// ── typed error surface (fail-closed, enumerated) ───────────────────────────────

/// Why a capability-cert operation failed. Every variant is a typed, fail-closed
/// rejection — there is no "partial trust" / degraded code point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CertError {
    /// An unregistered algorithm suite was presented.
    UnknownSuite,
    /// A hybrid signature did not fully verify (a leg missing or forged).
    BadSignature,
    /// A root, link, or capability is expired (TTL is load-bearing).
    Expired,
    /// A self-signed root's `node_id` does not bind both of its public keys.
    NodeIdMismatch,
    /// A revoked key or capability hash was encountered.
    Revoked,
    /// Attenuation violated (escalation), tail mis-binding, or effect ⊄ tail scope.
    ScopeViolation,
    /// The issuer of a link is not the expected/anchored issuer (or root not enrolled).
    UnknownIssuer,
    /// A delegation depth ceiling was exceeded (re-delegation / over-long chain).
    MaxDepthExceeded,
    /// Suite negotiation found no common suite between the peers.
    NoCommonSuite,
}

// ── self-signed hybrid root (§17.7) ─────────────────────────────────────────────

/// A self-signed hybrid root. Block 0 of a chain: signed by its OWN keypair. This is
/// the trust-domain root — NO dowiz needed. Mirrors `codesign::PinnedRoot` but hybrid.
///
/// `node_id` MUST equal `NodeId::from_keys(pq_pub, classical_pub)` — the §17.7
/// "self-signed root, no CA" invariant. `root_scope` is the authority the root holds;
/// every delegated link's scope must attenuate from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelfSignedRoot {
    /// Classical (Ed25519) public key.
    pub classical_pub: [u8; 32],
    /// Post-quantum (ML-DSA-65) public key.
    pub pq_pub: Vec<u8>,
    /// Stable identity = hash of both public keys.
    pub node_id: NodeId,
    /// The authority this root holds (attenuation floor for all children).
    pub root_scope: Scope,
    /// The algorithm suite this root is minted under.
    pub alg_suite: AlgSuite,
    /// Self-signature over the canonical root bytes (RequireBoth).
    pub self_sig: HybridSig,
    /// Root TTL (monotonic tick); short by policy (§5).
    pub not_after: u64,
}

impl SelfSignedRoot {
    /// Mint a self-signed hybrid root from a classical seed + a PQ seed.
    pub fn mint<V: SignatureVerifier>(
        verifier: &V,
        classical_seed: &[u8; 32],
        pq_seed: &[u8; 32],
        root_scope: Scope,
        not_after: u64,
    ) -> Self {
        let classical_pub = verifier.classical_public(classical_seed);
        let pq_pub = verifier.pq_public(pq_seed);
        let node_id = NodeId::from_keys(&pq_pub, &classical_pub);
        let alg_suite = AlgSuite::MlDsa65Ed25519;
        let msg = Self::canonical_bytes(&classical_pub, &pq_pub, &root_scope, alg_suite, not_after);
        let self_sig = HybridSig::sign(verifier, alg_suite, classical_seed, pq_seed, &msg);
        SelfSignedRoot {
            classical_pub,
            pq_pub,
            node_id,
            root_scope,
            alg_suite,
            self_sig,
            not_after,
        }
    }

    /// Canonical, domain-separated signing bytes for the root (binds every field,
    /// including `root_scope` and the suite; the suite tag is bound again by
    /// `HybridSig::sign`).
    fn canonical_bytes(
        classical_pub: &[u8; 32],
        pq_pub: &[u8],
        root_scope: &Scope,
        alg_suite: AlgSuite,
        not_after: u64,
    ) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"dowiz.root\x01");
        out.extend_from_slice(classical_pub);
        out.extend_from_slice(&(pq_pub.len() as u32).to_le_bytes());
        out.extend_from_slice(pq_pub);
        let rs = root_scope.to_tlv_bytes();
        out.extend_from_slice(&(rs.len() as u32).to_le_bytes());
        out.extend_from_slice(&rs);
        out.extend_from_slice(&alg_suite.to_u16().to_le_bytes());
        out.extend_from_slice(&not_after.to_le_bytes());
        out
    }

    /// Verify the self-signature under RequireBoth, the TTL, and the node-id binding.
    /// A `DowizCoSign` is NEVER consulted here — it is strictly additive (M4).
    pub fn verify_self<V: SignatureVerifier>(
        &self,
        verifier: &V,
        now: u64,
    ) -> Result<(), CertError> {
        // TTL is load-bearing (§5).
        if self.not_after <= now {
            return Err(CertError::Expired);
        }
        // node_id must bind BOTH public keys.
        let expected_id = NodeId::from_keys(&self.pq_pub, &self.classical_pub);
        if expected_id != self.node_id {
            return Err(CertError::NodeIdMismatch);
        }
        // suite must be registered.
        if self.suite().is_none() {
            return Err(CertError::UnknownSuite);
        }
        let msg = Self::canonical_bytes(
            &self.classical_pub,
            &self.pq_pub,
            &self.root_scope,
            self.alg_suite,
            self.not_after,
        );
        if !self
            .self_sig
            .verify(verifier, &self.classical_pub, &self.pq_pub, &msg)
        {
            return Err(CertError::BadSignature);
        }
        Ok(())
    }

    /// The registered suite, or `None` if the raw code point is unknown.
    pub fn suite(&self) -> Option<AlgSuite> {
        AlgSuite::from_u16(self.alg_suite.to_u16())
    }

    /// Enroll this root's classical public key as an anchor in `roster`.
    pub fn enroll(&self, roster: &mut AnchorRoster) {
        roster.enroll(&self.classical_pub);
    }
}

/// Optional detached dowiz co-signature over a root's public keys (§17.7). Its ABSENCE
/// never invalidates the root — it is a second voucher for relying parties that trust
/// dowiz, nothing more. Convenience only (claim-flow), NEVER load-bearing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DowizCoSign {
    /// The root node id this co-sign vouches for.
    pub over_node_id: NodeId,
    /// Dowiz-key hybrid sig over the root's node id + suite.
    pub sig: HybridSig,
}

impl DowizCoSign {
    /// Canonical bytes the dowiz key signs over.
    fn canonical(root: &SelfSignedRoot) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"dowiz.cosign\x01");
        out.extend_from_slice(root.node_id.as_bytes());
        out.extend_from_slice(&root.alg_suite.to_u16().to_le_bytes());
        out
    }

    /// Mint a dowiz co-signature over `root` using dowiz's classical + PQ seeds.
    pub fn sign<V: SignatureVerifier>(
        verifier: &V,
        dowiz_classical_seed: &[u8; 32],
        dowiz_pq_seed: &[u8; 32],
        root: &SelfSignedRoot,
    ) -> Self {
        let msg = Self::canonical(root);
        let sig = HybridSig::sign(
            verifier,
            AlgSuite::MlDsa65Ed25519,
            dowiz_classical_seed,
            dowiz_pq_seed,
            &msg,
        );
        DowizCoSign {
            over_node_id: root.node_id,
            sig,
        }
    }

    /// Verify the co-signature against dowiz's public keys. Returns `false` (never
    /// panics, never trusts) if the sig is invalid or does not match `root`.
    pub fn verify<V: SignatureVerifier>(
        &self,
        verifier: &V,
        dowiz_classical_pub: &[u8; 32],
        dowiz_pq_pub: &[u8],
        root: &SelfSignedRoot,
    ) -> bool {
        if self.over_node_id != root.node_id {
            return false;
        }
        let msg = Self::canonical(root);
        self.sig
            .verify(verifier, dowiz_classical_pub, dowiz_pq_pub, &msg)
    }
}

// ── delegation link (one chain hop) ─────────────────────────────────────────────

/// A single hybrid-signed delegation link in the capability-cert chain. Carries BOTH
/// classical + PQ public keys for its endpoints so each leg's signature can be verified
/// under RequireBoth (closing the Ed25519-only gap that existed in `cap.rs::Delegation`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CertDelegation {
    /// Issuer classical public key.
    pub issued_by: [u8; 32],
    /// Issuer PQ public key (needed to verify the PQ leg).
    pub issued_by_pq: Vec<u8>,
    /// Subject (child) classical public key.
    pub subject: [u8; 32],
    /// Subject (child) PQ public key.
    pub subject_pq: Vec<u8>,
    /// Scope the granter is willing to pass down (subset of parent's scope).
    pub scope: Scope,
    /// Effect actually authorized at this link (subset of `scope`).
    pub effect: Scope,
    /// Whether the subject may further delegate. Owner→hub children carry `false`.
    pub may_delegate: bool,
    /// Expiry (monotonic tick).
    pub expiry: u64,
    /// Single-use nonce.
    pub nonce: [u8; 8],
    /// Algorithm suite this link is signed under.
    pub alg_suite: AlgSuite,
    /// RequireBoth hybrid signature over the canonical link bytes.
    pub sig: HybridSig,
}

impl CertDelegation {
    /// Canonical bytes for one link (suite tag bound by `HybridSig::sign`).
    fn canonical_bytes(
        issued_by: &[u8; 32],
        issued_by_pq: &[u8],
        subject: &[u8; 32],
        subject_pq: &[u8],
        scope: &Scope,
        effect: &Scope,
        may_delegate: bool,
        expiry: u64,
        nonce: &[u8; 8],
    ) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"dowiz.dlg\x01");
        out.extend_from_slice(issued_by);
        out.extend_from_slice(&(issued_by_pq.len() as u32).to_le_bytes());
        out.extend_from_slice(issued_by_pq);
        out.extend_from_slice(subject);
        out.extend_from_slice(&(subject_pq.len() as u32).to_le_bytes());
        out.extend_from_slice(subject_pq);
        let s = scope.to_tlv_bytes();
        out.extend_from_slice(&(s.len() as u32).to_le_bytes());
        out.extend_from_slice(&s);
        let e = effect.to_tlv_bytes();
        out.extend_from_slice(&(e.len() as u32).to_le_bytes());
        out.extend_from_slice(&e);
        out.push(if may_delegate { 1u8 } else { 0u8 });
        out.extend_from_slice(&expiry.to_le_bytes());
        out.extend_from_slice(nonce);
        out
    }

    /// Sign a delegation link. `issued_by*` are derived from the issuer seeds; `subject*`
    /// are the child's public keys (provided by the child to be certified).
    #[allow(clippy::too_many_arguments)]
    pub fn sign<V: SignatureVerifier>(
        verifier: &V,
        issuer_classical_seed: &[u8; 32],
        issuer_pq_seed: &[u8; 32],
        issued_by: [u8; 32],
        issued_by_pq: Vec<u8>,
        subject_classical_pub: [u8; 32],
        subject_pq_pub: Vec<u8>,
        scope: Scope,
        effect: Scope,
        may_delegate: bool,
        alg_suite: AlgSuite,
        expiry: u64,
        nonce: [u8; 8],
    ) -> Self {
        let inner = Self::canonical_bytes(
            &issued_by,
            &issued_by_pq,
            &subject_classical_pub,
            &subject_pq_pub,
            &scope,
            &effect,
            may_delegate,
            expiry,
            &nonce,
        );
        let sig = HybridSig::sign(
            verifier,
            alg_suite,
            issuer_classical_seed,
            issuer_pq_seed,
            &inner,
        );
        CertDelegation {
            issued_by,
            issued_by_pq,
            subject: subject_classical_pub,
            subject_pq: subject_pq_pub,
            scope,
            effect,
            may_delegate,
            expiry,
            nonce,
            alg_suite,
            sig,
        }
    }
    /// The registered suite, or `None` if unknown.
    pub fn suite(&self) -> Option<AlgSuite> {
        AlgSuite::from_u16(self.alg_suite.to_u16())
    }

    /// Verify this link's RequireBoth signature against its own `issued_by` keys.
    pub fn verify_signature<V: SignatureVerifier>(&self, verifier: &V) -> bool {
        let inner = Self::canonical_bytes(
            &self.issued_by,
            &self.issued_by_pq,
            &self.subject,
            &self.subject_pq,
            &self.scope,
            &self.effect,
            self.may_delegate,
            self.expiry,
            &self.nonce,
        );
        self.sig
            .verify(verifier, &self.issued_by, &self.issued_by_pq, &inner)
    }
}

// ── suite negotiation + downgrade binding (TLS model, §6.4) ──────────────────────

/// The negotiated-suite handshake message. Advertised suites, strongest first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SuiteAdvertisement {
    /// Offered suites in preference order (strongest first).
    pub offered: Vec<AlgSuite>,
}

/// The result of a successful negotiation: the strongest common suite plus a
/// transcript hash binding BOTH parties' offered lists (downgrade protection).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NegotiatedSuite {
    /// The chosen (strongest common) suite.
    pub chosen: AlgSuite,
    /// SHA3-256 over BOTH parties' full offered lists — bound into the session's first
    /// signed frame so a MITM cannot strip strong suites without breaking the signature.
    pub transcript_hash: [u8; 32],
}

/// Negotiate the strongest common suite between two advertisements. Returns `None`
/// (fail-closed) if the lists are disjoint — never a silent fallback to a default.
pub fn negotiate(local: &SuiteAdvertisement, peer: &SuiteAdvertisement) -> Option<NegotiatedSuite> {
    let chosen = local.offered.iter().find(|s| peer.offered.contains(s))?;
    // Bind BOTH offered lists into the transcript hash.
    let mut buf = Vec::new();
    for s in &local.offered {
        buf.extend_from_slice(&s.to_u16().to_le_bytes());
    }
    for s in &peer.offered {
        buf.extend_from_slice(&s.to_u16().to_le_bytes());
    }
    let transcript_hash = sha3_256(&buf);
    Some(NegotiatedSuite {
        chosen: *chosen,
        transcript_hash,
    })
}

/// SSH-style overlap rotation state for fleet-wide suite migration (§6.3). A hub in
/// `Overlapping` publishes BOTH credentials for a hub-local window; verifiers learn
/// the new; then `retire` (transition to `Stable { new }`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RotationState {
    /// Steady state: only `suite` is accepted.
    Stable { suite: AlgSuite },
    /// Migration window: both `old` and `new` accepted until `overlap_until`.
    Overlapping {
        old: AlgSuite,
        new: AlgSuite,
        overlap_until: u64,
    },
}

impl RotationState {
    /// Whether a credential in `suite` is accepted at time `now` under this state.
    /// After the overlap window, only the new suite is accepted (old retired).
    pub fn accepts(&self, suite: AlgSuite, now: u64) -> bool {
        match self {
            RotationState::Stable { suite: s } => *s == suite,
            RotationState::Overlapping {
                old,
                new,
                overlap_until,
            } => {
                if now <= *overlap_until {
                    *old == suite || *new == suite
                } else {
                    *new == suite
                }
            }
        }
    }
}

// ── revocation blob + store (§5, gossip via RevocationSet::merge) ────────────────

/// Signed, gossip-able revocation blob. Reuses `RevocationSet` as the merge substrate;
/// this is the SIGNED envelope an owner root publishes to its own fleet. No dowiz relay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RevocationBlob {
    /// The owner/hub root that authored it.
    pub issuer_root: NodeId,
    /// Revoked classical/PQ subject keys.
    pub revoked_keys: Vec<[u8; 32]>,
    /// Revoked capability hashes.
    pub revoked_cap_hashes: Vec<[u8; 32]>,
    /// Monotonic sequence; a later seq from the same issuer supersedes (LWW).
    pub seq: u64,
    /// Blob authority expiry (bounds the blob's reach in time).
    pub not_after: u64,
    /// Algorithm suite.
    pub alg_suite: AlgSuite,
    /// RequireBoth hybrid sig over the canonical blob bytes by the issuer root.
    pub sig: HybridSig,
}

impl RevocationBlob {
    fn canonical_bytes(
        issuer_root: &NodeId,
        revoked_keys: &[[u8; 32]],
        revoked_cap_hashes: &[[u8; 32]],
        seq: u64,
        not_after: u64,
        alg_suite: AlgSuite,
    ) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(b"dowiz.revb\x01");
        out.extend_from_slice(issuer_root.as_bytes());
        out.extend_from_slice(&(revoked_keys.len() as u32).to_le_bytes());
        for k in revoked_keys {
            out.extend_from_slice(k);
        }
        out.extend_from_slice(&(revoked_cap_hashes.len() as u32).to_le_bytes());
        for c in revoked_cap_hashes {
            out.extend_from_slice(c);
        }
        out.extend_from_slice(&seq.to_le_bytes());
        out.extend_from_slice(&not_after.to_le_bytes());
        out.extend_from_slice(&alg_suite.to_u16().to_le_bytes());
        out
    }

    /// Sign a revocation blob with the issuer root's seeds.
    pub fn sign<V: SignatureVerifier>(
        verifier: &V,
        issuer_root: &SelfSignedRoot,
        issuer_classical_seed: &[u8; 32],
        issuer_pq_seed: &[u8; 32],
        revoked_keys: Vec<[u8; 32]>,
        revoked_cap_hashes: Vec<[u8; 32]>,
        seq: u64,
        not_after: u64,
    ) -> Self {
        let alg_suite = AlgSuite::MlDsa65Ed25519;
        let msg = Self::canonical_bytes(
            &issuer_root.node_id,
            &revoked_keys,
            &revoked_cap_hashes,
            seq,
            not_after,
            alg_suite,
        );
        let sig = HybridSig::sign(
            verifier,
            alg_suite,
            issuer_classical_seed,
            issuer_pq_seed,
            &msg,
        );
        RevocationBlob {
            issuer_root: issuer_root.node_id,
            revoked_keys,
            revoked_cap_hashes,
            seq,
            not_after,
            alg_suite,
            sig,
        }
    }

    /// Verify the blob's hybrid signature + TTL. Does NOT mutate any store.
    pub fn verify<V: SignatureVerifier>(
        &self,
        verifier: &V,
        issuer_root: &SelfSignedRoot,
        now: u64,
    ) -> Result<(), CertError> {
        if self.issuer_root != issuer_root.node_id {
            return Err(CertError::UnknownIssuer);
        }
        if self.not_after <= now {
            return Err(CertError::Expired);
        }
        if AlgSuite::from_u16(self.alg_suite.to_u16()).is_none() {
            return Err(CertError::UnknownSuite);
        }
        let msg = Self::canonical_bytes(
            &self.issuer_root,
            &self.revoked_keys,
            &self.revoked_cap_hashes,
            self.seq,
            self.not_after,
            self.alg_suite,
        );
        if !self.sig.verify(
            verifier,
            &issuer_root.classical_pub,
            &issuer_root.pq_pub,
            &msg,
        ) {
            return Err(CertError::BadSignature);
        }
        Ok(())
    }
}

/// Outcome of attempting to apply a revocation blob.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplyOutcome {
    /// Blob verified and its revocations were merged into the store.
    Applied,
    /// Blob verified but its `seq` was <= the last applied seq for this issuer, so it
    /// was ignored (monotone — a lower seq never un-revokes).
    IgnoredStaleSeq,
}

/// Owner-fleet revocation store: an append-only `RevocationSet` (gossip substrate) plus
/// a per-issuer last-`seq` tracker (LWW). Revocation is monotone — a lower `seq` can
/// never remove an existing revocation (§5 / R3 risk #3).
#[derive(Debug, Clone, Default)]
pub struct RevocationStore {
    set: RevocationSet,
    seqs: HashMap<NodeId, u64>,
}

impl RevocationStore {
    /// Empty store.
    pub fn new() -> Self {
        RevocationStore::default()
    }

    /// Verify `blob` (RequireBoth + TTL + issuer match) and, if its `seq` is newer than
    /// the last applied for its issuer, merge its revocations into the set. A stale `seq`
    /// is ignored (the blob still verified, but applies nothing). An unsigned/expired
    /// blob is rejected and the set is UNCHANGED.
    pub fn apply_blob<V: SignatureVerifier>(
        &mut self,
        blob: &RevocationBlob,
        verifier: &V,
        issuer_root: &SelfSignedRoot,
        now: u64,
    ) -> Result<ApplyOutcome, CertError> {
        blob.verify(verifier, issuer_root, now)?;
        let last = self.seqs.get(&blob.issuer_root).copied().unwrap_or(0);
        if blob.seq <= last {
            return Ok(ApplyOutcome::IgnoredStaleSeq);
        }
        let mut tmp = RevocationSet::new();
        for k in &blob.revoked_keys {
            tmp.revoke_key(*k);
        }
        for c in &blob.revoked_cap_hashes {
            tmp.revoke_capability(*c);
        }
        self.set.merge(&tmp);
        self.seqs.insert(blob.issuer_root, blob.seq);
        Ok(ApplyOutcome::Applied)
    }

    /// Whether `key` is revoked.
    pub fn is_revoked_key(&self, key: &[u8; 32]) -> bool {
        self.set.is_revoked_key(key)
    }

    /// Whether the capability with this revocation hash is revoked.
    pub fn is_revoked_capability(&self, cap_hash: &[u8; 32]) -> bool {
        self.set.is_revoked_capability(cap_hash)
    }
}

// ── chain verification (M2/M3) ───────────────────────────────────────────────────

// Named defaults (engineering-decision E, §5.3). Tick = 1 second for these constants.
/// Owner/hub root validity window (90 days).
pub const ROOT_TTL_TICKS: u64 = 90 * 24 * 3600;
/// Per-hub child cert validity window (24h; re-mint under an attended owner root).
pub const CHILD_CERT_TTL_TICKS: u64 = 24 * 3600;
/// Suite-rotation overlap window (≥ 2× the gossip cadence so no verifier strands).
pub const OVERLAP_WINDOW_TICKS: u64 = 2 * 3600;
/// Owner→fleet revocation-blob push cadence.
pub const REVOCATION_BLOB_GOSSIP_TICKS: u64 = 900;
/// Hard cap on chain length: root + ≤3 hops. Truncation/DoS guard.
pub const MAX_CHAIN_LEN: usize = 4;
/// Owner→hub single hop; children carry `may_delegate = false` (§2.4).
pub const MAX_DELEGATION_DEPTH: u8 = 1;

/// Anchor-rooted, hybrid, suite-gated, revocation-checked chain verification.
///
/// Reuses the existing UCAN-subset attenuation/expiry/tail-binding logic from
/// `cap.rs::verify_chain`, extended with: (a) every link + the root carries a
/// registered, consistent `AlgSuite`; (b) both hybrid legs verify under RequireBoth;
/// (c) revoked keys / cap-hashes are rejected; (d) the root is a self-signed hybrid
/// root enrolled as an anchor; (e) `may_delegate = false` forbids any following link
/// (owner→hub depth ceiling, §2.4); (f) hard `MAX_CHAIN_LEN` cap.
pub fn verify_chain_hybrid<V: SignatureVerifier>(
    verifier: &V,
    roster: &AnchorRoster,
    rev_store: &RevocationStore,
    root: &SelfSignedRoot,
    chain: &[CertDelegation],
    cap: &Capability,
    now: u64,
) -> Result<(), CertError> {
    // 1. Root must be valid (self-signed hybrid, TTL, node-id bind, registered suite).
    root.verify_self(verifier, now)?;
    // 2. Root must be an enrolled anchor (owner/hub root).
    if !roster.contains(&root.classical_pub) {
        return Err(CertError::UnknownIssuer);
    }
    // 3. Hard chain-length cap (truncation / DoS guard).
    if chain.len() > MAX_CHAIN_LEN {
        return Err(CertError::MaxDepthExceeded);
    }
    let root_suite = root.suite().ok_or(CertError::UnknownSuite)?;

    let mut prev: Option<&CertDelegation> = None;
    for (i, link) in chain.iter().enumerate() {
        // (a) suite registered + consistent across the chain.
        let suite = link.suite().ok_or(CertError::UnknownSuite)?;
        if suite != root_suite {
            return Err(CertError::UnknownSuite);
        }
        // (a2) the link's RequireBoth signature must actually verify under its issuer's
        // keys — otherwise a forged / zeroed-PQ link would be accepted (the gap M2 closes).
        if !link.verify_signature(verifier) {
            return Err(CertError::BadSignature);
        }
        // (b) issuer is the root (first) or the previous link's subject — both keys.
        let (exp_cls, exp_pq) = if let Some(p) = prev {
            (p.subject, p.subject_pq.clone())
        } else {
            (root.classical_pub, root.pq_pub.clone())
        };
        if link.issued_by != exp_cls || link.issued_by_pq != exp_pq {
            return Err(CertError::UnknownIssuer);
        }
        // (c) revocation: any revoked key in the link.
        if rev_store.is_revoked_key(&link.issued_by) || rev_store.is_revoked_key(&link.subject) {
            return Err(CertError::Revoked);
        }
        // (d) expiry (freshness).
        if link.expiry <= now {
            return Err(CertError::Expired);
        }
        // (e) effect ⊆ scope (narrow-only within the link).
        if !link.effect.is_subset_of(&link.scope) {
            return Err(CertError::ScopeViolation);
        }
        // (f) depth ceiling: a non-delegable link forbids any following link.
        if i > 0 && !prev.unwrap().may_delegate {
            return Err(CertError::MaxDepthExceeded);
        }
        // (g) attenuation across links (or, for the first link, vs the root scope).
        if let Some(p) = prev {
            if !link.scope.is_subset_of(&p.scope) || !link.effect.is_subset_of(&p.effect) {
                return Err(CertError::ScopeViolation);
            }
        } else if !link.scope.is_subset_of(&root.root_scope) {
            return Err(CertError::ScopeViolation);
        }
        prev = Some(link);
    }

    // 4. Tail must bind to the capability's subject (classical + PQ).
    let tail = chain.last().ok_or(CertError::UnknownIssuer)?;
    if tail.subject != cap.subject_key {
        return Err(CertError::ScopeViolation);
    }
    if let Some(pq) = &cap.subject_key_pq {
        if &tail.subject_pq != pq {
            return Err(CertError::ScopeViolation);
        }
    }
    // 5. Requested effect ⊆ tail's authorized effect.
    if !cap.scope.is_subset_of(&tail.effect) {
        return Err(CertError::ScopeViolation);
    }
    // 6. Capability freshness + revocation-by-hash.
    if !cap.is_fresh(now) {
        return Err(CertError::Expired);
    }
    if rev_store.is_revoked_capability(&revocation_hash(cap)) {
        return Err(CertError::Revoked);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // A test party with deterministic seeds + derived public keys.
    struct Party {
        cls_seed: [u8; 32],
        pq_seed: [u8; 32],
        cls_pub: [u8; 32],
        pq_pub: Vec<u8>,
    }
    impl Party {
        fn new(v: &RefSigner, i: u8) -> Self {
            let cls_seed = [i; 32];
            let pq_seed = [i.wrapping_add(100); 32];
            let cls_pub = v.classical_public(&cls_seed);
            let pq_pub = v.pq_public(&pq_seed);
            Party {
                cls_seed,
                pq_seed,
                cls_pub,
                pq_pub,
            }
        }
    }

    fn scope() -> Scope {
        Scope::single(Resource::AgentBridge, Action::AdmitAgent)
    }

    /// Build a linear chain root -> p0 -> p1 -> ... from `parties` (parties[0] is the
    /// root issuer). All links share `scope`/effect and `may_delegate`.
    fn linear_chain(
        v: &RefSigner,
        parties: &[Party],
        root: &SelfSignedRoot,
        may_delegate: bool,
        expiry: u64,
    ) -> Vec<CertDelegation> {
        let s = scope();
        let mut links = Vec::new();
        for i in 0..parties.len() - 1 {
            let (issuer, subject) = (&parties[i], &parties[i + 1]);
            let issued_by = if i == 0 {
                root.classical_pub
            } else {
                issuer.cls_pub
            };
            let issued_by_pq = if i == 0 {
                root.pq_pub.clone()
            } else {
                issuer.pq_pub.clone()
            };
            let link = CertDelegation::sign(
                v,
                &issuer.cls_seed,
                &issuer.pq_seed,
                issued_by,
                issued_by_pq,
                subject.cls_pub,
                subject.pq_pub.clone(),
                s.clone(),
                s.clone(),
                may_delegate,
                AlgSuite::MlDsa65Ed25519,
                expiry,
                [i as u8; 8],
            );
            links.push(link);
        }
        links
    }

    // ── M1: AlgSuite field + suite→OID mapping ──────────────────────────────────

    #[test]
    fn red_unknown_suite_rejected() {
        // An unknown raw code point decodes to None (fail-closed).
        assert_eq!(AlgSuite::from_u16(0x0002), Some(AlgSuite::MlDsa65SlhDsa));
        assert_eq!(AlgSuite::from_u16(0x0003), None);
        assert_eq!(AlgSuite::from_u16(0xFFFF), None);
        // The registered suite maps to the standardized OID.
        assert_eq!(AlgSuite::MlDsa65Ed25519.oid(), "1.3.6.1.5.5.7.6.48");
    }

    #[test]
    fn red_suite_swap_breaks_sig() {
        let v = RefSigner;
        let p = Party::new(&v, 1);
        let root = SelfSignedRoot::mint(&v, &p.cls_seed, &p.pq_seed, scope(), 9999);
        // Sign a block (a root self-sig) as v1.
        let msg = SelfSignedRoot::canonical_bytes(
            &root.classical_pub,
            &root.pq_pub,
            &root.root_scope,
            root.alg_suite,
            root.not_after,
        );
        let mut sig = HybridSig::sign(&v, AlgSuite::MlDsa65Ed25519, &p.cls_seed, &p.pq_seed, &msg);
        assert!(sig.verify(&v, &root.classical_pub, &root.pq_pub, &msg));
        // Flip the stored suite code point WITHOUT re-signing → must now fail.
        sig.alg_suite_raw = 0x0002; // not the suite the bytes were signed under
        assert!(sig.suite().is_none() || !sig.verify(&v, &root.classical_pub, &root.pq_pub, &msg));
        assert!(!sig.verify(&v, &root.classical_pub, &root.pq_pub, &msg));
    }

    #[test]
    fn red_oid_remap_is_one_line() {
        // The pre-RFC OID must be mapped in EXACTLY ONE match arm (single remap point).
        // The needle is assembled via `concat!` so this guard's own source line does not
        // self-match the `=> CONST` arm pattern it searches for.
        let needle = concat!("=> OID_MLDSA65", "_ED25519_SHA512");
        let src = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/capability_cert.rs"
        ));
        let arm_count = src.lines().filter(|l| l.contains(needle)).count();
        assert_eq!(arm_count, 1, "OID must be mapped in exactly one match arm");
    }

    // ── M2: hybrid-sign the chain (close the Ed25519-only gap) ──────────────────

    #[test]
    fn red_classical_only_link_rejected() {
        let v = RefSigner;
        let owner = Party::new(&v, 1);
        let hub = Party::new(&v, 2);
        let root = SelfSignedRoot::mint(&v, &owner.cls_seed, &owner.pq_seed, scope(), 9999);
        let mut link = CertDelegation::sign(
            &v,
            &owner.cls_seed,
            &owner.pq_seed,
            root.classical_pub,
            root.pq_pub.clone(),
            hub.cls_pub,
            hub.pq_pub.clone(),
            scope(),
            scope(),
            false,
            AlgSuite::MlDsa65Ed25519,
            9999,
            [1u8; 8],
        );
        // Valid Ed25519 leg, but ZEROED PQ leg → RequireBoth must fail.
        link.sig.pq = vec![0u8; 32];
        assert!(!link.verify_signature(&v));

        let mut roster = AnchorRoster::new();
        root.enroll(&mut roster);
        let cap = Capability::new_hybrid(hub.cls_pub, hub.pq_pub.clone(), scope(), [9u8; 8], 9999);
        let store = RevocationStore::new();
        assert_eq!(
            verify_chain_hybrid(&v, &roster, &store, &root, &[link], &cap, 0),
            Err(CertError::BadSignature)
        );
    }

    #[test]
    fn red_pq_forged_classical_valid() {
        let v = RefSigner;
        let owner = Party::new(&v, 1);
        let hub = Party::new(&v, 2);
        let root = SelfSignedRoot::mint(&v, &owner.cls_seed, &owner.pq_seed, scope(), 9999);
        let mut link = CertDelegation::sign(
            &v,
            &owner.cls_seed,
            &owner.pq_seed,
            root.classical_pub,
            root.pq_pub.clone(),
            hub.cls_pub,
            hub.pq_pub.clone(),
            scope(),
            scope(),
            false,
            AlgSuite::MlDsa65Ed25519,
            9999,
            [1u8; 8],
        );
        // Classical leg valid; PQ leg copied from a DIFFERENT message → AND degrades to OR? NO.
        let forged_pq = v.sign_pq(&owner.pq_seed, b"a totally different message");
        link.sig.pq = forged_pq;
        // The classical leg alone still validates (showing the PQ forgery — not the
        // classical leg — is what saves us from an OR-shortcut).
        assert!(verify_classical_only(&link, &v));
        // Full RequireBoth verify MUST reject the forged PQ leg — there is no OR fallback.
        assert!(!link.verify_signature(&v));
    }

    // Helper: confirm the classical leg alone validates (to show the PQ forgery is the
    // thing that saves us from an OR-shortcut). Not part of the public API.
    fn verify_classical_only(link: &CertDelegation, v: &RefSigner) -> bool {
        let inner = CertDelegation::canonical_bytes(
            &link.issued_by,
            &link.issued_by_pq,
            &link.subject,
            &link.subject_pq,
            &link.scope,
            &link.effect,
            link.may_delegate,
            link.expiry,
            &link.nonce,
        );
        let bound = suite_bound(link.alg_suite, &inner);
        v.verify_classical(&link.issued_by, &bound, &link.sig.classical)
    }

    #[test]
    fn red_block_reordering() {
        let v = RefSigner;
        let parties: Vec<Party> = (1u8..=5).map(|i| Party::new(&v, i)).collect();
        let root =
            SelfSignedRoot::mint(&v, &parties[0].cls_seed, &parties[0].pq_seed, scope(), 9999);
        let mut chain = linear_chain(&v, &parties.as_slice(), &root, true, 9999);
        // Swap the first two links → first link's issuer is no longer the root.
        chain.swap(0, 1);
        let mut roster = AnchorRoster::new();
        root.enroll(&mut roster);
        let last = &parties[parties.len() - 1];
        let cap =
            Capability::new_hybrid(last.cls_pub, last.pq_pub.clone(), scope(), [9u8; 8], 9999);
        let store = RevocationStore::new();
        assert_eq!(
            verify_chain_hybrid(&v, &roster, &store, &root, &chain, &cap, 0),
            Err(CertError::UnknownIssuer)
        );
    }

    #[test]
    fn red_chain_truncation() {
        let v = RefSigner;
        let parties: Vec<Party> = (1u8..=5).map(|i| Party::new(&v, i)).collect();
        let root =
            SelfSignedRoot::mint(&v, &parties[0].cls_seed, &parties[0].pq_seed, scope(), 9999);
        let chain = linear_chain(&v, &parties.as_slice(), &root, true, 9999);
        // Drop the tail so an intermediate becomes the tail; its subject != cap.subject_key.
        let truncated = &chain[..chain.len() - 1];
        let mut roster = AnchorRoster::new();
        root.enroll(&mut roster);
        let last = &parties[parties.len() - 1];
        let cap =
            Capability::new_hybrid(last.cls_pub, last.pq_pub.clone(), scope(), [9u8; 8], 9999);
        let store = RevocationStore::new();
        assert_eq!(
            verify_chain_hybrid(&v, &roster, &store, &root, truncated, &cap, 0),
            Err(CertError::ScopeViolation)
        );

        // Also: an over-long chain (beyond MAX_CHAIN_LEN) is rejected.
        let many: Vec<Party> = (1u8..=8).map(|i| Party::new(&v, i)).collect();
        let root2 = SelfSignedRoot::mint(&v, &many[0].cls_seed, &many[0].pq_seed, scope(), 9999);
        let long = linear_chain(&v, &many.as_slice(), &root2, true, 9999);
        assert!(long.len() > MAX_CHAIN_LEN);
        let last2 = &many[many.len() - 1];
        let cap2 =
            Capability::new_hybrid(last2.cls_pub, last2.pq_pub.clone(), scope(), [9u8; 8], 9999);
        let mut roster2 = AnchorRoster::new();
        root2.enroll(&mut roster2);
        let store2 = RevocationStore::new();
        assert_eq!(
            verify_chain_hybrid(&v, &roster2, &store2, &root2, &long, &cap2, 0),
            Err(CertError::MaxDepthExceeded)
        );
    }

    // ── M3: chain verification with revocation + alg_suite gate ──────────────────

    #[test]
    fn red_revoked_key_rejected() {
        let v = RefSigner;
        let owner = Party::new(&v, 1);
        let hub = Party::new(&v, 2);
        let root = SelfSignedRoot::mint(&v, &owner.cls_seed, &owner.pq_seed, scope(), 9999);
        let link = CertDelegation::sign(
            &v,
            &owner.cls_seed,
            &owner.pq_seed,
            root.classical_pub,
            root.pq_pub.clone(),
            hub.cls_pub,
            hub.pq_pub.clone(),
            scope(),
            scope(),
            false,
            AlgSuite::MlDsa65Ed25519,
            9999,
            [1u8; 8],
        );
        let mut roster = AnchorRoster::new();
        root.enroll(&mut roster);
        let cap = Capability::new_hybrid(hub.cls_pub, hub.pq_pub.clone(), scope(), [9u8; 8], 9999);
        let mut store = RevocationStore::new();
        // Healthy chain verifies.
        assert_eq!(
            verify_chain_hybrid(&v, &roster, &store, &root, &[link.clone()], &cap, 0),
            Ok(())
        );
        // Revoke the hub's key → rejected.
        store.set.revoke_key(hub.cls_pub);
        assert_eq!(
            verify_chain_hybrid(&v, &roster, &store, &root, &[link], &cap, 0),
            Err(CertError::Revoked)
        );
    }

    #[test]
    fn red_revoked_cap_hash_rejected() {
        let v = RefSigner;
        let owner = Party::new(&v, 1);
        let hub = Party::new(&v, 2);
        let root = SelfSignedRoot::mint(&v, &owner.cls_seed, &owner.pq_seed, scope(), 9999);
        let link = CertDelegation::sign(
            &v,
            &owner.cls_seed,
            &owner.pq_seed,
            root.classical_pub,
            root.pq_pub.clone(),
            hub.cls_pub,
            hub.pq_pub.clone(),
            scope(),
            scope(),
            false,
            AlgSuite::MlDsa65Ed25519,
            9999,
            [1u8; 8],
        );
        let mut roster = AnchorRoster::new();
        root.enroll(&mut roster);
        let cap = Capability::new_hybrid(hub.cls_pub, hub.pq_pub.clone(), scope(), [9u8; 8], 9999);
        let mut store = RevocationStore::new();
        assert_eq!(
            verify_chain_hybrid(&v, &roster, &store, &root, &[link.clone()], &cap, 0),
            Ok(())
        );
        // Revoke by cap hash (keys still live) → single-cap revocation.
        store.set.revoke_capability(revocation_hash(&cap));
        assert_eq!(
            verify_chain_hybrid(&v, &roster, &store, &root, &[link], &cap, 0),
            Err(CertError::Revoked)
        );
    }

    #[test]
    fn red_expired_root_rejected() {
        let v = RefSigner;
        let owner = Party::new(&v, 1);
        let hub = Party::new(&v, 2);
        // Root already expired at mint time.
        let root = SelfSignedRoot::mint(&v, &owner.cls_seed, &owner.pq_seed, scope(), 10);
        let link = CertDelegation::sign(
            &v,
            &owner.cls_seed,
            &owner.pq_seed,
            root.classical_pub,
            root.pq_pub.clone(),
            hub.cls_pub,
            hub.pq_pub.clone(),
            scope(),
            scope(),
            false,
            AlgSuite::MlDsa65Ed25519,
            9999,
            [1u8; 8],
        );
        let mut roster = AnchorRoster::new();
        root.enroll(&mut roster);
        let cap = Capability::new_hybrid(hub.cls_pub, hub.pq_pub.clone(), scope(), [9u8; 8], 9999);
        let store = RevocationStore::new();
        assert_eq!(
            verify_chain_hybrid(&v, &roster, &store, &root, &[link], &cap, 100),
            Err(CertError::Expired)
        );
    }

    // ── M4: self-signed hybrid roots + optional detached dowiz co-sign ───────────

    #[test]
    fn red_root_without_dowiz_is_valid() {
        let v = RefSigner;
        let owner = Party::new(&v, 1);
        // Mint a root with NO dowiz co-sign; it must verify on its own merit.
        let root = SelfSignedRoot::mint(&v, &owner.cls_seed, &owner.pq_seed, scope(), 9999);
        assert_eq!(root.verify_self(&v, 0), Ok(()));
    }

    #[test]
    fn red_forged_node_id_rejected() {
        let v = RefSigner;
        let owner = Party::new(&v, 1);
        let other = Party::new(&v, 2);
        let mut root = SelfSignedRoot::mint(&v, &owner.cls_seed, &owner.pq_seed, scope(), 9999);
        // Overwrite node_id with a hash that does NOT bind both keys → rejected.
        root.node_id = NodeId::from_keys(&other.pq_pub, &other.cls_pub);
        assert_eq!(root.verify_self(&v, 0), Err(CertError::NodeIdMismatch));
    }

    #[test]
    fn red_dowiz_cosign_absence_never_blocks() {
        let v = RefSigner;
        let owner = Party::new(&v, 1);
        let root = SelfSignedRoot::mint(&v, &owner.cls_seed, &owner.pq_seed, scope(), 9999);
        // No co-sign present at all → root still valid.
        assert_eq!(root.verify_self(&v, 0), Ok(()));
    }

    #[test]
    fn red_bad_dowiz_cosign_ignored() {
        let v = RefSigner;
        let owner = Party::new(&v, 1);
        let dowiz = Party::new(&v, 7);
        let root = SelfSignedRoot::mint(&v, &owner.cls_seed, &owner.pq_seed, scope(), 9999);
        // A PRESENT but INVALID co-sign (signed by the wrong/different key material):
        // forge it by signing over the root with a NON-dowiz seed.
        let bad = DowizCoSign::sign(&v, &owner.cls_seed, &owner.pq_seed, &root);
        // The bad co-sign must NOT verify against the real dowiz keys...
        assert!(!bad.verify(&v, &dowiz.cls_pub, &dowiz.pq_pub, &root));
        // ...yet the root still verifies on its own merit (co-sign is additive-only).
        assert_eq!(root.verify_self(&v, 0), Ok(()));
    }

    // ── M5: owner multi-hub delegation (offline, no dowiz) ──────────────────────

    #[test]
    fn red_owner_mints_child_offline() {
        let v = RefSigner;
        let owner = Party::new(&v, 1);
        let hub = Party::new(&v, 2);
        // Owner mints its own self-signed root (no network, no dowiz).
        let root = SelfSignedRoot::mint(&v, &owner.cls_seed, &owner.pq_seed, scope(), 9999);
        // Owner delegates a child hub cert (single hop, may_delegate=false).
        let link = CertDelegation::sign(
            &v,
            &owner.cls_seed,
            &owner.pq_seed,
            root.classical_pub,
            root.pq_pub.clone(),
            hub.cls_pub,
            hub.pq_pub.clone(),
            scope(),
            scope(),
            false,
            AlgSuite::MlDsa65Ed25519,
            9999,
            [1u8; 8],
        );
        // The hub verifies knowing ONLY the owner root's public material (offline).
        let mut roster = AnchorRoster::new();
        root.enroll(&mut roster);
        let cap = Capability::new_hybrid(hub.cls_pub, hub.pq_pub.clone(), scope(), [9u8; 8], 9999);
        let store = RevocationStore::new();
        assert_eq!(
            verify_chain_hybrid(&v, &roster, &store, &root, &[link], &cap, 0),
            Ok(())
        );
    }

    #[test]
    fn red_child_cannot_redelegate() {
        let v = RefSigner;
        let owner = Party::new(&v, 1);
        let hub = Party::new(&v, 2);
        let grandchild = Party::new(&v, 3);
        let root = SelfSignedRoot::mint(&v, &owner.cls_seed, &owner.pq_seed, scope(), 9999);
        // Owner -> hub (may_delegate=false).
        let l1 = CertDelegation::sign(
            &v,
            &owner.cls_seed,
            &owner.pq_seed,
            root.classical_pub,
            root.pq_pub.clone(),
            hub.cls_pub,
            hub.pq_pub.clone(),
            scope(),
            scope(),
            false,
            AlgSuite::MlDsa65Ed25519,
            9999,
            [1u8; 8],
        );
        // Hub tries to append a grandchild (forged, but structurally it would be a 2nd link).
        let l2 = CertDelegation::sign(
            &v,
            &hub.cls_seed,
            &hub.pq_seed,
            hub.cls_pub,
            hub.pq_pub.clone(),
            grandchild.cls_pub,
            grandchild.pq_pub.clone(),
            scope(),
            scope(),
            false,
            AlgSuite::MlDsa65Ed25519,
            9999,
            [2u8; 8],
        );
        let mut roster = AnchorRoster::new();
        root.enroll(&mut roster);
        let cap = Capability::new_hybrid(
            grandchild.cls_pub,
            grandchild.pq_pub.clone(),
            scope(),
            [9u8; 8],
            9999,
        );
        let store = RevocationStore::new();
        assert_eq!(
            verify_chain_hybrid(&v, &roster, &store, &root, &[l1, l2], &cap, 0),
            Err(CertError::MaxDepthExceeded)
        );
    }

    #[test]
    fn red_child_cannot_widen_scope() {
        let v = RefSigner;
        let owner = Party::new(&v, 1);
        let hub = Party::new(&v, 2);
        let root = SelfSignedRoot::mint(&v, &owner.cls_seed, &owner.pq_seed, scope(), 9999);
        // Child attempts a scope NOT subset of the root scope (Route:Send vs AgentBridge:AdmitAgent).
        let wide = Scope::single(Resource::Route, Action::Send);
        let link = CertDelegation::sign(
            &v,
            &owner.cls_seed,
            &owner.pq_seed,
            root.classical_pub,
            root.pq_pub.clone(),
            hub.cls_pub,
            hub.pq_pub.clone(),
            wide.clone(),
            wide,
            false,
            AlgSuite::MlDsa65Ed25519,
            9999,
            [1u8; 8],
        );
        let mut roster = AnchorRoster::new();
        root.enroll(&mut roster);
        let cap = Capability::new_hybrid(hub.cls_pub, hub.pq_pub.clone(), scope(), [9u8; 8], 9999);
        let store = RevocationStore::new();
        assert_eq!(
            verify_chain_hybrid(&v, &roster, &store, &root, &[link], &cap, 0),
            Err(CertError::ScopeViolation)
        );
    }

    #[test]
    fn red_cross_owner_forgery() {
        let v = RefSigner;
        let owner_a = Party::new(&v, 1);
        let owner_b = Party::new(&v, 2);
        let hub = Party::new(&v, 3);
        // Owner A's chain...
        let root_a = SelfSignedRoot::mint(&v, &owner_a.cls_seed, &owner_a.pq_seed, scope(), 9999);
        let link_a = CertDelegation::sign(
            &v,
            &owner_a.cls_seed,
            &owner_a.pq_seed,
            root_a.classical_pub,
            root_a.pq_pub.clone(),
            hub.cls_pub,
            hub.pq_pub.clone(),
            scope(),
            scope(),
            false,
            AlgSuite::MlDsa65Ed25519,
            9999,
            [1u8; 8],
        );
        // ...presented under owner B's anchor → root issuer not enrolled → UnknownIssuer.
        let root_b = SelfSignedRoot::mint(&v, &owner_b.cls_seed, &owner_b.pq_seed, scope(), 9999);
        let mut roster_b = AnchorRoster::new();
        root_b.enroll(&mut roster_b);
        let cap = Capability::new_hybrid(hub.cls_pub, hub.pq_pub.clone(), scope(), [9u8; 8], 9999);
        let store = RevocationStore::new();
        assert_eq!(
            verify_chain_hybrid(&v, &roster_b, &store, &root_a, &[link_a], &cap, 0),
            Err(CertError::UnknownIssuer)
        );
    }

    // ── M6: suite negotiation + downgrade binding + overlap rotation ─────────────

    #[test]
    fn red_downgrade_stripped_suite_detected() {
        let v = RefSigner;
        // v2 (strongest) offered first, then v1.
        let local = SuiteAdvertisement {
            offered: vec![AlgSuite::MlDsa65SlhDsa, AlgSuite::MlDsa65Ed25519],
        };
        let peer = SuiteAdvertisement {
            offered: vec![AlgSuite::MlDsa65Ed25519],
        };
        let neg = negotiate(&local, &peer).expect("common suite exists");
        // The peer advertised only `Ed25519`, so the strongest *common* suite is `Ed25519`
        // (SlhDsa is never on the wire from the peer). The downgrade is still detected via
        // the transcript hash below.
        assert_eq!(neg.chosen, AlgSuite::MlDsa65Ed25519);
        let transcript_orig = neg.transcript_hash;

        // MITM strips the strong suite from the local list before it reaches the peer.
        let local_stripped = SuiteAdvertisement {
            offered: vec![AlgSuite::MlDsa65Ed25519],
        };
        let neg_stripped = negotiate(&local_stripped, &peer).expect("still a common suite");
        assert_eq!(neg_stripped.chosen, AlgSuite::MlDsa65Ed25519);
        let transcript_stripped = neg_stripped.transcript_hash;
        // The transcript hash changed → a first-frame sig bound to the original fails.
        assert_ne!(transcript_orig, transcript_stripped);

        // Simulate the first-frame signature: it is signed over the ORIGINAL transcript.
        let seed = [42u8; 32];
        let frame_sig =
            HybridSig::sign(&v, AlgSuite::MlDsa65Ed25519, &seed, &seed, &transcript_orig);
        // It MUST NOT verify against the stripped transcript (downgrade detected).
        let pubk = v.classical_public(&seed);
        let pqk = v.pq_public(&seed);
        assert!(!frame_sig.verify(&v, &pubk, &pqk, &transcript_stripped));
        // ...but it DOES verify against the original transcript.
        assert!(frame_sig.verify(&v, &pubk, &pqk, &transcript_orig));
    }

    #[test]
    fn red_overlap_accepts_both_then_retires_old() {
        // Stable(v1) accepts v1 only.
        let stable = RotationState::Stable {
            suite: AlgSuite::MlDsa65Ed25519,
        };
        assert!(stable.accepts(AlgSuite::MlDsa65Ed25519, 0));
        assert!(!stable.accepts(AlgSuite::MlDsa65SlhDsa, 0));

        // Overlapping(v1, v2, until=100): during window both accepted.
        let overlapping = RotationState::Overlapping {
            old: AlgSuite::MlDsa65Ed25519,
            new: AlgSuite::MlDsa65SlhDsa,
            overlap_until: 100,
        };
        assert!(overlapping.accepts(AlgSuite::MlDsa65Ed25519, 50));
        assert!(overlapping.accepts(AlgSuite::MlDsa65SlhDsa, 50));
        assert!(!overlapping.accepts(AlgSuite::MlDsa65Ed25519, 200));
        // After the window, only the new suite is accepted (old retired).
        assert!(overlapping.accepts(AlgSuite::MlDsa65SlhDsa, 200));
    }

    #[test]
    fn red_no_common_suite_fails_closed() {
        let local = SuiteAdvertisement {
            offered: vec![AlgSuite::MlDsa65Ed25519],
        };
        let peer = SuiteAdvertisement {
            offered: vec![AlgSuite::MlDsa65SlhDsa],
        };
        assert_eq!(negotiate(&local, &peer), None);
    }

    // ── M7: revocation blobs ────────────────────────────────────────────────────

    #[test]
    fn red_unsigned_revocation_blob_ignored() {
        let v = RefSigner;
        let owner = Party::new(&v, 1);
        let hub = Party::new(&v, 2);
        let root = SelfSignedRoot::mint(&v, &owner.cls_seed, &owner.pq_seed, scope(), 9999);
        let mut blob = RevocationBlob::sign(
            &v,
            &root,
            &owner.cls_seed,
            &owner.pq_seed,
            vec![hub.cls_pub],
            vec![],
            1,
            9999,
        );
        // Corrupt the signature → blob is rejected and the store is UNCHANGED.
        blob.sig.classical = vec![0u8; 32];
        let mut store = RevocationStore::new();
        let before = store.is_revoked_key(&hub.cls_pub);
        assert_eq!(
            store.apply_blob(&blob, &v, &root, 0),
            Err(CertError::BadSignature)
        );
        assert_eq!(store.is_revoked_key(&hub.cls_pub), before);
    }

    #[test]
    fn red_expired_revocation_blob_ignored() {
        let v = RefSigner;
        let owner = Party::new(&v, 1);
        let hub = Party::new(&v, 2);
        let root = SelfSignedRoot::mint(&v, &owner.cls_seed, &owner.pq_seed, scope(), 9999);
        let blob = RevocationBlob::sign(
            &v,
            &root,
            &owner.cls_seed,
            &owner.pq_seed,
            vec![hub.cls_pub],
            vec![],
            1,
            10, // expires at tick 10
        );
        let mut store = RevocationStore::new();
        assert_eq!(
            store.apply_blob(&blob, &v, &root, 100),
            Err(CertError::Expired)
        );
        assert!(!store.is_revoked_key(&hub.cls_pub));
    }

    #[test]
    fn red_stale_seq_cannot_unrevoke() {
        let v = RefSigner;
        let owner = Party::new(&v, 1);
        let hub = Party::new(&v, 2);
        let root = SelfSignedRoot::mint(&v, &owner.cls_seed, &owner.pq_seed, scope(), 9999);
        let mut store = RevocationStore::new();

        // seq=5 revokes key K.
        let blob5 = RevocationBlob::sign(
            &v,
            &root,
            &owner.cls_seed,
            &owner.pq_seed,
            vec![hub.cls_pub],
            vec![],
            5,
            9999,
        );
        assert_eq!(
            store.apply_blob(&blob5, &v, &root, 0),
            Ok(ApplyOutcome::Applied)
        );
        assert!(store.is_revoked_key(&hub.cls_pub));

        // A replayed seq=3 blob NOT containing K must be ignored (cannot un-revoke).
        let blob3 = RevocationBlob::sign(
            &v,
            &root,
            &owner.cls_seed,
            &owner.pq_seed,
            vec![], // does NOT revoke K
            vec![],
            3,
            9999,
        );
        assert_eq!(
            store.apply_blob(&blob3, &v, &root, 0),
            Ok(ApplyOutcome::IgnoredStaleSeq)
        );
        // K stays revoked (revocation is monotone).
        assert!(store.is_revoked_key(&hub.cls_pub));
    }

    // ── D10: no break-glass / recovery path anywhere in this module ──────────────

    #[test]
    fn no_breakglass_recovery_path_check() {
        let src = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/src/capability_cert.rs"
        ));
        let lower = src.to_lowercase();
        // The forbidden tokens are assembled via `concat!` so this guard's own literal
        // list does not self-match the very tokens it prohibits in production code.
        for bad in [
            concat!("break", "_glass"),
            concat!("recovery", "_key"),
            concat!("recover", "_root"),
            concat!("master", "_key"),
            concat!("back", "door"),
        ] {
            assert!(
                !lower.contains(bad),
                "forbidden recovery-path token found: {}",
                bad
            );
        }
    }

    // ── happy-path: sign / verify / delegate / revoke end-to-end ─────────────────

    #[test]
    fn cert_chain_roundtrip_happy_path() {
        let v = RefSigner;
        let owner = Party::new(&v, 1);
        let hub = Party::new(&v, 2);
        let root = SelfSignedRoot::mint(&v, &owner.cls_seed, &owner.pq_seed, scope(), 9999);
        let link = CertDelegation::sign(
            &v,
            &owner.cls_seed,
            &owner.pq_seed,
            root.classical_pub,
            root.pq_pub.clone(),
            hub.cls_pub,
            hub.pq_pub.clone(),
            scope(),
            scope(),
            false,
            AlgSuite::MlDsa65Ed25519,
            9999,
            [1u8; 8],
        );
        let mut roster = AnchorRoster::new();
        root.enroll(&mut roster);
        let cap = Capability::new_hybrid(hub.cls_pub, hub.pq_pub.clone(), scope(), [9u8; 8], 9999);
        let mut store = RevocationStore::new();

        // Sign + verify (delegate) succeeds.
        assert!(link.verify_signature(&v));
        assert_eq!(
            verify_chain_hybrid(&v, &roster, &store, &root, &[link.clone()], &cap, 0),
            Ok(())
        );

        // Owner revokes the hub key via a signed blob (gossiped through RevocationSet::merge).
        let blob = RevocationBlob::sign(
            &v,
            &root,
            &owner.cls_seed,
            &owner.pq_seed,
            vec![hub.cls_pub],
            vec![],
            1,
            9999,
        );
        assert_eq!(
            store.apply_blob(&blob, &v, &root, 0),
            Ok(ApplyOutcome::Applied)
        );
        // Now the same chain is rejected (revoke).
        assert_eq!(
            verify_chain_hybrid(&v, &roster, &store, &root, &[link], &cap, 0),
            Err(CertError::Revoked)
        );
    }
}
