//! MCP (Model Context Protocol) adapter (P5): wrap an MCP JSON-RPC message in the PQ envelope.
//!
//! Thin shim only. It does NOT open an MCP transport or validate the full JSON-RPC 2.0 schema
//! — it maps a typed [`McpMsg`] onto the PQ envelope so outbound MCP calls/results are PQ-signed
//! (ML-DSA-65) and verifiable against the sender's key on receipt.

use super::AdapterError;
use dowiz_kernel::pq::envelope::{open, seal, SignedEnvelope, ENTROPY_LEN};

/// A minimal MCP JSON-RPC message: method + params + id.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct McpMsg {
    /// JSON-RPC method, e.g. "tools/call".
    pub method: String,
    /// Serialized params blob (opaque JSON string).
    pub params: String,
    /// Request id.
    pub id: u64,
}

/// Serialize the MCP message and PQ-seal it under the sender's ML-DSA-65 secret key.
pub fn to_envelope(msg: &McpMsg, sk: &[u8], rnd: &[u8; ENTROPY_LEN]) -> SignedEnvelope {
    let bytes = serde_json::to_vec(msg).expect("mcp msg serializes");
    seal(&bytes, sk, rnd)
}

/// Verify the envelope against `sender_pk`, then decode the MCP message. A message signed by
/// any other key (or tampered) fails here (RED gate).
pub fn from_envelope(env: &SignedEnvelope, sender_pk: &[u8]) -> Result<McpMsg, AdapterError> {
    let payload = open(env, sender_pk).map_err(|_| AdapterError::Envelope)?;
    serde_json::from_slice(&payload).map_err(|_| AdapterError::Decode)
}

#[cfg(test)]
mod tests {
    use super::*;
    use dowiz_kernel::pq::envelope::new_identity;

    const SEED_A: [u8; 32] = [55u8; 32];
    const SEED_B: [u8; 32] = [66u8; 32];
    const RND: [u8; ENTROPY_LEN] = [0u8; ENTROPY_LEN];

    fn sample() -> McpMsg {
        McpMsg {
            method: "tools/call".into(),
            params: r#"{"name":"dispatch"}"#.into(),
            id: 42,
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
