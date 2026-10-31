// SPDX-License-Identifier: MIT OR Apache-2.0
//! The two seams `bebop_log::doc::verify` asks the caller for, as THIS reader fills them.
//!
//! `ReaderIds` re-derives dowiz-hub's documented id schemes (`crates/dowiz-hub/src/chain/scheme.rs`,
//! FORMAT §9.2): a record's id is `sha256(prev ‖ payload)` (M0, the payload led by a 16-byte
//! nonce), or under one of the two legacy FNV-1a x4 schemes of older images (over `prev ‖ payload`
//! from 2026-09-20, over the payload alone before). A redaction leaves the nonce erased to sixteen
//! `'0'` bytes. These are FORMAT rules re-derived from the text, exactly as `oracle.py` re-derives
//! the fold; the test fixture written with `ToyIds` (bebop-log's test mix) therefore counts
//! `broken`, which is the honest answer about a toy file.
//!
//! `NoVerifier` knows NO signature algorithm: this crate is MIT OR Apache-2.0 and carries no
//! ML-DSA, so every seal is COUNTED and none is CHECKED. `abi::bw_counts` reports `checked = 0`
//! and the words in `status.rs` say so (R-MAINGOAL-A §5.3: testimony, not authority). A host that
//! wants the seals checked brings an ML-DSA-65 verifier (dowiz-hub, AGPL) to the same seam.

use bebop_log::doc::{Ids, Verifier};

use crate::sha256::sha256;

/// The nonce in front of every post-M0 payload, and what a redaction leaves there.
pub const NONCE_LEN: usize = 16;
const ERASED: [u8; NONCE_LEN] = [b'0'; NONCE_LEN];

/// FNV-1a 64 over four salted lanes, spread to 32 bytes: the pre-M0 id.
fn legacy_mix(bytes: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for lane in 0..4u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ lane.wrapping_mul(0x9E37_79B9_7F4A_7C15);
        for b in bytes {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100_0000_01b3);
        }
        out[lane as usize * 8..lane as usize * 8 + 8].copy_from_slice(&h.to_le_bytes());
    }
    out
}

pub struct ReaderIds;

impl Ids for ReaderIds {
    fn chained(&self, prev: &[u8; 32], payload: &[u8]) -> [u8; 32] {
        let mut m = Vec::with_capacity(32 + payload.len());
        m.extend_from_slice(prev);
        m.extend_from_slice(payload);
        sha256(&m)
    }
    /// Both legacy schemes: FNV over `prev ‖ payload` and over the payload alone.
    fn legacy(&self, flags: i64, prev: &[u8; 32], payload: &[u8], id: &[u8; 32]) -> bool {
        if flags & bebop_log::FLAG_SHA256 != 0 {
            return false;
        }
        let mut m = prev.to_vec();
        m.extend_from_slice(payload);
        legacy_mix(&m) == *id || legacy_mix(payload) == *id
    }
    fn redacted(&self, flags: i64, payload: &[u8]) -> bool {
        flags & bebop_log::FLAG_SHA256 != 0 && payload.len() >= NONCE_LEN && payload[..NONCE_LEN] == ERASED
    }
    fn body<'a>(&self, flags: i64, payload: &'a [u8]) -> &'a [u8] {
        if flags & bebop_log::FLAG_SHA256 != 0 {
            payload.get(NONCE_LEN..).unwrap_or(&[])
        } else {
            payload
        }
    }
}

pub struct NoVerifier;

impl Verifier for NoVerifier {
    fn verify(&self, _alg: i64, _pk: &[u8], _msg: &[u8], _sig: &[u8]) -> bool {
        false
    }
    fn checks_signatures(&self) -> bool {
        false
    }
}
