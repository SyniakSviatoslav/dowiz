// SPDX-License-Identifier: MIT OR Apache-2.0
//! The two seams `bebop_log::doc::verify` asks the caller for, as THIS reader fills them.
//!
//! `ReaderIds` re-derives dowiz-hub's documented id schemes (`crates/dowiz-hub/src/chain/scheme.rs`,
//! FORMAT §9.2): a record's id is the sha256 of `bebop_log::doc::id_message` (id v2, W-MG-HOLES:
//! `"ein-record/1" ‖ flags ‖ actor ‖ seq ‖ prev ‖ payload`, the payload led by M0's 16-byte
//! nonce), or under one of the two legacy FNV-1a x4 schemes of older images (over `prev ‖ payload`
//! from 2026-09-20, over the payload alone before). A redaction keeps its id and shows as dowiz-hub's
//! mark, the high bit of the kind byte (`forget.rs`), behind the nonce when there is one. That bit
//! is the ONLY mark (FORMAT §9.4): a legacy record has no nonce to erase, so a nonce of sixteen
//! `'0'` bytes with no bit behind it is an edit, and reads broken, as it does to `HubIds`.
//! These are FORMAT rules re-derived from the text, exactly as `oracle.py` re-derives
//! the fold; the test fixture written with `ToyIds` (bebop-log's test mix) therefore counts
//! `broken`, which is the honest answer about a toy file.
//!
//! `NoVerifier` knows NO signature algorithm: this crate is MIT OR Apache-2.0 and carries no
//! ML-DSA, so every seal is COUNTED and none is CHECKED. `abi::bw_counts` reports `checked = 0`
//! and the words in `status.rs` say so (R-MAINGOAL-A §5.3: testimony, not authority). A host that
//! wants the seals checked brings an ML-DSA-65 verifier (dowiz-hub, AGPL) to the same seam.

use bebop_log::doc::{Ids, Verifier};

use crate::sha256::sha256;

/// The nonce in front of every post-M0 payload.
pub const NONCE_LEN: usize = 16;
/// `forget.rs`'s tombstone mark on the kind byte.
const REDACTED_BIT: u8 = 0x80;

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
    fn chained(&self, r: &bebop_log::Record) -> [u8; 32] {
        sha256(&bebop_log::doc::id_message(r.flags, &r.actor_pubkey, r.actor_seq, &r.prev, &r.payload))
    }
    /// The SEG chain's hash (`seg::chain_hash`). Without it every cut file is refused (75).
    fn hash(&self, bytes: &[u8]) -> Option<[u8; 32]> {
        Some(sha256(bytes))
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
        let b = if flags & bebop_log::FLAG_SHA256 != 0 { payload.get(NONCE_LEN..).unwrap_or(&[]) } else { payload };
        b.first().is_some_and(|k| k & REDACTED_BIT != 0)
    }
    /// FORMAT §9.4, the owner's framing: M0's nonce, then dowiz-hub's kind byte on a declaration
    /// (`D`) or a key op (`K`) -- without this a REAL hub's TRUNCATE reads as no TRUNCATE (75). A
    /// body with no kind byte (bebop-log's own fixtures) starts at its mark.
    fn body<'a>(&self, flags: i64, payload: &'a [u8]) -> &'a [u8] {
        let b = if flags & bebop_log::FLAG_SHA256 != 0 { payload.get(NONCE_LEN..).unwrap_or(&[]) } else { payload };
        match b.first() {
            Some(b'D' | b'K') => &b[1..],
            _ => b,
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
