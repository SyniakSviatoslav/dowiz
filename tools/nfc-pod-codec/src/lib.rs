//! dowiz NFC proof-of-delivery (PoD) codec — hardware-independent, server-side.
//!
//! Two layers:
//!   * [`ndef`] — a minimal NFC Forum NDEF (v1.0) single-record External-Type codec
//!     (the transport container any NDEF reader — a courier phone, or a Flipper Zero
//!     in dev — reads/writes).
//!   * [`pod`]  — the dowiz PoD payload that lives inside the record: a versioned
//!     `(order_id: u64, issued_at: u64)` bound by a truncated keyed SHAKE256 MAC,
//!     reusing `kernel::pq::keccak` (no new crypto).
//!
//! The two convenience functions below stitch them into the full on-tag byte
//! string and back, which is exactly what a provisioning service writes to a tag
//! and what the courier-tap ingest endpoint verifies.

pub mod ndef;
pub mod pod;

/// The NFC Forum External Type name for a dowiz PoD tag (`<domain>:<type>`).
pub const POD_TYPE: &[u8] = b"dowiz.io:pod";

/// Errors from the full encode→NDEF / NDEF→verify path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TagError {
    Ndef(ndef::NdefError),
    Pod(pod::PodError),
}

impl From<ndef::NdefError> for TagError {
    fn from(e: ndef::NdefError) -> Self {
        TagError::Ndef(e)
    }
}
impl From<pod::PodError> for TagError {
    fn from(e: pod::PodError) -> Self {
        TagError::Pod(e)
    }
}

/// Produce the complete NDEF byte string to burn onto a passive PoD tag.
pub fn encode_tag(order_id: u64, issued_at: u64, key: &[u8]) -> Vec<u8> {
    let payload = pod::encode_payload(order_id, issued_at, key);
    ndef::encode_external(POD_TYPE, &payload)
}

/// Parse an NDEF message read off a tag and verify the PoD proof under `key`.
pub fn decode_and_verify_tag(buf: &[u8], key: &[u8]) -> Result<pod::PodRecord, TagError> {
    let payload = ndef::decode_external(buf, POD_TYPE)?;
    Ok(pod::decode_and_verify(payload, key)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &[u8] = b"provisioning-key-32-bytes-000000";

    #[test]
    fn full_tag_roundtrip() {
        let buf = encode_tag(777, 1_700_000_000, KEY);
        // Whole tag comfortably fits an NTAG213 (144 B user memory): 3 + 12 + 33 = 48 B.
        assert!(buf.len() <= 144, "tag payload {} B exceeds NTAG213", buf.len());
        let rec = decode_and_verify_tag(&buf, KEY).expect("valid tag verifies");
        assert_eq!(rec.order_id, 777);
        assert_eq!(rec.issued_at, 1_700_000_000);
    }

    #[test]
    fn tampered_tag_payload_rejected() {
        let mut buf = encode_tag(777, 1_700_000_000, KEY);
        let last = buf.len() - 1;
        buf[last] ^= 0x01; // flip a MAC byte
        assert_eq!(
            decode_and_verify_tag(&buf, KEY),
            Err(TagError::Pod(pod::PodError::BadMac))
        );
    }

    #[test]
    fn malformed_ndef_header_rejected() {
        // A buffer whose NDEF header is corrupt (TNF wrong) fails at the NDEF layer,
        // before any PoD/MAC work happens.
        let mut buf = encode_tag(777, 1_700_000_000, KEY);
        buf[0] = 0x00; // wipe flags/TNF
        assert!(matches!(decode_and_verify_tag(&buf, KEY), Err(TagError::Ndef(_))));
    }
}
