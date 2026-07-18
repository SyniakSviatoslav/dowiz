//! Proof-of-Delivery (PoD) tag payload — the bytes that live INSIDE the NDEF
//! external record and prove a courier physically tapped the venue/customer tag.
//!
//! Field vocabulary is bound to the kernel's real event vocabulary: the order
//! identifier is `u64` (see `bebop2/proto-cap/src/event_dict.rs`
//! `OrderPlacedPayload { order_id: u64, .. }`) and is encoded big-endian, the same
//! convention as `event_dict`'s `put_u64`/`get_u64`.
//!
//! The "short cryptographic proof" is a keyed MAC, NOT a signature. A passive NFC
//! tag (NTAG213 = 144 B user memory) cannot hold an ML-DSA-65 signature (3309 B),
//! so we reuse the kernel's audited SHAKE256 (FIPS 202, `kernel::pq::keccak`) as a
//! prefix-keyed MAC truncated to 16 bytes. No new crypto is invented: this is the
//! same SHAKE256 the envelope/dsa layer already depends on, used in the same
//! `SHAKE256(key || context)` shape the kernel uses in `keccak::prf` / `xof_j`.
//!
//! Trust model (honest scope): this MAC proves the tag was PROVISIONED by a holder
//! of the server provisioning key and binds (order_id, issued_at) together. It is a
//! symmetric secret verified server-side; it is a genuine tamper/forgery gate for
//! the tag bytes, but it is NOT a public-key device-identity — that belongs to
//! WebAuthn/FIDO2 on the auth side, not to the tag.

use dowiz_kernel::pq::keccak::shake256;

/// On-tag record version. Bump if the field layout below ever changes.
pub const POD_VERSION: u8 = 1;

/// Truncated-MAC length in bytes (128-bit tag: forgery prob 2^-128).
pub const MAC_LEN: usize = 16;

/// Fixed field sizes: version(1) + order_id(8) + issued_at(8) + mac(16) = 33 bytes.
pub const POD_PAYLOAD_LEN: usize = 1 + 8 + 8 + MAC_LEN;

/// Domain-separation tag mixed into the MAC so this key can never be confused with
/// another SHAKE256 use of the same secret elsewhere.
const MAC_DOMAIN: &[u8] = b"dowiz/pod-mac/v1";

/// A decoded, structurally-valid PoD record (MAC not yet checked / already checked
/// depending on the constructor).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PodRecord {
    pub version: u8,
    /// Kernel order identifier (`event_dict` `order_id: u64`).
    pub order_id: u64,
    /// Provisioning time, unix seconds.
    pub issued_at: u64,
}

/// Why a PoD payload was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PodError {
    /// Payload was not exactly `POD_PAYLOAD_LEN` bytes.
    BadLength,
    /// Unknown/unsupported version byte.
    BadVersion(u8),
    /// MAC did not verify under the provisioning key (tampered or forged).
    BadMac,
}

/// Compute the 16-byte keyed MAC over the record fields.
/// MAC = SHAKE256( key || MAC_DOMAIN || version || order_id_be || issued_at_be )[..16].
fn compute_mac(key: &[u8], rec: &PodRecord) -> [u8; MAC_LEN] {
    let mut input = Vec::with_capacity(key.len() + MAC_DOMAIN.len() + 17);
    input.extend_from_slice(key);
    input.extend_from_slice(MAC_DOMAIN);
    input.push(rec.version);
    input.extend_from_slice(&rec.order_id.to_be_bytes());
    input.extend_from_slice(&rec.issued_at.to_be_bytes());
    let mut out = [0u8; MAC_LEN];
    shake256(&input, &mut out);
    out
}

/// Constant-time-ish equality for the MAC (avoids early-exit timing leak on the
/// 16-byte compare). Length is fixed so this is a simple xor-accumulate.
fn macs_equal(a: &[u8; MAC_LEN], b: &[u8]) -> bool {
    if b.len() != MAC_LEN {
        return false;
    }
    let mut diff = 0u8;
    for i in 0..MAC_LEN {
        diff |= a[i] ^ b[i];
    }
    diff == 0
}

/// Encode a PoD record + MAC into the 33-byte on-tag payload.
pub fn encode_payload(order_id: u64, issued_at: u64, key: &[u8]) -> Vec<u8> {
    let rec = PodRecord { version: POD_VERSION, order_id, issued_at };
    let mac = compute_mac(key, &rec);
    let mut out = Vec::with_capacity(POD_PAYLOAD_LEN);
    out.push(rec.version);
    out.extend_from_slice(&rec.order_id.to_be_bytes());
    out.extend_from_slice(&rec.issued_at.to_be_bytes());
    out.extend_from_slice(&mac);
    debug_assert_eq!(out.len(), POD_PAYLOAD_LEN);
    out
}

/// Decode + verify a PoD payload under the provisioning key. Returns the record
/// only if the length, version, and MAC all check out.
pub fn decode_and_verify(payload: &[u8], key: &[u8]) -> Result<PodRecord, PodError> {
    if payload.len() != POD_PAYLOAD_LEN {
        return Err(PodError::BadLength);
    }
    let version = payload[0];
    if version != POD_VERSION {
        return Err(PodError::BadVersion(version));
    }
    let order_id = u64::from_be_bytes(payload[1..9].try_into().unwrap());
    let issued_at = u64::from_be_bytes(payload[9..17].try_into().unwrap());
    let rec = PodRecord { version, order_id, issued_at };
    let expected = compute_mac(key, &rec);
    if !macs_equal(&expected, &payload[17..33]) {
        return Err(PodError::BadMac);
    }
    Ok(rec)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &[u8] = b"provisioning-key-32-bytes-000000"; // 32 bytes

    #[test]
    fn payload_length_is_fixed_33() {
        let p = encode_payload(42, 1_700_000_000, KEY);
        assert_eq!(p.len(), POD_PAYLOAD_LEN);
        assert_eq!(p.len(), 33);
    }

    #[test]
    fn roundtrip_verifies() {
        let p = encode_payload(0xDEAD_BEEF_u64, 1_700_000_123, KEY);
        let rec = decode_and_verify(&p, KEY).expect("valid MAC verifies");
        assert_eq!(rec.order_id, 0xDEAD_BEEF);
        assert_eq!(rec.issued_at, 1_700_000_123);
        assert_eq!(rec.version, POD_VERSION);
    }

    #[test]
    fn tampered_order_id_rejected() {
        let mut p = encode_payload(1000, 1_700_000_000, KEY);
        p[8] ^= 0x01; // flip a byte of order_id → MAC no longer matches
        assert_eq!(decode_and_verify(&p, KEY), Err(PodError::BadMac));
    }

    #[test]
    fn tampered_timestamp_rejected() {
        let mut p = encode_payload(1000, 1_700_000_000, KEY);
        p[16] ^= 0xFF; // flip a byte of issued_at
        assert_eq!(decode_and_verify(&p, KEY), Err(PodError::BadMac));
    }

    #[test]
    fn wrong_key_rejected() {
        let p = encode_payload(1000, 1_700_000_000, KEY);
        assert_eq!(
            decode_and_verify(&p, b"different-key-32-bytes-0000000000"),
            Err(PodError::BadMac)
        );
    }

    #[test]
    fn bad_length_rejected() {
        assert_eq!(decode_and_verify(b"too short", KEY), Err(PodError::BadLength));
    }

    #[test]
    fn bad_version_rejected() {
        let mut p = encode_payload(1, 2, KEY);
        p[0] = 0xFF;
        assert_eq!(decode_and_verify(&p, KEY), Err(PodError::BadVersion(0xFF)));
    }

    #[test]
    fn distinct_orders_distinct_macs() {
        let a = encode_payload(1, 100, KEY);
        let b = encode_payload(2, 100, KEY);
        assert_ne!(&a[17..], &b[17..], "different order_id → different MAC");
    }
}
