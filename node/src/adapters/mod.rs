//! Protocol adapter shims (P5): bridge external messaging protocols onto the PQ envelope.
//!
//! Each adapter is a thin, dependency-free shim. It defines a typed `Msg` struct for its
//! protocol, and two operations:
//!   - `to_envelope(msg, sk, rnd) -> SignedEnvelope`  — serialize + `envelope::seal`.
//!   - `from_envelope(env, sender_pk) -> Result<Msg>`  — `envelope::open` + deserialize.
//!
//! No real network calls happen here. Every outbound message is PQ-signed (ML-DSA-65) BEFORE
//! it could ever leave the node, and every inbound message is verified against the *sender's*
//! public key. A message signed by a different key fails `from_envelope` (RED gate). A
//! seal→open roundtrip recovers the exact message (GREEN gate).

pub mod activitypub;
pub mod mcp;
pub mod nostr;

/// Adapter-level error: either the PQ envelope failed to verify, or the verified payload
/// did not deserialize into the expected typed message.
#[derive(Debug, PartialEq, Eq)]
pub enum AdapterError {
    /// The PQ envelope did not open under the given sender public key (bad sig / tampered).
    Envelope,
    /// The envelope opened but the payload was not a valid serialized message.
    Decode,
}
