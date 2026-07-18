//! Minimal NDEF record codec — single short record, NFC Forum External Type.
//!
//! Implemented against the NFC Forum "NFC Data Exchange Format (NDEF)" Technical
//! Specification v1.0 (NDEF 1.0), §3 "NDEF Record". We emit exactly one record
//! that is both Message-Begin and Message-End (a one-record message), in the
//! Short Record (SR) form, with TNF = 0x04 (NFC Forum External Type). External
//! type names are of the form `<domain>:<type>` (spec §3.2.6 / RTD "External Type"),
//! which is the correct TNF for a proprietary application payload such as ours.
//!
//! Record byte layout (SR form, IL = 0):
//!   byte 0 : flags|TNF  = MB(0x80) ME(0x40) CF(0x20) SR(0x10) IL(0x08) TNF(0x07)
//!   byte 1 : TYPE LENGTH (1 byte)
//!   byte 2 : PAYLOAD LENGTH (1 byte, because SR = 1)
//!   [TYPE]  : TYPE LENGTH bytes
//!   [PAYLOAD]: PAYLOAD LENGTH bytes
//!
//! We deliberately keep this to what the PoD use-case needs (one short external
//! record). Multi-record messages, chunking (CF), long records, and the ID field
//! are rejected on decode rather than silently mis-parsed.

/// Type Name Format = NFC Forum External Type (NDEF 1.0 §3.2.6, value 0x04).
pub const TNF_EXTERNAL: u8 = 0x04;

const FLAG_MB: u8 = 0x80;
const FLAG_ME: u8 = 0x40;
const FLAG_CF: u8 = 0x20;
const FLAG_SR: u8 = 0x10;
const FLAG_IL: u8 = 0x08;
const MASK_TNF: u8 = 0x07;

/// Errors returned when a byte buffer is not a well-formed single-record short
/// external NDEF message of the shape this codec emits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NdefError {
    /// Buffer ended before a complete record could be read.
    Truncated,
    /// Chunk flag set — chunked records are not supported here.
    Chunked,
    /// Not a Short Record (SR = 0). Long records are not supported here.
    NotShortRecord,
    /// TNF was not 0x04 (External Type).
    WrongTnf(u8),
    /// The record is not both Message-Begin and Message-End (i.e. not a lone
    /// single-record message), which is all this codec accepts.
    NotSingleRecord,
    /// Decoded TYPE did not match the expected external type name.
    TypeMismatch,
}

/// Encode a single short NDEF External-Type record.
///
/// `type_name` is the external type (e.g. `b"dowiz.io:pod"`), `payload` the record
/// payload. Panics only on programmer error (type or payload longer than a Short
/// Record can express, 255 bytes) — callers control both, and both are tiny here.
pub fn encode_external(type_name: &[u8], payload: &[u8]) -> Vec<u8> {
    assert!(type_name.len() <= u8::MAX as usize, "type name too long for SR");
    assert!(payload.len() <= u8::MAX as usize, "payload too long for SR");
    let flags = FLAG_MB | FLAG_ME | FLAG_SR | TNF_EXTERNAL; // IL=0, CF=0
    let mut out = Vec::with_capacity(3 + type_name.len() + payload.len());
    out.push(flags);
    out.push(type_name.len() as u8);
    out.push(payload.len() as u8);
    out.extend_from_slice(type_name);
    out.extend_from_slice(payload);
    out
}

/// Decode a single short NDEF External-Type record, verifying the type name.
/// Returns the record payload as a borrowed slice into `buf`.
pub fn decode_external<'a>(buf: &'a [u8], expected_type: &[u8]) -> Result<&'a [u8], NdefError> {
    // Need at least flags + type_len + payload_len.
    if buf.len() < 3 {
        return Err(NdefError::Truncated);
    }
    let flags = buf[0];
    if flags & FLAG_CF != 0 {
        return Err(NdefError::Chunked);
    }
    if flags & FLAG_SR == 0 {
        return Err(NdefError::NotShortRecord);
    }
    let tnf = flags & MASK_TNF;
    if tnf != TNF_EXTERNAL {
        return Err(NdefError::WrongTnf(tnf));
    }
    // We only accept a lone record: both MB and ME must be set.
    if flags & FLAG_MB == 0 || flags & FLAG_ME == 0 {
        return Err(NdefError::NotSingleRecord);
    }

    let type_len = buf[1] as usize;
    let payload_len = buf[2] as usize;
    let mut off = 3usize;

    // ID length byte only present when IL = 1; we emit IL = 0 and reject IL = 1.
    if flags & FLAG_IL != 0 {
        // An ID field would shift offsets; unsupported by this minimal codec.
        return Err(NdefError::NotSingleRecord);
    }

    // TYPE field.
    let type_end = off.checked_add(type_len).ok_or(NdefError::Truncated)?;
    if type_end > buf.len() {
        return Err(NdefError::Truncated);
    }
    let ty = &buf[off..type_end];
    if ty != expected_type {
        return Err(NdefError::TypeMismatch);
    }
    off = type_end;

    // PAYLOAD field.
    let pay_end = off.checked_add(payload_len).ok_or(NdefError::Truncated)?;
    if pay_end > buf.len() {
        return Err(NdefError::Truncated);
    }
    Ok(&buf[off..pay_end])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_external_record() {
        let ty = b"dowiz.io:pod";
        let payload = b"\x01\x02\x03\x04";
        let enc = encode_external(ty, payload);
        // flags byte: MB|ME|SR|TNF04 = 0xD4
        assert_eq!(enc[0], 0xD4);
        assert_eq!(enc[1] as usize, ty.len());
        assert_eq!(enc[2] as usize, payload.len());
        let dec = decode_external(&enc, ty).expect("decodes");
        assert_eq!(dec, payload);
    }

    #[test]
    fn rejects_truncated_header() {
        assert_eq!(decode_external(&[0xD4, 0x0C], b"dowiz.io:pod"), Err(NdefError::Truncated));
    }

    #[test]
    fn rejects_truncated_payload() {
        // Claims payload_len = 10 but supplies fewer bytes.
        let mut enc = encode_external(b"x", b"hello");
        enc[2] = 200; // lie about payload length
        assert_eq!(decode_external(&enc, b"x"), Err(NdefError::Truncated));
    }

    #[test]
    fn rejects_wrong_tnf() {
        let mut enc = encode_external(b"x", b"y");
        enc[0] = (enc[0] & !MASK_TNF) | 0x01; // TNF -> Well-Known
        assert!(matches!(decode_external(&enc, b"x"), Err(NdefError::WrongTnf(0x01))));
    }

    #[test]
    fn rejects_type_mismatch() {
        let enc = encode_external(b"dowiz.io:pod", b"y");
        assert_eq!(decode_external(&enc, b"evil.io:pod"), Err(NdefError::TypeMismatch));
    }

    #[test]
    fn rejects_chunked() {
        let mut enc = encode_external(b"x", b"y");
        enc[0] |= FLAG_CF;
        assert_eq!(decode_external(&enc, b"x"), Err(NdefError::Chunked));
    }
}
