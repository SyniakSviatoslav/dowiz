//! codec — the serialization boundary for the Immutable Log.
//!
//! Canonical bytes for events (and any core value): DETERMINISTIC, because our types are ordered
//! structs and enums with no maps — the same log always encodes to the same bytes. That determinism
//! is the property everything downstream needs: replaying/persisting the log (Manifesto §2 Storage),
//! a content hash or signature over a command/event (Phase Three PQC — stable bytes to sign), and
//! cross-node replication (Phase Three mesh — every node must derive identical bytes).
//!
//! `serde_json` is the current wire. rkyv/protobuf (Manifesto §1 binary serialization) is a future
//! ADDITIVE swap BEHIND this seam — the core types never change, only the encoder bolted on here.
//!
//! Sibling module [`request_hash`] — the S5 idempotency canonicalisation — lives here too: it is the
//! same family (deterministic canonical bytes → a content hash), relocated from the `api` shell in
//! Phase-Zero Step 3. It hashes an ALREADY-INTEGER-projected input (the f64→i64 coordinate projection
//! stays in the shell, keeping this core float-free).

pub mod request_hash;

use crate::kernel::Event;
use serde::{Serialize, de::DeserializeOwned};

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CodecError {
    #[error("codec encode failed: {0}")]
    Encode(String),
    #[error("codec decode failed: {0}")]
    Decode(String),
}

/// Canonical bytes for any core value. Deterministic for the core's ordered structs/enums.
pub fn canonical_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, CodecError> {
    serde_json::to_vec(value).map_err(|e| CodecError::Encode(e.to_string()))
}

/// Decode any core value from canonical bytes.
pub fn from_bytes<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, CodecError> {
    serde_json::from_slice(bytes).map_err(|e| CodecError::Decode(e.to_string()))
}

/// Encode a whole event log to canonical bytes (the Immutable Log's on-wire form).
pub fn encode_log(events: &[Event]) -> Result<Vec<u8>, CodecError> {
    canonical_bytes(&events)
}

/// Decode an event log from canonical bytes.
pub fn decode_log(bytes: &[u8]) -> Result<Vec<Event>, CodecError> {
    from_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{OrderStatus, Ts};

    fn sample_log() -> Vec<Event> {
        vec![
            Event::StatusChanged { from: OrderStatus::Pending, to: OrderStatus::Confirmed, at: Ts(1) },
            Event::StatusChanged { from: OrderStatus::Confirmed, to: OrderStatus::Preparing, at: Ts(2) },
        ]
    }

    #[test]
    fn log_round_trips_through_canonical_bytes() {
        let log = sample_log();
        let bytes = encode_log(&log).unwrap();
        assert_eq!(decode_log(&bytes).unwrap(), log);
    }

    #[test]
    fn encoding_is_deterministic() {
        let log = sample_log();
        assert_eq!(encode_log(&log).unwrap(), encode_log(&log).unwrap());
    }

    #[test]
    fn event_encodes_with_its_screaming_snake_tag() {
        let bytes = canonical_bytes(&sample_log()[0]).unwrap();
        let s = String::from_utf8(bytes).unwrap();
        assert!(s.contains("\"STATUS_CHANGED\""), "got {s}");
        assert!(s.contains("\"PENDING\"") && s.contains("\"CONFIRMED\""));
    }

    #[test]
    fn decode_rejects_garbage() {
        assert!(matches!(decode_log(b"not json"), Err(CodecError::Decode(_))));
    }
}
