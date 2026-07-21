//! hex_util.rs — Kernel-native hex encode/decode primitive.
//!
//! # What this is
//! A single, canonical hex encoding/decoding module that replaces the 6+
//! redundant hand-rolled implementations scattered across the codebase.
//! Pure Rust, zero deps, deterministic, SHA3-verifiable.
//!
//! # Why this exists
//! The 2026-07-21 audit found 6+ independent hex encode/decode implementations
//! (wallet/record.rs, backup.rs, deploy_config.rs, spine_snapshot.rs,
//! breaker/audit.rs, hydra.rs, csr.rs, json.rs, pq/keccak.rs, pq/x25519.rs)
//! each with slightly different signatures and error handling. This module
//! provides the canonical implementation.
//!
//! # Design
//! - `encode(bytes) -> String` — lower-case hex, no prefix
//! - `decode(hex_str) -> Result<Vec<u8>>` — validates input, rejects non-hex chars
//! - `encode_fixed32(bytes) -> [u8; 32]` — for 32-byte arrays (SHA3 digests)
//! - `decode_fixed32(hex_str) -> Result<[u8; 32]>` — parse exactly 32 bytes
//! - All functions are pure, deterministic, zero-alloc where possible

use std::fmt;

/// Error from hex decoding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HexError {
    /// Input contains non-hex characters.
    InvalidChar { pos: usize, ch: char },
    /// Input has odd length (not a valid hex string).
    OddLength(usize),
    /// Input is too long for a fixed-size target.
    TooLong { max: usize, got: usize },
    /// Input is too short for a fixed-size target.
    TooShort { max: usize, got: usize },
}

impl fmt::Display for HexError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HexError::InvalidChar { pos, ch } => {
                write!(f, "hex: invalid character '{}' at position {}", ch, pos)
            }
            HexError::OddLength(len) => {
                write!(f, "hex: odd length {} (must be even)", len)
            }
            HexError::TooLong { max, got } => {
                write!(f, "hex: too long — max {} chars, got {}", max, got)
            }
            HexError::TooShort { max, got } => {
                write!(f, "hex: too short — need {} chars, got {}", max, got)
            }
        }
    }
}

/// Nibble value from a hex character.
fn nibble(ch: u8) -> Result<u8, HexError> {
    match ch {
        b'0'..=b'9' => Ok(ch - b'0'),
        b'a'..=b'f' => Ok(ch - b'a' + 10),
        b'A'..=b'F' => Ok(ch - b'A' + 10),
        _ => Err(HexError::InvalidChar {
            pos: 0,
            ch: ch as char,
        }),
    }
}

/// Lookup table for hex encoding (lowercase).
const HEX_LUT: &[u8; 16] = b"0123456789abcdef";

/// Encode bytes to lowercase hex string (no prefix).
///
/// ```text
/// encode(b"\xde\xad") -> "dead"
/// encode(b"\xff")     -> "ff"
/// encode(b"")         -> ""
/// ```
pub fn encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX_LUT[(b >> 4) as usize] as char);
        out.push(HEX_LUT[(b & 0x0f) as usize] as char);
    }
    out
}

/// Decode a hex string to bytes.
///
/// ```text
/// decode("dead") -> Ok(vec![0xde, 0xad])
/// decode("xyz")  -> Err(InvalidChar)
/// decode("abc")  -> Err(OddLength(3))
/// ```
pub fn decode(hex_str: &str) -> Result<Vec<u8>, HexError> {
    let bytes = hex_str.as_bytes();
    if bytes.len() % 2 != 0 {
        return Err(HexError::OddLength(bytes.len()));
    }
    let mut out = Vec::with_capacity(bytes.len() / 2);
    for pair in bytes.chunks(2) {
        let hi = nibble(pair[0]).map_err(|_| HexError::InvalidChar {
            pos: 0,
            ch: pair[0] as char,
        })?;
        let lo = nibble(pair[1]).map_err(|_| HexError::InvalidChar {
            pos: 1,
            ch: pair[1] as char,
        })?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

/// Encode a 32-byte array to exactly 64 hex characters.
pub fn encode_fixed32(bytes: &[u8; 32]) -> String {
    encode(bytes)
}

/// Decode a 64-character hex string to exactly 32 bytes.
pub fn decode_fixed32(hex_str: &str) -> Result<[u8; 32], HexError> {
    let bytes = hex_str.as_bytes();
    if bytes.len() > 64 {
        return Err(HexError::TooLong { max: 64, got: bytes.len() });
    }
    if bytes.len() < 64 {
        return Err(HexError::TooShort { max: 64, got: bytes.len() });
    }
    let mut out = [0u8; 32];
    for i in 0..32 {
        let hi = nibble(bytes[i * 2]).map_err(|_| HexError::InvalidChar {
            pos: i * 2,
            ch: bytes[i * 2] as char,
        })?;
        let lo = nibble(bytes[i * 2 + 1]).map_err(|_| HexError::InvalidChar {
            pos: i * 2 + 1,
            ch: bytes[i * 2 + 1] as char,
        })?;
        out[i] = (hi << 4) | lo;
    }
    Ok(out)
}

/// Encode a 16-byte array to exactly 32 hex characters.
pub fn encode_fixed16(bytes: &[u8; 16]) -> String {
    encode(bytes)
}

/// Decode a 32-character hex string to exactly 16 bytes.
pub fn decode_fixed16(hex_str: &str) -> Result<[u8; 16], HexError> {
    let bytes = hex_str.as_bytes();
    if bytes.len() > 32 {
        return Err(HexError::TooLong { max: 32, got: bytes.len() });
    }
    if bytes.len() < 32 {
        return Err(HexError::TooShort { max: 32, got: bytes.len() });
    }
    let mut out = [0u8; 16];
    for i in 0..16 {
        let hi = nibble(bytes[i * 2]).map_err(|_| HexError::InvalidChar {
            pos: i * 2,
            ch: bytes[i * 2] as char,
        })?;
        let lo = nibble(bytes[i * 2 + 1]).map_err(|_| HexError::InvalidChar {
            pos: i * 2 + 1,
            ch: bytes[i * 2 + 1] as char,
        })?;
        out[i] = (hi << 4) | lo;
    }
    Ok(out)
}

/// Check if a byte is a valid hex character.
pub fn is_hex_char(ch: u8) -> bool {
    matches!(ch, b'0'..=b'9' | b'a'..=b'f' | b'A'..=b'F')
}

/// Check if a string is valid hex (even length, all hex chars).
pub fn is_hex_str(s: &str) -> bool {
    !s.is_empty()
        && s.len() % 2 == 0
        && s.bytes().all(is_hex_char)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_empty() {
        assert_eq!(encode(b""), "");
    }

    #[test]
    fn encode_single_byte() {
        assert_eq!(encode(&[0xff]), "ff");
        assert_eq!(encode(&[0x00]), "00");
        assert_eq!(encode(&[0x0a]), "0a");
    }

    #[test]
    fn encode_multi_byte() {
        assert_eq!(encode(&[0xde, 0xad, 0xbe, 0xef]), "deadbeef");
    }

    #[test]
    fn decode_empty() {
        assert_eq!(decode("").unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn decode_valid() {
        assert_eq!(decode("dead").unwrap(), vec![0xde, 0xad]);
        assert_eq!(decode("ff").unwrap(), vec![0xff]);
        assert_eq!(decode("00").unwrap(), vec![0x00]);
    }

    #[test]
    fn decode_odd_length_rejected() {
        assert_eq!(decode("abc"), Err(HexError::OddLength(3)));
    }

    #[test]
    fn decode_invalid_char_rejected() {
        let err = decode("xyzw").unwrap_err();
        assert!(matches!(err, HexError::InvalidChar { .. }));
    }

    #[test]
    fn roundtrip_bytes() {
        let original = vec![0u8, 1, 127, 128, 255, 0xab, 0xcd];
        let hex = encode(&original);
        let decoded = decode(&hex).unwrap();
        assert_eq!(original, decoded);
    }

    #[test]
    fn fixed32_roundtrip() {
        let bytes = [0x42u8; 32];
        let hex = encode_fixed32(&bytes);
        assert_eq!(hex.len(), 64);
        let decoded = decode_fixed32(&hex).unwrap();
        assert_eq!(bytes, decoded);
    }

    #[test]
    fn fixed32_too_short_rejected() {
        assert!(decode_fixed32("ab").is_err());
    }

    #[test]
    fn fixed32_too_long_rejected() {
        assert!(decode_fixed32(&"ff".repeat(33)).is_err());
    }

    #[test]
    fn fixed16_roundtrip() {
        let bytes = [0x42u8; 16];
        let hex = encode_fixed16(&bytes);
        assert_eq!(hex.len(), 32);
        let decoded = decode_fixed16(&hex).unwrap();
        assert_eq!(bytes, decoded);
    }

    #[test]
    fn is_hex_str_valid() {
        assert!(is_hex_str("deadbeef"));
        assert!(is_hex_str("DEADBEEF"));
        assert!(is_hex_str("0123456789abcdef"));
        assert!(!is_hex_str("xyz"));
        assert!(!is_hex_str("abc")); // odd length
        assert!(!is_hex_str("")); // empty
    }

    #[test]
    fn uppercase_decode_works() {
        assert_eq!(decode("DEAD").unwrap(), vec![0xde, 0xad]);
        assert_eq!(decode("DeadBeef").unwrap(), vec![0xde, 0xad, 0xbe, 0xef]);
    }
}
