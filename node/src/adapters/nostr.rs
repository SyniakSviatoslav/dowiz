//! NOSTR adapter (P5): wrap a NOSTR-style note in the PQ envelope before any relay send.
//!
//! This is a thin shim. It does NOT talk to relays or implement NIP-01 wire format — it
//! only maps a typed [`NostrMsg`] onto the PQ envelope so that everything a node would
//! publish is PQ-signed (ML-DSA-65) and verifiable against the sender's key on receipt.

use super::AdapterError;
use dowiz_kernel::pq::envelope::{open, seal, SignedEnvelope, ENTROPY_LEN};

/// A minimal NOSTR event (NIP-01 shaped): kind + content + created_at.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct NostrMsg {
    /// Event kind (e.g. 1 = short text note).
    pub kind: u32,
    /// UTF-8 note content.
    pub content: String,
    /// Creation timestamp (seconds since epoch).
    pub created_at: u64,
}

/// Serialize the note and PQ-seal it under the sender's ML-DSA-65 secret key.
pub fn to_envelope(msg: &NostrMsg, sk: &[u8], rnd: &[u8; ENTROPY_LEN]) -> SignedEnvelope {
    let bytes = serde_json::to_vec(msg).expect("nostr msg serializes");
    seal(&bytes, sk, rnd)
}

/// Verify the envelope against `sender_pk`, then decode the note. A message signed by any
/// other key (or tampered) fails here (RED gate).
pub fn from_envelope(env: &SignedEnvelope, sender_pk: &[u8]) -> Result<NostrMsg, AdapterError> {
    let payload = open(env, sender_pk).map_err(|_| AdapterError::Envelope)?;
    serde_json::from_slice(&payload).map_err(|_| AdapterError::Decode)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dowiz_kernel::pq::envelope::new_identity;

    const SEED_A: [u8; 32] = [11u8; 32];
    const SEED_B: [u8; 32] = [22u8; 32];
    const RND: [u8; ENTROPY_LEN] = [0u8; ENTROPY_LEN];

    fn sample() -> NostrMsg {
        NostrMsg {
            kind: 1,
            content: "gm nostr".into(),
            created_at: 1000,
        }
    }

    #[test]
    fn green_roundtrip_recovers_msg() {
        let (pk, sk) = new_identity(&SEED_A);
        let msg = sample();
        let env = to_envelope(&msg, &sk, &RND);
        assert_eq!(from_envelope(&env, &pk).unwrap(), msg);
    }

    #[test]
    fn red_wrong_sender_key_rejected() {
        let (_pk_a, sk_a) = new_identity(&SEED_A);
        let (pk_b, _sk_b) = new_identity(&SEED_B);
        let env = to_envelope(&sample(), &sk_a, &RND);
        // Opened against B's key though signed by A → must fail.
        assert_eq!(from_envelope(&env, &pk_b), Err(AdapterError::Envelope));
    }

    #[test]
    fn red_tampered_payload_rejected() {
        let (pk, sk) = new_identity(&SEED_A);
        let mut env = to_envelope(&sample(), &sk, &RND);
        if !env.sig.is_empty() {
            env.sig[0] ^= 0xFF;
        }
        assert_eq!(from_envelope(&env, &pk), Err(AdapterError::Envelope));
    }
}
