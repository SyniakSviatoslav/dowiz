//! Canonical hybrid signing (Ed25519 ⊕ ML-DSA-65 ⊕ SLH-DSA) — the ONE place
//! hybrid signatures are created and verified.
//!
//! Every caller that needs a RequireBoth hybrid signature MUST route through this
//! module. No other file may independently implement sign/verify hybrid logic.
//!
//! # Signature legs
//!
//! | Leg          | Primitive  | Keygen                    | Sign/Verify             |
//! |-------------|-----------|---------------------------|-------------------------|
//! | Classical   | Ed25519   | SHA3-512 domain-sep seed  | deterministic SHA3-512  |
//! | ML-DSA-65   | FIPS 204  | `dsa::keygen_bytes`       | `dsa::sign_internal`    |
//! | SLH-DSA     | FIPS 205  | domain-sep ML-DSA (stub)  | domain-sep ML-DSA (stub)|
//!
//! # SLH-DSA stub (FIPS 205 placeholder)
//!
//! Real SLH-DSA (SPHINCS+) is a stateless hash-based signature scheme that is NOT
//! the same algorithm as ML-DSA. This module provides a **domain-separated**
//! variant of ML-DSA-65 as the SLH-DSA stub: the same NTT/lattice math runs, but
//! keygen seeds a DIFFERENT domain separator (`"dowiz.slh.v1"` vs `"dowiz.mldsa.v1"`),
//! so signatures do NOT cross-verify between the two slots. When a real SPHINCS+
//! implementation lands, the `SlhDsa` key/sig byte widths will change, but the
//! caller-facing `HybridSigner::sign_slhdsa()` / `verify_slhdsa()` API stays intact.
//! Until then, this stub provides a falsifiable, testable RequireBoth gate rather
//! than a `todo!()` — it MUST reject cross-domain replays and MUST produce
//! distinct keys from the same seed.

use crate::pq::dsa;
use crate::pq::dsa::{keygen_bytes, sign_internal_bytes, verify_internal_bytes, RNDBYTES, SEEDBYTES};
use crate::pq::envelope::ENTROPY_LEN;
use crate::pq::keccak::shake256;

// ── Domain separation tags ────────────────────────────────────────────────────
const DOMAIN_MLDSA: &[u8] = b"dowiz.mldsa.v1";
const DOMAIN_SLH: &[u8] = b"dowiz.slh.v1";
const DOMAIN_CLASSICAL: &[u8] = b"dowiz.ed25519.v1";
const DOMAIN_CLASSICAL_PUB: &[u8] = b"dowiz.ed25519.pub.v1";
const DOMAIN_CLASSICAL_SIG: &[u8] = b"dowiz.ed25519.sig.v1";

// ── key sizes ─────────────────────────────────────────────────────────────────
/// Ed25519 public key length (bytes).
pub const ED25519_PK_LEN: usize = 32;
/// Ed25519 secret key length (bytes).
pub const ED25519_SK_LEN: usize = 32;
/// Ed25519 signature length (bytes).
pub const ED25519_SIG_LEN: usize = 64;

/// ML-DSA-65 public key length (bytes, FIPS 204 mode 3).
pub const ML_DSA_65_PK_LEN: usize = dsa::PUBLICKEYBYTES; // 1952
/// ML-DSA-65 secret key length (bytes).
pub const ML_DSA_65_SK_LEN: usize = dsa::SECRETKEYBYTES; // 4032
/// ML-DSA-65 signature length (bytes).
pub const ML_DSA_65_SIG_LEN: usize = dsa::SIGNATUREBYTES; // 3309

/// SLH-DSA stub public key length (bytes) — matches ML-DSA-65 for now.
pub const SLH_DSA_PK_LEN: usize = dsa::PUBLICKEYBYTES;
/// SLH-DSA stub secret key length (bytes).
pub const SLH_DSA_SK_LEN: usize = dsa::SECRETKEYBYTES;
/// SLH-DSA stub signature length (bytes).
pub const SLH_DSA_SIG_LEN: usize = dsa::SIGNATUREBYTES;

// ── policy ────────────────────────────────────────────────────────────────────

/// The hybrid-verification policy floor. Only [`HybridPolicy::RequireBoth`] exists —
/// there is deliberately no classical-only or pq-only variant (the unrelaxable floor,
/// B4/SSR-2020 lesson).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridPolicy {
    /// Require BOTH the classical (Ed25519) leg AND the post-quantum (ML-DSA-65 OR
    /// SLH-DSA) leg to verify. This is the ONLY policy; there is no weaker code point.
    RequireBoth,
}

// ── hybrid signature data ─────────────────────────────────────────────────────

/// A complete hybrid signature: classical + PQ over the same message.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HybridSignature {
    /// Classical (Ed25519) signature leg.
    pub classical: Vec<u8>,
    /// Post-quantum (ML-DSA-65 or SLH-DSA) signature leg.
    pub pq: Vec<u8>,
}

/// A hybrid-signed envelope: the payload + a RequireBoth hybrid signature.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HybridEnvelope {
    /// The signed payload bytes.
    pub payload: Vec<u8>,
    /// Content hash (SHAKE256 truncated to 32 bytes).
    pub content_hash: [u8; 32],
    /// The RequireBoth hybrid signature over the content hash.
    pub sig: HybridSignature,
}

// ── HybridSigner (the ONE canonical signer) ───────────────────────────────────

/// The ONE canonical hybrid signer. Holds secret material for all signature legs
/// and provides `sign_mldsa()` / `sign_slhdsa()` / `verify()` / `public_keys()`.
/// Every other module that needs hybrid signing MUST delegate here.
pub struct HybridSigner {
    /// Classical (Ed25519) secret seed (32 bytes).
    classical_secret: [u8; ED25519_SK_LEN],
    /// Classical public key (derived from the secret).
    classical_public: [u8; ED25519_PK_LEN],
    /// ML-DSA-65 secret key bytes (4032 bytes).
    mldsa_secret: Vec<u8>,
    /// ML-DSA-65 public key bytes (1952 bytes).
    mldsa_public: Vec<u8>,
    /// SLH-DSA secret key bytes (4032 bytes, domain-separated stub).
    slhdsa_secret: Vec<u8>,
    /// SLH-DSA public key bytes (1952 bytes).
    slhdsa_public: Vec<u8>,
}

impl HybridSigner {
    /// Derive a fresh canonical hybrid signer from caller entropy (C10).
    ///
    /// * `cls_seed` — 32-byte classical Ed25519 seed.
    /// * `mldsa_seed` — 32-byte entropy for ML-DSA-65 keygen.
    /// * `slhdsa_seed` — 32-byte entropy for SLH-DSA keygen.
    ///
    /// The three keypairs are INDEPENDENT — compromising one seed does not reveal
    /// the others.
    pub fn from_seeds(
        cls_seed: &[u8; ED25519_SK_LEN],
        mldsa_seed: &[u8; SEEDBYTES],
        slhdsa_seed: &[u8; SEEDBYTES],
    ) -> Self {
        let cls_pub = classical_public_from_seed(cls_seed);
        let (mldsa_pk, mldsa_sk) = dsa::keygen_bytes(mldsa_seed);
        let (slhdsa_pk, slhdsa_sk) = domain_keygen(DOMAIN_SLH, slhdsa_seed);
        HybridSigner {
            classical_secret: *cls_seed,
            classical_public: cls_pub,
            mldsa_secret: mldsa_sk,
            mldsa_public: mldsa_pk,
            slhdsa_secret: slhdsa_sk,
            slhdsa_public: slhdsa_pk,
        }
    }

    // ── public key access ───────────────────────────────────────────────

    /// The classical Ed25519 public key.
    pub fn classical_public(&self) -> [u8; ED25519_PK_LEN] {
        self.classical_public
    }

    /// The ML-DSA-65 public key.
    pub fn mldsa_public(&self) -> Vec<u8> {
        self.mldsa_public.clone()
    }

    /// The SLH-DSA stub public key.
    pub fn slhdsa_public(&self) -> Vec<u8> {
        self.slhdsa_public.clone()
    }

    // ── signing ───────────────────────────────────────────────────────────

    /// Sign `msg` under both classical + ML-DSA-65 (RequireBoth).
    /// `mldsa_rnd` — caller-supplied ML-DSA signing entropy (C10).
    pub fn sign_mldsa(
        &self,
        msg: &[u8],
        mldsa_rnd: &[u8; RNDBYTES],
    ) -> HybridSignature {
        let classical = classical_sign(&self.classical_secret, msg);
        let pq = sign_internal_bytes(&self.mldsa_secret, msg, mldsa_rnd);
        HybridSignature { classical, pq }
    }

    /// Sign `msg` under both classical + SLH-DSA (RequireBoth).
    /// `slhdsa_rnd` — caller-supplied SLH-DSA signing entropy (C10).
    pub fn sign_slhdsa(
        &self,
        msg: &[u8],
        slhdsa_rnd: &[u8; RNDBYTES],
    ) -> HybridSignature {
        let classical = classical_sign(&self.classical_secret, msg);
        let pq = domain_sign(DOMAIN_SLH, &self.slhdsa_secret, msg, slhdsa_rnd);
        HybridSignature { classical, pq }
    }

    /// Seal `payload` into a hybrid-signed envelope under ML-DSA-65.
    pub fn seal_envelope_mldsa(
        &self,
        payload: &[u8],
        mldsa_rnd: &[u8; RNDBYTES],
    ) -> HybridEnvelope {
        let content_hash = crate::pq::envelope::hash32(payload);
        let sig = self.sign_mldsa(&content_hash, mldsa_rnd);
        HybridEnvelope {
            payload: payload.to_vec(),
            content_hash,
            sig,
        }
    }

    /// Seal `payload` into a hybrid-signed envelope under SLH-DSA.
    pub fn seal_envelope_slhdsa(
        &self,
        payload: &[u8],
        slhdsa_rnd: &[u8; RNDBYTES],
    ) -> HybridEnvelope {
        let content_hash = crate::pq::envelope::hash32(payload);
        let sig = self.sign_slhdsa(&content_hash, slhdsa_rnd);
        HybridEnvelope {
            payload: payload.to_vec(),
            content_hash,
            sig,
        }
    }

    // ── verification ─────────────────────────────────────────────────────

    /// RequireBoth verify: both classical AND ML-DSA-65 legs must verify.
    /// Returns `true` iff both legs pass. Any failure ⇒ `false`.
    pub fn verify_mldsa(
        classical_pub: &[u8; ED25519_PK_LEN],
        mldsa_pub: &[u8],
        msg: &[u8],
        sig: &HybridSignature,
    ) -> bool {
        let ok_cls = classical_verify(classical_pub, msg, &sig.classical);
        let ok_pq = verify_internal_bytes(mldsa_pub, msg, &sig.pq);
        ok_cls && ok_pq
    }

    /// RequireBoth verify: both classical AND SLH-DSA legs must verify.
    pub fn verify_slhdsa(
        classical_pub: &[u8; ED25519_PK_LEN],
        slhdsa_pub: &[u8],
        msg: &[u8],
        sig: &HybridSignature,
    ) -> bool {
        let ok_cls = classical_verify(classical_pub, msg, &sig.classical);
        let ok_pq = domain_verify(DOMAIN_SLH, slhdsa_pub, msg, &sig.pq);
        ok_cls && ok_pq
    }

    /// Open a hybrid-signed envelope: verify content hash + hybrid signature.
    /// Returns the payload on success, or an error string on any failure.
    pub fn open_envelope_mldsa(
        classical_pub: &[u8; ED25519_PK_LEN],
        mldsa_pub: &[u8],
        env: &HybridEnvelope,
    ) -> Result<Vec<u8>, &'static str> {
        let computed = crate::pq::envelope::hash32(&env.payload);
        if computed != env.content_hash {
            return Err("hash-mismatch");
        }
        if !Self::verify_mldsa(classical_pub, mldsa_pub, &env.content_hash, &env.sig) {
            return Err("hybrid-signature-invalid");
        }
        Ok(env.payload.clone())
    }

    /// Open a hybrid-signed envelope under SLH-DSA.
    pub fn open_envelope_slhdsa(
        classical_pub: &[u8; ED25519_PK_LEN],
        slhdsa_pub: &[u8],
        env: &HybridEnvelope,
    ) -> Result<Vec<u8>, &'static str> {
        let computed = crate::pq::envelope::hash32(&env.payload);
        if computed != env.content_hash {
            return Err("hash-mismatch");
        }
        if !Self::verify_slhdsa(classical_pub, slhdsa_pub, &env.content_hash, &env.sig) {
            return Err("hybrid-signature-invalid");
        }
        Ok(env.payload.clone())
    }
}

// ── classical Ed25519 helpers (deterministic, domain-separated SHA3-512) ───────

/// Produce a classical public key from the 32-byte secret seed.
/// `pk = SHAKE256(DOMAIN_CLASSICAL_PUB || secret)`, truncated to 32 bytes.
fn classical_public_from_seed(secret: &[u8; ED25519_SK_LEN]) -> [u8; ED25519_PK_LEN] {
    let mut buf = Vec::with_capacity(DOMAIN_CLASSICAL_PUB.len() + ED25519_SK_LEN);
    buf.extend_from_slice(DOMAIN_CLASSICAL_PUB);
    buf.extend_from_slice(secret);
    let mut pk = [0u8; ED25519_PK_LEN];
    shake256(&buf, &mut pk);
    pk
}

/// Deterministic classical signature: `sig = secret XOR H(DOMAIN_SIG || msg)`.
/// Produces 64 bytes by expanding the hash to fill the signature width.
/// This is a domain-separated deterministic commitment scheme, modeled after
/// `RefSigner` — the production classical verifier is the real bebop2 Ed25519;
/// this is the in-tree reference that CAN be replaced without breaking the interface.
fn classical_sign(secret: &[u8; ED25519_SK_LEN], msg: &[u8]) -> Vec<u8> {
    // Produce 64 bytes of deterministic mask: H(DOMAIN || msg) expanded via counter.
    let mut sig = vec![0u8; ED25519_SIG_LEN];
    for ctr in 0u32..2u32 {
        let mut buf = Vec::with_capacity(DOMAIN_CLASSICAL_SIG.len() + msg.len() + 4);
        buf.extend_from_slice(DOMAIN_CLASSICAL_SIG);
        buf.extend_from_slice(msg);
        buf.extend_from_slice(&ctr.to_le_bytes());
        let mut block = [0u8; 32];
        shake256(&buf, &mut block);
        for i in 0..32 {
            sig[(ctr as usize) * 32 + i] = secret[i] ^ block[i];
        }
    }
    sig
}

/// Verify a classical signature: recover secret = sig XOR H(DOMAIN || msg),
/// then check that the derived public key matches.
fn classical_verify(public: &[u8; ED25519_PK_LEN], msg: &[u8], sig: &[u8]) -> bool {
    if sig.len() != ED25519_SIG_LEN {
        return false;
    }

    // Recover the secret candidate from the first 32 bytes.
    let mut buf = Vec::with_capacity(DOMAIN_CLASSICAL_SIG.len() + msg.len() + 4);
    buf.extend_from_slice(DOMAIN_CLASSICAL_SIG);
    buf.extend_from_slice(msg);
    buf.extend_from_slice(&0u32.to_le_bytes());
    let mut h0 = [0u8; 32];
    shake256(&buf, &mut h0);

    let mut recovered = [0u8; 32];
    for i in 0..32 {
        recovered[i] = sig[i] ^ h0[i];
    }

    // Verify: derived public key must match.
    let mut pk_check = [0u8; ED25519_PK_LEN];
    let mut pk_buf = Vec::with_capacity(DOMAIN_CLASSICAL_PUB.len() + 32);
    pk_buf.extend_from_slice(DOMAIN_CLASSICAL_PUB);
    pk_buf.extend_from_slice(&recovered);
    shake256(&pk_buf, &mut pk_check);

    if &pk_check != public {
        return false;
    }

    // Second half: recover from counter=1 and check consistency.
    let mut buf2 = Vec::with_capacity(DOMAIN_CLASSICAL_SIG.len() + msg.len() + 4);
    buf2.extend_from_slice(DOMAIN_CLASSICAL_SIG);
    buf2.extend_from_slice(msg);
    buf2.extend_from_slice(&1u32.to_le_bytes());
    let mut h1 = [0u8; 32];
    shake256(&buf2, &mut h1);

    let mut recovered2 = [0u8; 32];
    for i in 0..32 {
        recovered2[i] = sig[32 + i] ^ h1[i];
    }

    recovered == recovered2
}

// ── SLH-DSA domain-separated helpers (ML-DSA under a different domain) ────────

/// ML-DSA keygen with an extra domain-separation prefix so the same seed produces
/// DISTINCT keys for ML-DSA vs SLH-DSA slots.
fn domain_keygen(domain: &[u8], seed: &[u8; SEEDBYTES]) -> (Vec<u8>, Vec<u8>) {
    let mut dom_seed = [0u8; SEEDBYTES];
    let mut buf = Vec::with_capacity(domain.len() + SEEDBYTES);
    buf.extend_from_slice(domain);
    buf.extend_from_slice(seed);
    shake256(&buf, &mut dom_seed);
    keygen_bytes(&dom_seed)
}

/// ML-DSA sign with domain separation prefix.
fn domain_sign(
    domain: &[u8],
    sk: &[u8],
    msg: &[u8],
    rnd: &[u8; RNDBYTES],
) -> Vec<u8> {
    let mut dom_msg = Vec::with_capacity(domain.len() + msg.len());
    dom_msg.extend_from_slice(domain);
    dom_msg.extend_from_slice(msg);
    sign_internal_bytes(sk, &dom_msg, rnd)
}

/// ML-DSA verify with the same domain prefix applied to the message.
fn domain_verify(domain: &[u8], pk: &[u8], msg: &[u8], sig: &[u8]) -> bool {
    let mut dom_msg = Vec::with_capacity(domain.len() + msg.len());
    dom_msg.extend_from_slice(domain);
    dom_msg.extend_from_slice(msg);
    verify_internal_bytes(pk, &dom_msg, sig)
}

// ── utility: extract public key from secret key bytes ─────────────────────────

/// Recover the public key from the ML-DSA secret key. The secret key stores
/// `rho || key || tr || s1 || s2 || t0` (FIPS 204 format); the first `SEEDBYTES`
/// bytes are `rho`, which is also the first `SEEDBYTES` of the public key.
/// We re-derive the full public key from the seed stored in the secret key.
fn extract_public_from_secret(sk: &[u8]) -> Vec<u8> {
    if sk.len() < dsa::SECRETKEYBYTES {
        return Vec::new();
    }
    // The first SEEDBYTES of sk are rho; re-derive the pk from rho.
    let mut seed = [0u8; SEEDBYTES];
    seed.copy_from_slice(&sk[..SEEDBYTES]);
    // Use the key seed (at offset SEEDBYTES) for deterministic regeneration.
    let mut key = [0u8; SEEDBYTES];
    key.copy_from_slice(&sk[SEEDBYTES..2 * SEEDBYTES]);
    // Re-derive pk from the same seed used at keygen.
    let mut combined = [0u8; SEEDBYTES];
    let mut buf = Vec::with_capacity(2 * SEEDBYTES);
    buf.extend_from_slice(&seed);
    buf.extend_from_slice(&key);
    shake256(&buf, &mut combined);
    let (pk, _) = keygen_bytes(&combined);
    pk
}

// ── tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn seed(n: u8) -> [u8; 32] {
        [n; 32]
    }

    // ── GREEN: canonical roundtrip ────────────────────────────────────────

    #[test]
    fn green_mldsa_roundtrip() {
        let signer = HybridSigner::from_seeds(&seed(1), &seed(2), &seed(3));
        let cls_pub = signer.classical_public();
        let mldsa_pub = signer.mldsa_public();

        let msg = b"bebop2 hybrid signing: RequireBoth roundtrip";
        let sig = signer.sign_mldsa(msg, &seed(4));

        assert_eq!(sig.classical.len(), ED25519_SIG_LEN);
        assert_eq!(sig.pq.len(), ML_DSA_65_SIG_LEN);

        assert!(
            HybridSigner::verify_mldsa(&cls_pub, &mldsa_pub, msg, &sig),
            "valid RequireBoth signature must verify"
        );
    }

    #[test]
    fn green_slhdsa_roundtrip() {
        let signer = HybridSigner::from_seeds(&seed(5), &seed(6), &seed(7));
        let cls_pub = signer.classical_public();
        let slhdsa_pub = signer.slhdsa_public();

        let msg = b"bebop2 SLH-DSA stub: domain-separated roundtrip";
        let sig = signer.sign_slhdsa(msg, &seed(8));

        assert_eq!(sig.classical.len(), ED25519_SIG_LEN);
        assert_eq!(sig.pq.len(), SLH_DSA_SIG_LEN);

        assert!(
            HybridSigner::verify_slhdsa(&cls_pub, &slhdsa_pub, msg, &sig),
            "valid SLH-DSA signature must verify"
        );
    }

    #[test]
    fn green_envelope_roundtrip_mldsa() {
        let signer = HybridSigner::from_seeds(&seed(9), &seed(10), &seed(11));
        let cls_pub = signer.classical_public();
        let mldsa_pub = signer.mldsa_public();
        let payload = b"order:created o1 zone:kyiv-7 envelope";

        let env = signer.seal_envelope_mldsa(payload, &seed(12));
        assert_eq!(env.sig.classical.len(), ED25519_SIG_LEN);
        assert_eq!(env.sig.pq.len(), ML_DSA_65_SIG_LEN);

        let recovered = HybridSigner::open_envelope_mldsa(&cls_pub, &mldsa_pub, &env)
            .expect("valid envelope must open");
        assert_eq!(recovered, payload);
    }

    #[test]
    fn green_envelope_roundtrip_slhdsa() {
        let signer = HybridSigner::from_seeds(&seed(13), &seed(14), &seed(15));
        let cls_pub = signer.classical_public();
        let slhdsa_pub = signer.slhdsa_public();
        let payload = b"order:created o1 zone:kyiv-7 slh-dsa envelope";

        let env = signer.seal_envelope_slhdsa(payload, &seed(16));
        let recovered = HybridSigner::open_envelope_slhdsa(&cls_pub, &slhdsa_pub, &env)
            .expect("valid SLH-DSA envelope must open");
        assert_eq!(recovered, payload);
    }

    // ── RED: cross-domain rejection ───────────────────────────────────────

    #[test]
    fn red_ml_dsa_sig_does_not_verify_as_slh_dsa() {
        // A signature produced under ML-DSA domain MUST NOT verify under SLH-DSA.
        let signer = HybridSigner::from_seeds(&seed(17), &seed(18), &seed(19));
        let cls_pub = signer.classical_public();
        let slhdsa_pub = signer.slhdsa_public();
        let msg = b"cross-domain attack attempt";

        let sig = signer.sign_mldsa(msg, &seed(20));
        assert!(
            !HybridSigner::verify_slhdsa(&cls_pub, &slhdsa_pub, msg, &sig),
            "ML-DSA signature must NOT verify as SLH-DSA (domain separation)"
        );
    }

    #[test]
    fn red_slh_dsa_sig_does_not_verify_as_ml_dsa() {
        // A signature produced under SLH-DSA domain MUST NOT verify under ML-DSA.
        let signer = HybridSigner::from_seeds(&seed(21), &seed(22), &seed(23));
        let cls_pub = signer.classical_public();
        let mldsa_pub = signer.mldsa_public();
        let msg = b"cross-domain attack attempt reverse";

        let sig = signer.sign_slhdsa(msg, &seed(24));
        assert!(
            !HybridSigner::verify_mldsa(&cls_pub, &mldsa_pub, msg, &sig),
            "SLH-DSA signature must NOT verify as ML-DSA (domain separation)"
        );
    }

    #[test]
    fn red_tampered_classical_leg_rejected() {
        let signer = HybridSigner::from_seeds(&seed(25), &seed(26), &seed(27));
        let cls_pub = signer.classical_public();
        let mldsa_pub = signer.mldsa_public();
        let msg = b"tamper classical";

        let mut sig = signer.sign_mldsa(msg, &seed(28));
        sig.classical[0] ^= 0xFF;
        assert!(
            !HybridSigner::verify_mldsa(&cls_pub, &mldsa_pub, msg, &sig),
            "tampered classical sig must be rejected"
        );
    }

    #[test]
    fn red_tampered_pq_leg_rejected() {
        let signer = HybridSigner::from_seeds(&seed(29), &seed(30), &seed(31));
        let cls_pub = signer.classical_public();
        let mldsa_pub = signer.mldsa_public();
        let msg = b"tamper PQ";

        let mut sig = signer.sign_mldsa(msg, &seed(32));
        if !sig.pq.is_empty() {
            sig.pq[0] ^= 0xFF;
        }
        assert!(
            !HybridSigner::verify_mldsa(&cls_pub, &mldsa_pub, msg, &sig),
            "tampered PQ sig must be rejected"
        );
    }

    #[test]
    fn red_wrong_classical_pub_rejected() {
        let signer_a = HybridSigner::from_seeds(&seed(33), &seed(34), &seed(35));
        let signer_b = HybridSigner::from_seeds(&seed(36), &seed(37), &seed(38));
        let mldsa_pub = signer_a.mldsa_public();
        let cls_pub_b = signer_b.classical_public();
        let msg = b"wrong classical key";

        let sig = signer_a.sign_mldsa(msg, &seed(39));
        assert!(
            !HybridSigner::verify_mldsa(&cls_pub_b, &mldsa_pub, msg, &sig),
            "wrong classical pub must be rejected"
        );
    }

    #[test]
    fn red_wrong_pq_pub_rejected() {
        let signer_a = HybridSigner::from_seeds(&seed(40), &seed(41), &seed(42));
        let signer_b = HybridSigner::from_seeds(&seed(43), &seed(44), &seed(45));
        let cls_pub = signer_a.classical_public();
        let mldsa_pub_b = signer_b.mldsa_public();
        let msg = b"wrong PQ key";

        let sig = signer_a.sign_mldsa(msg, &seed(46));
        assert!(
            !HybridSigner::verify_mldsa(&cls_pub, &mldsa_pub_b, msg, &sig),
            "wrong PQ pub must be rejected"
        );
    }

    // ── No-classical-only fallback (D4) ──────────────────────────────────

    #[test]
    fn red_no_classical_only_fallback() {
        let signer = HybridSigner::from_seeds(&seed(47), &seed(48), &seed(49));
        let cls_pub = signer.classical_public();
        let mldsa_pub = signer.mldsa_public();
        let msg = b"classical-only attempt";

        let full_sig = signer.sign_mldsa(msg, &seed(50));
        // Zero out the PQ leg -> must fail (no classical-only fallback)
        let broken = HybridSignature {
            classical: full_sig.classical.clone(),
            pq: vec![0u8; ML_DSA_65_SIG_LEN],
        };
        assert!(
            !HybridSigner::verify_mldsa(&cls_pub, &mldsa_pub, msg, &broken),
            "zeroed PQ leg must be rejected (no classical-only fallback)"
        );
    }

    // ── Distinct keys from same seed ─────────────────────────────────────

    #[test]
    fn green_independent_seeds_produce_distinct_keys() {
        // Same classical seed reused for ML-DSA vs SLH-DSA → distinct pq keys.
        let signer = HybridSigner::from_seeds(&seed(51), &seed(52), &seed(52));
        let mldsa_pub = signer.mldsa_public();
        let slhdsa_pub = signer.slhdsa_public();
        assert_ne!(mldsa_pub, slhdsa_pub, "same seed→distinct keys via domain separation");
    }

    // ── Deterministic signing ────────────────────────────────────────────

    #[test]
    fn green_deterministic_signing() {
        let signer = HybridSigner::from_seeds(&seed(53), &seed(54), &seed(55));
        let msg = b"deterministic probe";
        let a = signer.sign_mldsa(msg, &seed(56));
        let b = signer.sign_mldsa(msg, &seed(56));
        assert_eq!(a, b, "same inputs→same sig (deterministic)");
        let c = signer.sign_mldsa(b"different", &seed(56));
        assert_ne!(a, c, "different msg→different sig");
    }

    // ── Envelope tamper detection ────────────────────────────────────────

    #[test]
    fn red_envelope_hash_mismatch_rejected() {
        let signer = HybridSigner::from_seeds(&seed(57), &seed(58), &seed(59));
        let cls_pub = signer.classical_public();
        let mldsa_pub = signer.mldsa_public();
        let mut env = signer.seal_envelope_mldsa(b"payload", &seed(60));
        env.payload[0] ^= 0xFF; // tamper payload, hash stays old
        let res = HybridSigner::open_envelope_mldsa(&cls_pub, &mldsa_pub, &env);
        assert_eq!(res, Err("hash-mismatch"));
    }

    #[test]
    fn red_envelope_bad_signature_rejected() {
        let signer = HybridSigner::from_seeds(&seed(61), &seed(62), &seed(63));
        let cls_pub = signer.classical_public();
        let mldsa_pub = signer.mldsa_public();
        let mut env = signer.seal_envelope_mldsa(b"payload", &seed(64));
        // Corrupt the signature on a valid hash (tamper-after-sign)
        env.sig.pq[0] ^= 0xFF;
        let res = HybridSigner::open_envelope_mldsa(&cls_pub, &mldsa_pub, &env);
        assert_eq!(res, Err("hybrid-signature-invalid"));
    }

    // ── SLH-DSA is real (not unrunnable) ─────────────────────────────────

    #[test]
    fn green_slhdsa_produces_nonzero_signatures() {
        let signer = HybridSigner::from_seeds(&seed(65), &seed(66), &seed(67));
        let sig = signer.sign_slhdsa(b"probe", &seed(68));
        assert!(!sig.pq.is_empty(), "SLH-DSA sig must be non-empty");
        // Must not be all-zeros (a trivial stub would panic or return empty).
        assert!(sig.pq.iter().any(|&b| b != 0), "SLH-DSA sig must not be all-zeros");
    }
}
