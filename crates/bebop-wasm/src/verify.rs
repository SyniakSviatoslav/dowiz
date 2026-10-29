//! The price-proof reader (LV-PCD-C, AC-LV-PCD-03; normative text `docs/design/SPEC-PCD-2026-10-27.md`):
//! a receipt line's certificate, climbed to its root IN THE BROWSER, with no network.
//!
//! `bw_verify(ptr, len, out)` reads one certificate frame and writes the 32-byte root its path
//! climbs to into `out` (zeros when the frame is malformed); the status says whether that root is
//! the one the certificate names. `bw_qr(ptr, len, out, cap)` draws a text as a QR module grid --
//! the SAME `qrcodegen` crate the Worker draws table codes with (`services/orders/room/table_qr.rs`),
//! so the tree keeps one QR implementation and the receipt's code is drawn offline.
//!
//! Compiled only for tests and under `--features verify` (built apart by `verify.sh` into
//! `target/verify/`, promoted to `workers/api/public/store/verify.wasm`), so the Worker's reader
//! module and `bytes.baseline` carry none of it.

use bebop_store::merkle::{Cert, CertError, Hash};

/// The certificate does not parse, or its shape is impossible (SPEC-PCD §3).
pub const MALFORMED: i32 = 50;
/// The path climbs to a root that is not the one the certificate names.
pub const MISMATCH: i32 = 51;
/// `bw_qr`: the text does not fit a QR code, or `cap` is too small for the grid.
pub const QR_TOO_LONG: i32 = 52;

/// Status and the climbed root (zeros when malformed).
pub fn verify_cert(bytes: &[u8]) -> (i32, Hash) {
    match Cert::decode(bytes).and_then(|c| c.check()) {
        Ok(r) => (crate::abi::OK, r),
        Err(CertError::Mismatch { climbed }) => (MISMATCH, climbed),
        Err(CertError::Malformed) => (MALFORMED, [0; 32]),
    }
}

/// The line every reader of `fixtures/verify.expected` prints.
pub fn line(name: &str, bytes: &[u8]) -> String {
    let (s, r) = verify_cert(bytes);
    format!("verify {name} status={s} root={}", bebop_store::nodekey::hex(&r))
}

/// # Safety
/// `ptr..ptr+len` is readable and `out` points at 32 writable bytes.
#[cfg_attr(feature = "verify", no_mangle)]
pub unsafe extern "C" fn bw_verify(ptr: *const u8, len: usize, out: *mut u8) -> i32 {
    if ptr.is_null() || out.is_null() {
        return crate::abi::NULL_ARG;
    }
    let (s, r) = verify_cert(core::slice::from_raw_parts(ptr, len));
    core::ptr::copy_nonoverlapping(r.as_ptr(), out, 32);
    s
}

/// The QR grid of `text` (ECC medium), one byte per module (1 = dark), row-major. Returns the side
/// length on success (the grid is side*side bytes at `out`), or a status <= 0 negated.
#[cfg(feature = "verify")]
pub fn qr(text: &[u8]) -> Option<(usize, Vec<u8>)> {
    let s = core::str::from_utf8(text).ok()?;
    let q = qrcodegen::QrCode::encode_text(s, qrcodegen::QrCodeEcc::Medium).ok()?;
    let n = q.size() as usize;
    let mut v = Vec::with_capacity(n * n);
    for y in 0..n as i32 {
        for x in 0..n as i32 {
            v.push(q.get_module(x, y) as u8);
        }
    }
    Some((n, v))
}

/// # Safety
/// `ptr..ptr+len` is readable and `out..out+cap` writable. Returns the side (> 0) or `-QR_TOO_LONG`.
#[cfg(feature = "verify")]
#[no_mangle]
pub unsafe extern "C" fn bw_qr(ptr: *const u8, len: usize, out: *mut u8, cap: usize) -> i32 {
    if ptr.is_null() || out.is_null() {
        return -crate::abi::NULL_ARG;
    }
    match qr(core::slice::from_raw_parts(ptr, len)) {
        Some((n, v)) if v.len() <= cap => {
            core::ptr::copy_nonoverlapping(v.as_ptr(), out, v.len());
            n as i32
        }
        _ => -QR_TOO_LONG,
    }
}

#[cfg(test)]
mod tests;
