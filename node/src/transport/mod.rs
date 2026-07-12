//! Transport layer: wire codec mapping the in-memory [`crate::Bundle`] onto a
//! real RFC 9171 BPv7 bundle (via the `bp7` crate, `dtn7/bp7-rs`).
//!
//! S1 integration surface (operator-gated). This module ONLY adds the codec +
//! a `Transport` trait; it does NOT touch `Node`, roles, store, or adapters.
//! The in-memory `Node` mesh remains the oracle for custody/forward/deliver.

pub mod bp7;

use crate::Bundle;

/// A transport encodes/decodes a [`Bundle`] to/from wire bytes.
///
/// The concrete RFC 9171 impl is [`bp7::Bp7Transport`]; the trait lets the
/// node's store-and-forward semantics stay underlay-agnostic (QUIC/TCPCLv4/
/// SpaceWire all satisfy the same contract — D3 underlay independence).
pub trait Transport {
    /// Serialize a bundle to wire bytes (BPv7 CBOR for [`bp7::Bp7Transport`]).
    fn encode(&self, b: &Bundle) -> Vec<u8>;

    /// Parse wire bytes back into a [`Bundle`]. Returns `Err` on any malformed,
    /// truncated, or non-conforming input (RED gate: bad bytes never map to a
    /// usable bundle).
    fn decode(&self, raw: &[u8]) -> Result<Bundle, String>;
}
