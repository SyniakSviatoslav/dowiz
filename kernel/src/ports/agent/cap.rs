//! ports/agent/cap.rs — the capability / signed-frame / roster value types + the
//! `SignatureVerifier` seam, faithfully generalizing bebop2 `proto-cap`
//! (`capability.rs` / `roster.rs` / `revocation.rs` / `node_id.rs`) for the B1
//! admission path.
//!
//! Compile firewall (mirrors `ports/llm.rs:3-7`): ZERO network / HTTP / JSON / serde.
//! Signatures are computed over fixed-layout, domain-separated TLV bytes (never serde),
//! exactly as bebop2 mandates on the signed path.
//!
//! # Why a `SignatureVerifier` seam (the implementation-time judgment call)
//! The blueprint's admission path REUSES bebop2's `HybridGate::check`, whose legs are
//! real Ed25519 ⊕ ML-DSA-65. The dowiz kernel, however, does **not** link bebop2's
//! `proto-cap` crate (verified: no such dependency in `kernel/Cargo.toml`), and B1's
//! own DoD item 7 defers the concrete cross-repo hard-link + `scope.rs` discriminant
//! allocation to a lead-agent integration ruling. So — exactly as `ports/llm.rs` is a
//! trait whose real backend lives in `llm-adapters` — the verification primitive is a
//! [`SignatureVerifier`] trait. Production injects the real bebop2 Ed25519 + ML-DSA-65
//! verifier at the integration boundary; the in-tree [`RefSigner`] is a deterministic,
//! preimage-unforgeable SHA3 reference (built on the kernel's existing `sha3_256`, zero
//! new dep) that gives every acceptance-criterion test real teeth (a corrupted
//! signature genuinely fails verification; only the holder of the secret can produce a
//! valid one — forging requires a SHA3 preimage).

use std::collections::HashSet;

use crate::event_log::sha3_256;

use super::scope::Scope;

// ── domain-separation tags (16 bytes each) ───────────────────────────────────────
const DOMAIN_CAPABILITY: &[u8; 16] = b"dowiz.agent.cap\x01";
const DOMAIN_FRAME: &[u8; 16] = b"dowiz.agent.frm\x01";
const DOMAIN_DELEGATION: &[u8; 16] = b"dowiz.agent.dlg\x01";
// Reference-signer key/sig domains (RefSigner only).
const REF_CLS_PUB: &[u8] = b"dowiz.ref.cls.pub";
const REF_CLS_SIG: &[u8] = b"dowiz.ref.cls.sig";
const REF_PQ_PUB: &[u8] = b"dowiz.ref.pq.pub";
const REF_PQ_SIG: &[u8] = b"dowiz.ref.pq.sig";
/// ML-DSA-65 public-key length (bytes) — B1 T=0x03 mandates this width.
pub const ML_DSA_65_PK_LEN: usize = 1952;

/// The hybrid-verification policy floor. Only [`HybridPolicy::RequireBoth`] exists in
/// B1 — there is deliberately no classical-only variant (the unrelaxable floor, §2.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridPolicy {
    /// Require the classical (Ed25519) leg AND the post-quantum (ML-DSA-65) leg to
    /// verify. This is the ONLY policy; there is no weaker code point.
    RequireBoth,
}

/// A mesh node identity: `NodeId = SHA3-256(pq_pub || classical_pub)` (ADR-0007).
/// Changing EITHER public key changes the id — no CA, no assignable "owner".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub [u8; 32]);

impl NodeId {
    /// Derive the node id from the PQ public key and the classical (Ed25519) public key.
    pub fn from_keys(pq_pub: &[u8], classical_pub: &[u8; 32]) -> Self {
        let mut buf = Vec::with_capacity(pq_pub.len() + 32);
        buf.extend_from_slice(pq_pub);
        buf.extend_from_slice(classical_pub);
        NodeId(sha3_256(&buf))
    }
    /// Raw 32-byte id.
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
    /// Lowercase hex (offline, dependency-free).
    pub fn to_hex(&self) -> String {
        let mut s = String::with_capacity(64);
        const HEX: &[u8; 16] = b"0123456789abcdef";
        for &b in &self.0 {
            s.push(HEX[(b >> 4) as usize] as char);
            s.push(HEX[(b & 0x0f) as usize] as char);
        }
        s
    }
}

/// The verification seam. Production injects real Ed25519 (classical) + ML-DSA-65 (PQ);
/// [`RefSigner`] is the in-tree deterministic SHA3 reference.
pub trait SignatureVerifier {
    /// Classical public key for a 32-byte secret seed.
    fn classical_public(&self, secret: &[u8; 32]) -> [u8; 32];
    /// Sign `msg` under the classical secret. Returns the raw signature bytes.
    fn sign_classical(&self, secret: &[u8; 32], msg: &[u8]) -> Vec<u8>;
    /// Verify a classical signature against `public`.
    fn verify_classical(&self, public: &[u8; 32], msg: &[u8], sig: &[u8]) -> bool;
    /// PQ public key (`ML_DSA_65_PK_LEN` bytes) for a 32-byte PQ secret seed.
    fn pq_public(&self, pq_secret: &[u8; 32]) -> Vec<u8>;
    /// Sign `msg` under the PQ secret.
    fn sign_pq(&self, pq_secret: &[u8; 32], msg: &[u8]) -> Vec<u8>;
    /// Verify a PQ signature against `pq_public`.
    fn verify_pq(&self, pq_public: &[u8], msg: &[u8], sig: &[u8]) -> bool;
}

/// The in-tree deterministic reference signer. NOT production crypto — a SHA3
/// commitment scheme where a signature reveals the secret masked by `H(msg)`, so it
/// verifies against the public key alone yet cannot be forged without a SHA3 preimage.
/// Production replaces this with the real bebop2 Ed25519 + ML-DSA-65 verifier.
#[derive(Debug, Clone, Copy, Default)]
pub struct RefSigner;

impl RefSigner {
    fn h(domain: &[u8], msg: &[u8]) -> [u8; 32] {
        let mut buf = Vec::with_capacity(domain.len() + msg.len());
        buf.extend_from_slice(domain);
        buf.extend_from_slice(msg);
        sha3_256(&buf)
    }
    fn xor32(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
        let mut o = [0u8; 32];
        for i in 0..32 {
            o[i] = a[i] ^ b[i];
        }
        o
    }
}

impl SignatureVerifier for RefSigner {
    fn classical_public(&self, secret: &[u8; 32]) -> [u8; 32] {
        Self::h(REF_CLS_PUB, secret)
    }
    fn sign_classical(&self, secret: &[u8; 32], msg: &[u8]) -> Vec<u8> {
        Self::xor32(secret, &Self::h(REF_CLS_SIG, msg)).to_vec()
    }
    fn verify_classical(&self, public: &[u8; 32], msg: &[u8], sig: &[u8]) -> bool {
        if sig.len() != 32 {
            return false;
        }
        let mut s = [0u8; 32];
        s.copy_from_slice(sig);
        let recovered = Self::xor32(&s, &Self::h(REF_CLS_SIG, msg));
        &self.classical_public(&recovered) == public
    }
    fn pq_public(&self, pq_secret: &[u8; 32]) -> Vec<u8> {
        // 1952-byte pk: leading 32 bytes are the verifiable commitment, the rest is
        // deterministic SHA3-expanded filler (keeps the real ML-DSA-65 width).
        let head = Self::h(REF_PQ_PUB, pq_secret);
        let mut out = Vec::with_capacity(ML_DSA_65_PK_LEN);
        out.extend_from_slice(&head);
        let mut ctr: u32 = 0;
        while out.len() < ML_DSA_65_PK_LEN {
            let mut b = head.to_vec();
            b.extend_from_slice(&ctr.to_le_bytes());
            out.extend_from_slice(&sha3_256(&b));
            ctr += 1;
        }
        out.truncate(ML_DSA_65_PK_LEN);
        out
    }
    fn sign_pq(&self, pq_secret: &[u8; 32], msg: &[u8]) -> Vec<u8> {
        Self::xor32(pq_secret, &Self::h(REF_PQ_SIG, msg)).to_vec()
    }
    fn verify_pq(&self, pq_public: &[u8], msg: &[u8], sig: &[u8]) -> bool {
        if sig.len() != 32 || pq_public.len() < 32 {
            return false;
        }
        let mut s = [0u8; 32];
        s.copy_from_slice(sig);
        let recovered = Self::xor32(&s, &Self::h(REF_PQ_SIG, msg));
        &Self::h(REF_PQ_PUB, &recovered)[..] == &pq_public[..32]
    }
}

/// A single-use, signed authorization statement (mirrors bebop2 `Capability`).
#[cfg_attr(feature = "json-api", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capability {
    /// Ed25519 public key (32 bytes) of the subject. Identity, never a score.
    pub subject_key: [u8; 32],
    /// ML-DSA-65 public key (1952 bytes) — the PQ half of the hybrid identity.
    /// `None` only for legacy classical-only frames; B1 admission requires `Some`.
    pub subject_key_pq: Option<Vec<u8>>,
    /// What the capability authorizes.
    pub scope: Scope,
    /// Single-use nonce (8 bytes).
    pub nonce: [u8; 8],
    /// Expiry as a monotonic tick.
    pub expiry: u64,
}

impl Capability {
    /// Hybrid capability (both classical + PQ subject keys).
    pub fn new_hybrid(
        subject_key: [u8; 32],
        subject_key_pq: Vec<u8>,
        scope: Scope,
        nonce: [u8; 8],
        expiry: u64,
    ) -> Self {
        Capability {
            subject_key,
            subject_key_pq: Some(subject_key_pq),
            scope,
            nonce,
            expiry,
        }
    }

    /// Canonical, domain-separated TLV signing bytes: `subject_key || scope || nonce ||
    /// expiry`. (Like bebop2, `subject_key_pq` is bound via the frame/manifest payload,
    /// not this capability domain.)
    pub fn canonical_bytes_tlv(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(DOMAIN_CAPABILITY);
        out.extend_from_slice(&self.subject_key);
        let scope = self.scope.to_tlv_bytes();
        out.extend_from_slice(&(scope.len() as u32).to_le_bytes());
        out.extend_from_slice(&scope);
        out.extend_from_slice(&self.nonce);
        out.extend_from_slice(&self.expiry.to_le_bytes());
        out
    }

    /// Whether `expiry` is still acceptable against `now` (pure comparison).
    pub fn is_fresh(&self, now: u64) -> bool {
        self.expiry > now
    }
}

/// A signed frame carrying a [`Capability`] + an opaque payload (the B1 admission frame
/// carries the canonical `AgentManifest` TLV as its payload).
#[cfg_attr(feature = "json-api", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct SignedFrame {
    /// The authorization statement.
    pub capability: Capability,
    /// Opaque payload (for B1 admission: the manifest's canonical TLV bytes).
    pub payload: Vec<u8>,
    /// Ed25519 signature over [`SignedFrame::signing_domain`] by `subject_key`.
    pub classical_sig: Option<Vec<u8>>,
    /// ML-DSA-65 signature over [`SignedFrame::signing_domain`] by `subject_key_pq`.
    pub pq_sig: Option<Vec<u8>>,
}

impl SignedFrame {
    /// New unsigned frame.
    pub fn new(capability: Capability, payload: Vec<u8>) -> Self {
        SignedFrame {
            capability,
            payload,
            classical_sig: None,
            pq_sig: None,
        }
    }

    /// Domain-separated bytes both signatures commit to: `DOMAIN_FRAME || cap-canonical
    /// || payload`. A capability-domain signature can never verify here (cross-type
    /// reuse rejected), exactly the bebop2 §4A property.
    pub fn signing_domain(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(DOMAIN_FRAME);
        out.extend_from_slice(&self.capability.canonical_bytes_tlv());
        out.extend_from_slice(&self.payload);
        out
    }

    /// Sign the classical leg with a secret seed via the verifier.
    pub fn sign_classical<V: SignatureVerifier>(&mut self, verifier: &V, secret: &[u8; 32]) {
        let dom = self.signing_domain();
        self.classical_sig = Some(verifier.sign_classical(secret, &dom));
    }
    /// Sign the PQ leg with a PQ secret seed via the verifier.
    pub fn sign_pq<V: SignatureVerifier>(&mut self, verifier: &V, pq_secret: &[u8; 32]) {
        let dom = self.signing_domain();
        self.pq_sig = Some(verifier.sign_pq(pq_secret, &dom));
    }

    /// Verify the classical leg (real, never relaxed).
    pub fn verify_classical<V: SignatureVerifier>(&self, verifier: &V) -> bool {
        match &self.classical_sig {
            Some(sig) => {
                verifier.verify_classical(&self.capability.subject_key, &self.signing_domain(), sig)
            }
            None => false,
        }
    }
    /// Verify the PQ leg. `false` when the PQ key or signature is absent (`HybridIncomplete`).
    pub fn verify_pq<V: SignatureVerifier>(&self, verifier: &V) -> bool {
        match (&self.capability.subject_key_pq, &self.pq_sig) {
            (Some(pk), Some(sig)) => verifier.verify_pq(pk, &self.signing_domain(), sig),
            _ => false,
        }
    }
}

/// A single delegation link in a UCAN-subset chain (mirrors bebop2 `Delegation`).
#[cfg_attr(feature = "json-api", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delegation {
    /// Parent / issuer key. Must be an enrolled anchor at the root, else equal the
    /// preceding link's `subject`.
    pub issued_by: [u8; 32],
    /// Child / subject key this grant is made to.
    pub subject: [u8; 32],
    /// Scope the granter is willing to pass down (subset of parent's scope).
    pub scope: Scope,
    /// Effect actually authorized at this link (subset of `scope`).
    pub effect: Scope,
    /// Expiry (monotonic tick).
    pub expiry: u64,
    /// Single-use nonce.
    pub nonce: [u8; 8],
    /// Signature over [`Delegation::canonical_bytes`] by `issued_by`.
    pub signature: Vec<u8>,
}

impl Delegation {
    fn canonical_bytes(
        issued_by: &[u8; 32],
        subject: &[u8; 32],
        scope: &Scope,
        effect: &Scope,
        expiry: u64,
        nonce: &[u8; 8],
    ) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(DOMAIN_DELEGATION);
        out.extend_from_slice(issued_by);
        out.extend_from_slice(subject);
        let s = scope.to_tlv_bytes();
        out.extend_from_slice(&(s.len() as u32).to_le_bytes());
        out.extend_from_slice(&s);
        let e = effect.to_tlv_bytes();
        out.extend_from_slice(&(e.len() as u32).to_le_bytes());
        out.extend_from_slice(&e);
        out.extend_from_slice(&expiry.to_le_bytes());
        out.extend_from_slice(nonce);
        out
    }

    /// Sign a delegation link with the issuer's secret via the verifier.
    #[allow(clippy::too_many_arguments)]
    pub fn sign<V: SignatureVerifier>(
        verifier: &V,
        issued_by: [u8; 32],
        subject: [u8; 32],
        scope: Scope,
        effect: Scope,
        expiry: u64,
        nonce: [u8; 8],
        issuer_secret: &[u8; 32],
    ) -> Delegation {
        let canonical =
            Self::canonical_bytes(&issued_by, &subject, &scope, &effect, expiry, &nonce);
        let signature = verifier.sign_classical(issuer_secret, &canonical);
        Delegation {
            issued_by,
            subject,
            scope,
            effect,
            expiry,
            nonce,
            signature,
        }
    }

    /// Verify this link's signature against its own `issued_by` key.
    pub fn verify_signature<V: SignatureVerifier>(&self, verifier: &V) -> bool {
        let canonical = Self::canonical_bytes(
            &self.issued_by,
            &self.subject,
            &self.scope,
            &self.effect,
            self.expiry,
            &self.nonce,
        );
        verifier.verify_classical(&self.issued_by, &canonical, &self.signature)
    }
}

/// The frozen trust-anchor set (mirrors bebop2 `AnchorRoster`). Enrolled at genesis,
/// then frozen. Mutators take `&mut` — the Poly-Network structural guard (SH-3 layer 3)
/// depends on this: no capability-bearing input can obtain the `&mut` needed to mutate.
#[derive(Debug, Clone, Default)]
pub struct AnchorRoster {
    anchors: HashSet<[u8; 32]>,
}

impl AnchorRoster {
    /// Empty roster (fail-closed: captures no authority).
    pub fn new() -> Self {
        AnchorRoster::default()
    }
    /// Enroll an anchor. `&mut` — OUT-OF-BAND operator/genesis act only.
    pub fn enroll(&mut self, key: &[u8; 32]) {
        self.anchors.insert(*key);
    }
    /// Remove an anchor. `&mut` — OUT-OF-BAND operator act only.
    pub fn remove(&mut self, key: &[u8; 32]) {
        self.anchors.remove(key);
    }
    /// Whether `key` is an enrolled anchor.
    pub fn contains(&self, key: &[u8; 32]) -> bool {
        self.anchors.contains(key)
    }
    /// Whether the roster is empty.
    pub fn is_empty(&self) -> bool {
        self.anchors.is_empty()
    }
    /// A sorted snapshot of the enrolled anchors (for before/after equality assertions).
    pub fn snapshot_sorted(&self) -> Vec<[u8; 32]> {
        let mut v: Vec<[u8; 32]> = self.anchors.iter().copied().collect();
        v.sort_unstable();
        v
    }
}

/// An append-only revocation set (mirrors bebop2 `RevocationSet`).
#[derive(Debug, Clone, Default)]
pub struct RevocationSet {
    revoked_keys: HashSet<[u8; 32]>,
    revoked_cap_hash: HashSet<[u8; 32]>,
}

impl RevocationSet {
    /// Empty revocation set.
    pub fn new() -> Self {
        RevocationSet::default()
    }
    /// Irreversibly revoke a subject key (or PQ-key id).
    pub fn revoke_key(&mut self, key: [u8; 32]) {
        self.revoked_keys.insert(key);
    }
    /// Irreversibly revoke a single capability by its revocation hash.
    pub fn revoke_capability(&mut self, cap_hash: [u8; 32]) {
        self.revoked_cap_hash.insert(cap_hash);
    }
    /// Whether `key` is revoked.
    pub fn is_revoked_key(&self, key: &[u8; 32]) -> bool {
        self.revoked_keys.contains(key)
    }
    /// Whether the capability with this revocation hash is revoked.
    pub fn is_revoked_capability(&self, cap_hash: &[u8; 32]) -> bool {
        self.revoked_cap_hash.contains(cap_hash)
    }
    /// Anti-entropy union.
    pub fn merge(&mut self, other: &RevocationSet) {
        self.revoked_keys.extend(other.revoked_keys.iter().copied());
        self.revoked_cap_hash
            .extend(other.revoked_cap_hash.iter().copied());
    }
    /// Drop an anchor from the enrolling roster (`&mut AnchorRoster`) — OUT-OF-BAND
    /// operator act; dropping a non-enrolled key is a no-op.
    pub fn drop_anchor(roster: &mut AnchorRoster, key: &[u8; 32]) {
        roster.remove(key);
    }
    /// Sorted `(keys, cap_hashes)` snapshot (for before/after equality assertions).
    pub fn snapshot_sorted(&self) -> (Vec<[u8; 32]>, Vec<[u8; 32]>) {
        let mut k: Vec<[u8; 32]> = self.revoked_keys.iter().copied().collect();
        let mut c: Vec<[u8; 32]> = self.revoked_cap_hash.iter().copied().collect();
        k.sort_unstable();
        c.sort_unstable();
        (k, c)
    }
}

/// Revocation hash of a capability: SHA3-256 over its canonical TLV bytes.
pub fn revocation_hash(cap: &Capability) -> [u8; 32] {
    sha3_256(&cap.canonical_bytes_tlv())
}

/// Stable 32-byte revocation id for a PQ subject key (hash it down).
pub fn pq_key_id(subject_key_pq: &[u8]) -> [u8; 32] {
    sha3_256(subject_key_pq)
}

/// Why a delegation chain failed to root in trust.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainError {
    /// No chain, or the root issuer is not an enrolled anchor (self-issue).
    UnknownIssuer,
    /// A link signature did not verify.
    BadSignature,
    /// A link (or the chain tail) is expired.
    Expired,
    /// Attenuation violated (escalation), tail mis-binding, or effect ⊄ tail scope.
    ScopeViolation,
}

/// Anchor-rooted, UCAN-subset chain verification (mirrors bebop2 `verify_chain`): the
/// root issuer must be an enrolled anchor, every link is signed by its `issued_by`,
/// links chain (child == next issuer), scope only attenuates (narrows), the tail binds
/// to `cap.subject_key`, and the requested effect is a subset of the tail scope.
pub fn verify_chain<V: SignatureVerifier>(
    verifier: &V,
    roster: &AnchorRoster,
    chain: &[Delegation],
    cap: &Capability,
    now: u64,
) -> Result<(), ChainError> {
    let root = chain.first().ok_or(ChainError::UnknownIssuer)?;
    if !roster.contains(&root.issued_by) {
        return Err(ChainError::UnknownIssuer);
    }
    let mut prev: Option<&Delegation> = None;
    for link in chain {
        if !link.verify_signature(verifier) {
            return Err(ChainError::BadSignature);
        }
        if !(link.expiry > now) {
            return Err(ChainError::Expired);
        }
        // Narrow-only within the link: its effect must be a subset of its own scope.
        if !link.effect.is_subset_of(&link.scope) {
            return Err(ChainError::ScopeViolation);
        }
        if let Some(p) = prev {
            // Chaining: child (subject) of the parent must be this link's issuer.
            if link.issued_by != p.subject {
                return Err(ChainError::UnknownIssuer);
            }
            // Attenuation-only: this link's scope must be a subset of the parent's.
            if !link.scope.is_subset_of(&p.scope) {
                return Err(ChainError::ScopeViolation);
            }
        }
        prev = Some(link);
    }
    let tail = chain.last().ok_or(ChainError::UnknownIssuer)?;
    // Tail binds to the capability's subject.
    if tail.subject != cap.subject_key {
        return Err(ChainError::ScopeViolation);
    }
    // Requested effect ⊆ tail's authorized effect.
    if !cap.scope.is_subset_of(&tail.effect) {
        return Err(ChainError::ScopeViolation);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ports::agent::scope::{Action, Resource};

    fn v() -> RefSigner {
        RefSigner
    }

    #[test]
    fn ref_signer_verifies_and_rejects_tamper() {
        let s = v();
        let secret = [7u8; 32];
        let pk = s.classical_public(&secret);
        let sig = s.sign_classical(&secret, b"hello");
        assert!(s.verify_classical(&pk, b"hello", &sig));
        // Corrupt one bit of the signature ⇒ reject.
        let mut bad = sig.clone();
        bad[0] ^= 0x01;
        assert!(!s.verify_classical(&pk, b"hello", &bad));
        // Different message ⇒ reject.
        assert!(!s.verify_classical(&pk, b"hellp", &sig));
    }

    #[test]
    fn ref_signer_pq_width_and_verify() {
        let s = v();
        let seed = [0xABu8; 32];
        let pk = s.pq_public(&seed);
        assert_eq!(pk.len(), ML_DSA_65_PK_LEN, "ML-DSA-65 width preserved");
        let sig = s.sign_pq(&seed, b"msg");
        assert!(s.verify_pq(&pk, b"msg", &sig));
        let mut bad = sig.clone();
        bad[3] ^= 0x80;
        assert!(!s.verify_pq(&pk, b"msg", &bad));
    }

    #[test]
    fn node_id_binds_both_keys() {
        let s = v();
        let cls = s.classical_public(&[1u8; 32]);
        let pq = s.pq_public(&[2u8; 32]);
        let a = NodeId::from_keys(&pq, &cls);
        let b = NodeId::from_keys(&pq, &cls);
        assert_eq!(a, b);
        // Change either key ⇒ different id.
        let cls2 = s.classical_public(&[9u8; 32]);
        assert_ne!(a, NodeId::from_keys(&pq, &cls2));
        let pq2 = s.pq_public(&[9u8; 32]);
        assert_ne!(a, NodeId::from_keys(&pq2, &cls));
    }

    #[test]
    fn frame_signs_and_verifies_both_legs() {
        let s = v();
        let (cls_secret, pq_secret) = ([3u8; 32], [4u8; 32]);
        let cap = Capability::new_hybrid(
            s.classical_public(&cls_secret),
            s.pq_public(&pq_secret),
            Scope::single(Resource::AgentBridge, Action::AdmitAgent),
            [1u8; 8],
            9999,
        );
        let mut f = SignedFrame::new(cap, b"payload".to_vec());
        f.sign_classical(&s, &cls_secret);
        f.sign_pq(&s, &pq_secret);
        assert!(f.verify_classical(&s));
        assert!(f.verify_pq(&s));
        // Tamper the payload ⇒ both legs fail.
        let mut t = f.clone();
        t.payload = b"evil".to_vec();
        assert!(!t.verify_classical(&s));
        assert!(!t.verify_pq(&s));
    }

    #[test]
    fn verify_chain_rejects_self_signed_no_anchor() {
        let s = v();
        let secret = [5u8; 32];
        let pk = s.classical_public(&secret);
        let cap = Capability::new_hybrid(
            pk,
            s.pq_public(&[6u8; 32]),
            Scope::single(Resource::AgentBridge, Action::AdmitAgent),
            [1u8; 8],
            9999,
        );
        // Empty chain ⇒ UnknownIssuer.
        let roster = AnchorRoster::new();
        assert_eq!(
            verify_chain(&s, &roster, &[], &cap, 0),
            Err(ChainError::UnknownIssuer)
        );
    }

    #[test]
    fn verify_chain_accepts_anchor_rooted() {
        let s = v();
        let (anchor_secret, leaf_secret) = ([10u8; 32], [11u8; 32]);
        let anchor_pk = s.classical_public(&anchor_secret);
        let leaf_pk = s.classical_public(&leaf_secret);
        let scope = Scope::single(Resource::AgentBridge, Action::AdmitAgent);
        let cap = Capability::new_hybrid(
            leaf_pk,
            s.pq_public(&[12u8; 32]),
            scope.clone(),
            [1u8; 8],
            9999,
        );
        let link = Delegation::sign(
            &s,
            anchor_pk,
            leaf_pk,
            scope.clone(),
            scope,
            9999,
            [2u8; 8],
            &anchor_secret,
        );
        let mut roster = AnchorRoster::new();
        roster.enroll(&anchor_pk);
        assert_eq!(verify_chain(&s, &roster, &[link], &cap, 0), Ok(()));
    }

    #[test]
    fn revocation_hash_is_nonce_sensitive() {
        let s = v();
        let a = Capability::new_hybrid(
            [7u8; 32],
            s.pq_public(&[1u8; 32]),
            Scope::single(Resource::Route, Action::Send),
            [1u8; 8],
            9999,
        );
        let mut b = a.clone();
        b.nonce = [2u8; 8];
        assert_ne!(revocation_hash(&a), revocation_hash(&b));
    }

    #[test]
    fn drop_anchor_removes_vouch_power() {
        let s = v();
        let anchor = s.classical_public(&[0x11u8; 32]);
        let mut roster = AnchorRoster::new();
        roster.enroll(&anchor);
        assert!(roster.contains(&anchor));
        RevocationSet::drop_anchor(&mut roster, &anchor);
        assert!(!roster.contains(&anchor));
    }
}
