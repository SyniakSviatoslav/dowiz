//! The seams the rule fixtures are written under (W-MG-FIX): sha256 ids and the two legacy FNV
//! schemes, FORMAT §9.4's tombstone mark and kind byte, and a verifier that judges no seal. This is
//! what the MIT reader fills, so `fixture_tests.rs`'s `.expected` lines are that reader's answers.

use super::super::{id_message, Ids, Verifier};
use super::tests::mth;
use crate::{Record, FLAG_SHA256};
use crate::doc::tests::sha256;

pub(super) const NONCE: usize = 16;
pub(super) const ERASED_NONCE: [u8; NONCE] = [b'0'; NONCE];
pub(super) const TOMB_BIT: u8 = 0x80;
pub(super) const ORDER_KIND: u8 = 1;

/// The pre-M0 id: FNV-1a 64 over four salted lanes (FORMAT §9.2). This crate writes none of its
/// own, so a legacy record needs it spelled out; `FixIds::legacy` is its only other caller.
pub(super) fn legacy_id(bytes: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for lane in 0..4usize {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325 ^ (lane as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        for b in bytes {
            h = (h ^ u64::from(*b)).wrapping_mul(0x100_0000_01b3);
        }
        out[lane * 8..lane * 8 + 8].copy_from_slice(&h.to_le_bytes());
    }
    out
}

pub(super) fn sha_id(r: &Record) -> [u8; 32] {
    sha256(&id_message(r.flags, &r.actor_pubkey, r.actor_seq, &r.prev, &r.payload))
}

/// The seams as FORMAT §9.2-9.4 states them, which is what the MIT reader fills.
pub(super) struct FixIds;
impl FixIds {
    fn tail(flags: i64, payload: &[u8]) -> &[u8] {
        if flags & FLAG_SHA256 != 0 {
            payload.get(NONCE..).unwrap_or(&[])
        } else {
            payload
        }
    }
}
impl Ids for FixIds {
    fn chained(&self, r: &Record) -> [u8; 32] {
        sha_id(r)
    }
    fn legacy(&self, flags: i64, prev: &[u8; 32], payload: &[u8], id: &[u8; 32]) -> bool {
        flags & FLAG_SHA256 == 0 && (legacy_id(&[&prev[..], payload].concat()) == *id || legacy_id(payload) == *id)
    }
    fn redacted(&self, flags: i64, payload: &[u8]) -> bool {
        Self::tail(flags, payload).first().is_some_and(|k| k & TOMB_BIT != 0)
    }
    fn body<'a>(&self, flags: i64, payload: &'a [u8]) -> &'a [u8] {
        let t = Self::tail(flags, payload);
        match t.first() {
            Some(b'D' | b'K') => &t[1..],
            _ => t,
        }
    }
    fn hash(&self, bytes: &[u8]) -> Option<[u8; 32]> {
        Some(sha256(bytes))
    }
    fn tree_root(&self, ids: &[[u8; 32]]) -> Option<[u8; 32]> {
        Some(mth(ids))
    }
}

/// The reader with no signature algorithm: what the fixtures' `.expected` lines are written under.
pub(super) struct Blind;
impl Verifier for Blind {
    fn verify(&self, _: i64, _: &[u8], _: &[u8], _: &[u8]) -> bool {
        false
    }
    fn checks_signatures(&self) -> bool {
        false
    }
}
