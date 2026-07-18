//! dowiz Proof-of-Delivery NFC reader — Flipper Zero dev-tooling app.
//!
//! PURPOSE (dev/test tooling, NOT production hardware): give an engineer a
//! portable device that reads a dowiz PoD NDEF tag and prints the decoded
//! `order_id`, so the on-tag wire format and the courier-tap read flow can be
//! validated BEFORE production NTAG stickers or courier phones exist.
//!
//! HARDWARE HONESTY: this file was compiled but NOT run — there is no physical
//! Flipper Zero in the build environment. The NFC poller callback (the part that
//! reads bytes off a real tag) is therefore left as an explicitly-marked device
//! seam rather than faked. The pure NDEF/PoD decode below is real and identical in
//! wire format to the server-side `nfc-pod-codec` crate.

#![no_main]
#![no_std]

// Provides the panic handler + the FAP entry/manifest machinery.
extern crate flipperzero_rt;

use core::ffi::CStr;

use flipperzero::{error, info};
use flipperzero_rt::{entry, manifest};
use flipperzero_sys as sys;

// FAP manifest (no icon, to avoid bundling an asset).
manifest!(name = "dowiz PoD NFC reader");
entry!(main);

/// NFC Forum External Type name for a dowiz PoD tag (`<domain>:<type>`).
const POD_TYPE: &[u8] = b"dowiz.io:pod";

/// Fixed on-tag PoD payload length: version(1) + order_id(8) + issued_at(8) + mac(16).
const POD_PAYLOAD_LEN: usize = 33;

// ── Pure decode (no_std, no hardware) — mirrors nfc_pod_codec::ndef/pod ──────────

/// Decode a single short NDEF External-Type record and verify the type name.
/// Returns the record payload slice, or `None` if the buffer is malformed / not a
/// dowiz PoD tag. Wire format matches `nfc_pod_codec::ndef::decode_external`.
fn decode_external<'a>(buf: &'a [u8], expected_type: &[u8]) -> Option<&'a [u8]> {
    if buf.len() < 3 {
        return None;
    }
    let flags = buf[0];
    // Require: MB(0x80) ME(0x40) set, CF(0x20) clear, SR(0x10) set, IL(0x08) clear,
    // TNF(0x07) == 0x04 (External).
    if flags & 0x20 != 0 || flags & 0x10 == 0 || flags & 0x08 != 0 {
        return None;
    }
    if flags & 0x80 == 0 || flags & 0x40 == 0 {
        return None;
    }
    if flags & 0x07 != 0x04 {
        return None;
    }
    let type_len = buf[1] as usize;
    let payload_len = buf[2] as usize;
    let type_end = 3usize.checked_add(type_len)?;
    if type_end > buf.len() || &buf[3..type_end] != expected_type {
        return None;
    }
    let pay_end = type_end.checked_add(payload_len)?;
    if pay_end > buf.len() {
        return None;
    }
    Some(&buf[type_end..pay_end])
}

/// Extract `order_id` from a structurally-valid PoD payload. The MAC is NOT
/// verified on-device (no provisioning key lives on the Flipper — verification is
/// server-side). This is display-only.
fn pod_order_id(payload: &[u8]) -> Option<u64> {
    if payload.len() != POD_PAYLOAD_LEN || payload[0] != 1 {
        return None;
    }
    let mut oid = [0u8; 8];
    oid.copy_from_slice(&payload[1..9]);
    Some(u64::from_be_bytes(oid))
}

/// A sample PoD tag byte string (order_id = 42), used to exercise the decode path
/// at build time in the absence of a physical tag. Byte layout:
///   D4 0C 21 | "dowiz.io:pod" | 01 <order_id BE=42> <ts BE> <16-byte MAC placeholder>
#[rustfmt::skip]
const SAMPLE_TAG: [u8; 48] = [
    0xD4, 0x0C, 0x21,
    b'd', b'o', b'w', b'i', b'z', b'.', b'i', b'o', b':', b'p', b'o', b'd',
    0x01,                                           // version
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x2A, // order_id = 42
    0x00, 0x00, 0x00, 0x00, 0x65, 0x50, 0x8D, 0x00, // issued_at (arbitrary)
    0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x11, 0x22, // MAC (placeholder — verified server-side)
    0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0x00,
];

fn main(_args: Option<&CStr>) -> i32 {
    // ── Device NFC seam (real flipperzero-sys calls) ────────────────────────────
    // Allocate the NFC HAL and an ISO14443-3A poller (NTAG21x are ISO14443-3A).
    // On a physical Flipper this is followed by `nfc_poller_start(poller, cb, ctx)`
    // whose callback, on `NfcPollerEventTypeReady`, reads the tag's NDEF memory into
    // a buffer that is then fed to `decode_external`/`pod_order_id` below. That
    // callback cannot be exercised without hardware, so it is intentionally NOT
    // faked here — only the real alloc/free lifecycle is wired.
    unsafe {
        let nfc = sys::nfc_alloc();
        let poller = sys::nfc_poller_alloc(nfc, sys::NfcProtocolIso14443_3a);
        // <-- nfc_poller_start(...) + callback would run here on real hardware -->
        sys::nfc_poller_free(poller);
        sys::nfc_free(nfc);
    }

    // ── Pure decode demo (compiled + reasoned, no hardware needed) ──────────────
    // Decodes the SAMPLE_TAG exactly as the on-device read path would decode bytes
    // read off a real tag, and logs the order_id.
    match decode_external(&SAMPLE_TAG, POD_TYPE).and_then(pod_order_id) {
        Some(order_id) => info!("dowiz PoD tag: order_id={}", order_id),
        None => error!("not a dowiz PoD tag (malformed NDEF or wrong type)"),
    }

    0
}
