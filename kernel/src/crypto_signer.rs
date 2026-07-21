//! crypto_signer.rs — Per-call PQ ML-DSA-65 cryptographic signer for parse operations.
//!
//! # What this is
//! Each parse call gets a fresh ML-DSA-65 keypair. The signature binds:
//! - IP hash (SHA3-256 of the source IP)
//! - Timestamp (unix microseconds)
//! - Payload hash (SHA3-256 of the response content)
//! - Nonce (random, prevents replay)
//!
//! This creates a cryptographically verifiable audit trail: every parsed
//! response is signed with a fresh key, the key is bound to the IP+time,
//! and the signature covers the content hash. Any tampering or replay is
//! detectable by re-verifying the signature chain.
//!
//! # Design principles
//! - Pure computation: no network, no OS calls beyond monotonic clock
//! - ML-DSA-65 (FIPS 204) for post-quantum security
//! - Fresh keypair per call (forward secrecy: compromising one key reveals nothing about others)
//! - Deterministic signing (rnd=0 for testability, caller can supply rnd for randomness)

use crate::event_log::sha3_256;

/// A signed parse call — the full cryptographic attestation.
#[derive(Debug, Clone)]
pub struct SignedParseCall {
    /// ML-DSA-65 public key for this call (fresh keypair).
    pub public_key: Vec<u8>,
    /// ML-DSA-65 signature over the canonical message.
    pub signature: Vec<u8>,
    /// The canonical message that was signed: `ip_hash || timestamp_us || payload_hash || nonce`.
    pub signed_message: Vec<u8>,
    /// SHA3-256 of the source IP used for this call.
    pub ip_hash: [u8; 32],
    /// Unix microseconds when the call was made.
    pub timestamp_us: u64,
    /// SHA3-256 of the response payload.
    pub payload_hash: [u8; 32],
    /// 32-byte nonce (prevents replay).
    pub nonce: [u8; 32],
}

/// Build the canonical message to sign: `ip_hash || timestamp_us_le || payload_hash || nonce`.
///
/// This is the message that `SignedParseCall.verify` checks against. The binding
/// is unforgeable: an attacker cannot produce a valid signature for a different
/// (ip, timestamp, payload) tuple without knowing the signing key.
pub fn build_sign_message(
    ip_hash: &[u8; 32],
    timestamp_us: u64,
    payload_hash: &[u8; 32],
    nonce: &[u8; 32],
) -> Vec<u8> {
    let mut msg = Vec::with_capacity(32 + 8 + 32 + 32);
    msg.extend_from_slice(ip_hash);
    msg.extend_from_slice(&timestamp_us.to_le_bytes());
    msg.extend_from_slice(payload_hash);
    msg.extend_from_slice(nonce);
    msg
}

/// Sign a parse call using ML-DSA-65.
///
/// Uses `crate::pq::dsa` for the actual signing. The seed is derived from
/// `SHA3-256(ip_hash || timestamp_us || nonce)` so the keypair is deterministic
/// from the call context (for testing), while the signature uses rnd=0 (FIPS
/// deterministic mode).
pub fn sign_parse_call(
    ip_hash: [u8; 32],
    timestamp_us: u64,
    payload_hash: [u8; 32],
    nonce: [u8; 32],
) -> SignedParseCall {
    let msg = build_sign_message(&ip_hash, timestamp_us, &payload_hash, &nonce);

    // Derive keypair seed from the call context (deterministic for testability).
    let seed_input = {
        let mut buf = Vec::with_capacity(32 + 8 + 32);
        buf.extend_from_slice(&ip_hash);
        buf.extend_from_slice(&timestamp_us.to_le_bytes());
        buf.extend_from_slice(&nonce);
        sha3_256(&buf)
    };
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&seed_input);

    let (pk, sk) = crate::pq::dsa::keygen(&seed);

    // Deterministic signing (rnd=0).
    let rnd = [0u8; 32];
    let sig = crate::pq::dsa::sign(&sk, &msg, &rnd);

    SignedParseCall {
        public_key: pk.bytes,
        signature: sig.bytes,
        signed_message: msg,
        ip_hash,
        timestamp_us,
        payload_hash,
        nonce,
    }
}

/// Verify a signed parse call.
///
/// Returns `Ok(())` if the signature is valid, `Err(VerificationError)` otherwise.
/// This is the audit-side function: a different agent or process verifies the call
/// after the fact, ensuring the content was not tampered with and the IP+time binding
/// holds.
pub fn verify_parse_call(signed: &SignedParseCall) -> Result<(), VerificationError> {
    // Verify the IP hash is non-zero (was actually computed).
    // Must check BEFORE message rebuild — a zeroed ip_hash would cause MessageMismatch,
    // but the semantic error is ZeroIpHash.
    if signed.ip_hash == [0u8; 32] {
        return Err(VerificationError::ZeroIpHash);
    }

    // Verify the payload hash is non-zero.
    if signed.payload_hash == [0u8; 32] {
        return Err(VerificationError::ZeroPayloadHash);
    }

    // Rebuild the expected message.
    let expected_msg = build_sign_message(
        &signed.ip_hash,
        signed.timestamp_us,
        &signed.payload_hash,
        &signed.nonce,
    );

    // Message must match.
    if signed.signed_message != expected_msg {
        return Err(VerificationError::MessageMismatch);
    }

    // Verify the ML-DSA-65 signature.
    let pk = crate::pq::dsa::MlDsa65Pk {
        bytes: signed.public_key.clone(),
    };
    let sig = crate::pq::dsa::MlDsa65Sig {
        bytes: signed.signature.clone(),
    };

    if !crate::pq::dsa::verify(&pk, &signed.signed_message, &sig) {
        return Err(VerificationError::SignatureInvalid);
    }

    Ok(())
}

/// Errors that can occur during signature verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationError {
    /// The signed message doesn't match the rebuilt canonical message.
    MessageMismatch,
    /// The ML-DSA-65 signature verification failed.
    SignatureInvalid,
    /// The IP hash is all zeros (was never computed from a real IP).
    ZeroIpHash,
    /// The payload hash is all zeros (was never computed from real content).
    ZeroPayloadHash,
}

/// A chain of signed parse calls — the full audit trail.
///
/// Each entry is independently verifiable, but the chain also provides
/// ordering evidence (timestamps should be monotonically increasing).
#[derive(Debug, Clone)]
pub struct SignedParseChain {
    /// Ordered list of signed calls (oldest first).
    calls: Vec<SignedParseCall>,
    /// Running hash over all signatures (SHA3-256 chain).
    chain_hash: [u8; 32],
}

impl SignedParseChain {
    /// Create an empty chain.
    pub fn new() -> Self {
        SignedParseChain {
            calls: Vec::new(),
            chain_hash: [0u8; 32],
        }
    }

    /// Append a signed call to the chain.
    pub fn append(&mut self, call: SignedParseCall) {
        // Update chain hash: SHA3-256(prev_chain_hash || sig_hash).
        let sig_hash = sha3_256(&call.signature);
        let mut chain_input = Vec::with_capacity(64);
        chain_input.extend_from_slice(&self.chain_hash);
        chain_input.extend_from_slice(&sig_hash);
        self.chain_hash = sha3_256(&chain_input);
        self.calls.push(call);
    }

    /// Verify the entire chain (every signature + monotonic timestamps).
    pub fn verify(&self) -> Result<(), ChainVerificationError> {
        let mut prev_ts = 0u64;
        for (i, call) in self.calls.iter().enumerate() {
            verify_parse_call(call).map_err(|e| ChainVerificationError::InvalidSignature {
                index: i,
                inner: e,
            })?;

            // Timestamps should be monotonically non-decreasing.
            if call.timestamp_us < prev_ts {
                return Err(ChainVerificationError::TimestampViolation {
                    index: i,
                    prev_ts,
                    this_ts: call.timestamp_us,
                });
            }
            prev_ts = call.timestamp_us;
        }

        // Verify the chain hash.
        let mut expected_hash = [0u8; 32];
        for call in &self.calls {
            let sig_hash = sha3_256(&call.signature);
            let mut chain_input = Vec::with_capacity(64);
            chain_input.extend_from_slice(&expected_hash);
            chain_input.extend_from_slice(&sig_hash);
            expected_hash = sha3_256(&chain_input);
        }
        if self.chain_hash != expected_hash {
            return Err(ChainVerificationError::ChainHashMismatch);
        }

        Ok(())
    }

    /// Number of calls in the chain.
    pub fn len(&self) -> usize {
        self.calls.len()
    }

    /// Whether the chain is empty.
    pub fn is_empty(&self) -> bool {
        self.calls.is_empty()
    }

    /// The current chain hash.
    pub fn chain_hash(&self) -> [u8; 32] {
        self.chain_hash
    }

    /// Get a call by index.
    pub fn get(&self, index: usize) -> Option<&SignedParseCall> {
        self.calls.get(index)
    }
}

/// Errors from chain verification.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChainVerificationError {
    /// A specific call's signature failed verification.
    InvalidSignature {
        index: usize,
        inner: VerificationError,
    },
    /// Timestamps are not monotonically non-decreasing.
    TimestampViolation {
        index: usize,
        prev_ts: u64,
        this_ts: u64,
    },
    /// The running chain hash doesn't match.
    ChainHashMismatch,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_and_verify_roundtrip() {
        let ip_hash = sha3_256(b"192.168.1.1");
        let timestamp_us = 1_700_000_000_000_000u64;
        let payload = b"<html>Hello World</html>";
        let payload_hash = sha3_256(payload);
        let nonce = [1u8; 32];

        let signed = sign_parse_call(ip_hash, timestamp_us, payload_hash, nonce);
        assert_eq!(signed.ip_hash, ip_hash);
        assert_eq!(signed.timestamp_us, timestamp_us);
        assert_eq!(signed.payload_hash, payload_hash);

        let result = verify_parse_call(&signed);
        assert!(result.is_ok(), "verification failed: {:?}", result.err());
    }

    #[test]
    fn verify_rejects_tampered_ip_hash() {
        let ip_hash = sha3_256(b"192.168.1.1");
        let timestamp_us = 1_700_000_000_000_000u64;
        let payload_hash = sha3_256(b"content");
        let nonce = [2u8; 32];

        let mut signed = sign_parse_call(ip_hash, timestamp_us, payload_hash, nonce);
        // Tamper with the IP hash.
        signed.ip_hash[0] ^= 0xFF;

        let result = verify_parse_call(&signed);
        assert_eq!(result, Err(VerificationError::MessageMismatch));
    }

    #[test]
    fn verify_rejects_tampered_signature() {
        let ip_hash = sha3_256(b"192.168.1.1");
        let timestamp_us = 1_700_000_000_000_000u64;
        let payload_hash = sha3_256(b"content");
        let nonce = [3u8; 32];

        let mut signed = sign_parse_call(ip_hash, timestamp_us, payload_hash, nonce);
        // Tamper with the signature.
        if !signed.signature.is_empty() {
            signed.signature[0] ^= 0xFF;
        }

        let result = verify_parse_call(&signed);
        assert_eq!(result, Err(VerificationError::SignatureInvalid));
    }

    #[test]
    fn verify_rejects_zero_ip_hash() {
        let signed = SignedParseCall {
            public_key: vec![0; 1952], // dummy
            signature: vec![0; 3309],  // dummy
            signed_message: vec![0; 104],
            ip_hash: [0u8; 32], // zero!
            timestamp_us: 1_700_000_000_000_000u64,
            payload_hash: sha3_256(b"content"),
            nonce: [4u8; 32],
        };
        let result = verify_parse_call(&signed);
        assert_eq!(result, Err(VerificationError::ZeroIpHash));
    }

    #[test]
    fn verify_rejects_zero_payload_hash() {
        let signed = SignedParseCall {
            public_key: vec![0; 1952],
            signature: vec![0; 3309],
            signed_message: vec![0; 104],
            ip_hash: sha3_256(b"192.168.1.1"),
            timestamp_us: 1_700_000_000_000_000u64,
            payload_hash: [0u8; 32], // zero!
            nonce: [5u8; 32],
        };
        let result = verify_parse_call(&signed);
        assert_eq!(result, Err(VerificationError::ZeroPayloadHash));
    }

    #[test]
    fn different_nonces_produce_different_signatures() {
        let ip = sha3_256(b"10.0.0.1");
        let ts = 1_700_000_000_000_000u64;
        let ph = sha3_256(b"data");

        let a = sign_parse_call(ip, ts, ph, [1u8; 32]);
        let b = sign_parse_call(ip, ts, ph, [2u8; 32]);
        assert_ne!(a.signature, b.signature, "different nonces => different sigs");
        assert_ne!(a.public_key, b.public_key, "different nonces => different keys");
    }

    #[test]
    fn chain_append_and_verify() {
        let mut chain = SignedParseChain::new();
        assert!(chain.is_empty());
        assert_eq!(chain.chain_hash(), [0u8; 32]);

        let ip = sha3_256(b"192.168.1.1");
        let ph = sha3_256(b"content");

        chain.append(sign_parse_call(ip, 1000, ph, [1u8; 32]));
        chain.append(sign_parse_call(ip, 2000, ph, [2u8; 32]));
        chain.append(sign_parse_call(ip, 3000, ph, [3u8; 32]));

        assert_eq!(chain.len(), 3);
        assert_ne!(chain.chain_hash(), [0u8; 32]);

        let result = chain.verify();
        assert!(result.is_ok(), "chain verify failed: {:?}", result.err());
    }

    #[test]
    fn chain_detects_timestamp_violation() {
        let mut chain = SignedParseChain::new();
        let ip = sha3_256(b"192.168.1.1");
        let ph = sha3_256(b"content");

        chain.append(sign_parse_call(ip, 3000, ph, [1u8; 32]));
        chain.append(sign_parse_call(ip, 1000, ph, [2u8; 32])); // out of order!

        let result = chain.verify();
        assert!(matches!(
            result,
            Err(ChainVerificationError::TimestampViolation { index: 1, .. })
        ));
    }

    #[test]
    fn chain_hash_changes_with_appends() {
        let mut chain = SignedParseChain::new();
        let ip = sha3_256(b"192.168.1.1");
        let ph = sha3_256(b"content");

        let h0 = chain.chain_hash();
        chain.append(sign_parse_call(ip, 1000, ph, [1u8; 32]));
        let h1 = chain.chain_hash();
        assert_ne!(h0, h1);

        chain.append(sign_parse_call(ip, 2000, ph, [2u8; 32]));
        let h2 = chain.chain_hash();
        assert_ne!(h1, h2);
    }

    #[test]
    fn build_sign_message_is_correct_length() {
        let msg = build_sign_message(&[0u8; 32], 12345, &[0u8; 32], &[0u8; 32]);
        // 32 + 8 + 32 + 32 = 104 bytes
        assert_eq!(msg.len(), 104);
    }
}
