//! PQ-signed envelope — the L1 identity/integrity seam for the protocol.
//!
//! Transport-agnostic (MANIFESTO C4): an envelope is just
//! `payload ‖ content_hash(SHAKE256) ‖ ML-DSA-65 sig`. Any layer above (DTN
//! bundle, NOSTR event, MCP call) wraps its bytes in this. The transport
//! carries opaque bytes; PQ holds at the protocol layer regardless of underlay
//! (DECISIONS D3 red-line #3).
//!
//! RNG-free hot path (C10): the caller supplies `rnd` entropy for signing.
//! ponytail: no seq/replay here — that is the transport's job (DTN lifetime +
//! EID dedupe, RFC 9171). Envelope proves *integrity + authorship* only.

use crate::pq::dsa::{keygen_bytes, sign_internal_bytes, verify_internal_bytes};
use crate::pq::keccak::shake256;

/// SHAKE256 truncated to 256 bits (the envelope content hash).
pub fn hash32(input: &[u8]) -> [u8; HASH_LEN] {
    let mut out = [0u8; HASH_LEN];
    shake256(input, &mut out);
    out
}

/// ML-DSA-65 signature byte length (FIPS 204 mode 3).
pub const SIG_LEN: usize = 3309;
/// Public key length (FIPS 204 mode 3).
pub const PK_LEN: usize = 1952;
/// Secret key length (FIPS 204 mode 3).
pub const SK_LEN: usize = 4032;
/// Content hash length (SHAKE256, 256-bit).
pub const HASH_LEN: usize = 32;
/// Entropy byte length for keygen / signing (caller-supplied, C10).
pub const ENTROPY_LEN: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvelopeError {
    /// Stored content hash ≠ hash(payload). Payload tampered or wrong envelope.
    HashMismatch,
    /// ML-DSA verification rejected (wrong key, tampered sig, or tampered payload).
    BadSignature,
}

/// A PQ-signed, content-addressed envelope.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SignedEnvelope {
    pub payload: Vec<u8>,
    pub content_hash: [u8; HASH_LEN],
    pub sig: Vec<u8>,
}

/// Seal `payload` with the signer's secret key. `rnd` is caller-supplied
/// signing entropy (C10); never reuse it across seals.
pub fn seal(payload: &[u8], sk: &[u8], rnd: &[u8; ENTROPY_LEN]) -> SignedEnvelope {
    let content_hash = hash32(payload);
    let sig = sign_internal_bytes(sk, payload, rnd);
    SignedEnvelope {
        payload: payload.to_vec(),
        content_hash,
        sig,
    }
}

/// Verify + unpack. Returns the payload only if (a) content_hash matches and
/// (b) ML-DSA-65 verifies under `pk`. Either failure is rejected (RED gate).
pub fn open(env: &SignedEnvelope, pk: &[u8]) -> Result<Vec<u8>, EnvelopeError> {
    let computed = hash32(&env.payload);
    if computed != env.content_hash {
        return Err(EnvelopeError::HashMismatch);
    }
    if verify_internal_bytes(pk, &env.payload, &env.sig) {
        Ok(env.payload.clone())
    } else {
        Err(EnvelopeError::BadSignature)
    }
}

/// Derive a self-certifying node id from the PQ public key (MANIFESTO §2):
/// `id = H(pq_pub)`, no directory/phone-home.
pub fn node_id(pk: &[u8]) -> [u8; HASH_LEN] {
    hash32(pk)
}

/// Generate a fresh PQ identity (pk, sk) from caller entropy.
pub fn new_identity(seed: &[u8; ENTROPY_LEN]) -> (Vec<u8>, Vec<u8>) {
    keygen_bytes(seed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed(n: u8) -> [u8; ENTROPY_LEN] {
        [n; ENTROPY_LEN]
    }

    #[test]
    fn green_valid_roundtrip() {
        let (pk, sk) = new_identity(&seed(1));
        assert_eq!(pk.len(), PK_LEN);
        assert_eq!(sk.len(), SK_LEN);
        let payload = b"order:created o1 zone:kyiv-7";
        let env = seal(payload, &sk, &seed(2));
        assert_eq!(env.sig.len(), SIG_LEN);
        let out = open(&env, &pk).expect("valid envelope opens");
        assert_eq!(out, payload);
    }

    #[test]
    fn red_tampered_payload_rejected() {
        let (pk, sk) = new_identity(&seed(3));
        let mut env = seal(b"pay 100", &sk, &seed(4));
        env.payload[3] = b'9'; // "pay 900" — tamper
        assert_eq!(open(&env, &pk), Err(EnvelopeError::HashMismatch));
    }

    #[test]
    fn red_wrong_key_rejected() {
        let (_pk, sk) = new_identity(&seed(5));
        let (pk2, _sk2) = new_identity(&seed(6));
        let env = seal(b"hello", &sk, &seed(7));
        assert_eq!(open(&env, &pk2), Err(EnvelopeError::BadSignature));
    }

    #[test]
    fn red_tampered_sig_rejected() {
        let (pk, sk) = new_identity(&seed(8));
        let mut env = seal(b"data", &sk, &seed(9));
        if !env.sig.is_empty() {
            env.sig[0] ^= 0xff;
        }
        assert_eq!(open(&env, &pk), Err(EnvelopeError::BadSignature));
    }

    #[test]
    fn node_id_is_content_address() {
        let (pk, _sk) = new_identity(&seed(10));
        let id = node_id(&pk);
        let (pk2, _sk2) = new_identity(&seed(11));
        assert_ne!(id, node_id(&pk2), "distinct keys → distinct ids");
    }
}
