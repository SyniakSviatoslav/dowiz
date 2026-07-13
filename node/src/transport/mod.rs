//! Transport layer: wire codecs and bearers for carrying [`crate::Bundle`] between
//! nodes.
//!
//! - S1: `bp7` — RFC 9171 BPv7 Bundle codec (dtn7/bp7-rs).
//! - S2+S3: `quic` — production bearer = RFC 9000 QUIC + RFC 9174 (TCPCLv4-style)
//!   under TLS 1.3 (rustls + ring; operator FIPS swap to aws-lc-rs per DECISIONS D9).
//!
//! The `Transport` trait keeps the node's store-and-forward semantics underlay-
//! agnostic (QUIC/TCPCLv4/SpaceWire all satisfy the same contract — D3).

pub mod bp7;
pub mod quic;

use crate::Bundle;

/// A transport encodes/decodes a [`Bundle`] to/from wire bytes.
///
/// The concrete RFC 9171 impl is [`bp7::Bp7Transport`]; the trait lets the
/// node's store-and-forward semantics stay underlay-agnostic.
pub trait Transport {
    /// Serialize a bundle to wire bytes (BPv7 CBOR for [`bp7::Bp7Transport`]).
    fn encode(&self, b: &Bundle) -> Vec<u8>;

    /// Parse wire bytes back into a [`Bundle`]. Returns `Err` on any malformed,
    /// truncated, or non-conforming input (RED gate: bad bytes never map to a
    /// usable bundle).
    fn decode(&self, raw: &[u8]) -> Result<Bundle, String>;
}
