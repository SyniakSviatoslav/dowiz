//! THE ID SCHEMES AND THE NONCE (M0): what `chain.rs` checks with. `chain_id` is the one
//! function a writer uses; the legacy functions exist to CHECK records written before M0,
//! and `verify_id` says which scheme an id holds under. `Entropy` is the nonce source a log
//! carries. The reasoning -- what sha256 buys, what it does not -- is `chain.rs`'s header.

use bebop_store::evlog::{EvLog, Record, FLAG_NONCE_DERIVED, FLAG_SHA256};
use bebop_store::Store;

/// The nonce in front of every post-M0 payload: 16 base64url characters (96 bits), so the
/// text logs stay text and the stock log's wire form carries it as part of the record.
pub(crate) const NONCE_LEN: usize = 16;

/// What a redaction leaves where the nonce was (`forget.rs`, `consent/forget.rs`).
pub(crate) const ERASED_NONCE: [u8; NONCE_LEN] = [b'0'; NONCE_LEN];

/// THE ID OF A RECORD WRITTEN NOW: `sha256(prev ‖ payload)`, the payload with its nonce in
/// front. Every writer in this crate uses this; the legacy functions below exist to CHECK
/// what was written before M0, and `id_for` to re-chain a record whose bytes say which.
pub(crate) fn chain_id(prev: &[u8; 32], payload: &[u8]) -> [u8; 32] {
    let mut buf = Vec::with_capacity(32 + payload.len());
    buf.extend_from_slice(prev);
    buf.extend_from_slice(payload);
    bebop_store::nodekey::sha256(&buf)
}

/// The id a record's bytes call for: sha256 when it carries a nonce (`sha`), the legacy FNV
/// chain otherwise. The stock log's wire form travels as record text with no flags cell, so
/// the hub and a tablet both derive the scheme from the text (`stock::codec::nonced`) and
/// land on the same ids; it is also how a tablet replays a hub's pre-M0 records verbatim.
pub(crate) fn id_for(prev: &[u8; 32], payload: &[u8], sha: bool) -> [u8; 32] {
    if sha {
        chain_id(prev, payload)
    } else {
        legacy_id_chained(prev, payload)
    }
}

/// The flags cell of a record written now.
pub(crate) fn flags_for(derived: bool) -> i64 {
    FLAG_SHA256 | if derived { FLAG_NONCE_DERIVED } else { 0 }
}

/// A record's payload without its nonce: what every decoder reads. A legacy record's payload
/// is the whole of it. A flagged record shorter than its nonce is damaged and reads empty,
/// which every decoder refuses by name.
pub(crate) fn body_of(r: &Record) -> &[u8] {
    if r.flags & FLAG_SHA256 != 0 {
        r.payload.get(NONCE_LEN..).unwrap_or(&[])
    } else {
        &r.payload
    }
}

/// `nonce ‖ body`: the payload a writer stores.
pub(crate) fn with_nonce(nonce: &[u8; NONCE_LEN], body: &[u8]) -> Vec<u8> {
    let mut p = Vec::with_capacity(NONCE_LEN + body.len());
    p.extend_from_slice(nonce);
    p.extend_from_slice(body);
    p
}

/// Which scheme an id verifies under, or `None`. A record flagged sha256 must verify as
/// sha256 -- a flagged record whose id only matches FNV was edited and repaired under the
/// wrong scheme. An unflagged one is tried as legacy first; sha256 last, for a record whose
/// flag was stripped (a pointless edit: the id still holds) or that sits in a v1 store.
pub(crate) fn verify_id(flags: i64, prev: &[u8; 32], payload: &[u8], id: &[u8; 32]) -> Option<IdScheme> {
    if flags & FLAG_SHA256 != 0 {
        return (chain_id(prev, payload) == *id).then_some(IdScheme::Sha256);
    }
    if legacy_id_chained(prev, payload) == *id {
        Some(IdScheme::LegacyChained)
    } else if legacy_id(payload) == *id {
        Some(IdScheme::LegacyPayload)
    } else if chain_id(prev, payload) == *id {
        Some(IdScheme::Sha256)
    } else {
        None
    }
}

/// The scheme a record's id was found to verify under.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdScheme {
    /// `sha256(prev ‖ payload)`, M0.
    Sha256,
    /// FNV-1a x4 over `prev ‖ payload`: 2026-09-20 to M0.
    LegacyChained,
    /// FNV-1a x4 over the payload alone: before 2026-09-20.
    LegacyPayload,
}

/// The nonce source a log carries: the caller's seed, or nothing -- and THE SCHEME CHOICE AT
/// WRITE (W-MG-MIG, 2026-10-31): whether a record written now is sha256 (M0) or legacy.
#[derive(Debug, Clone)]
pub(crate) struct Entropy {
    seed: Option<[u8; 32]>,
    drawn: u64,
    /// What the caller asked for (`set_sha256`). The crate's default is sha256 -- what M0
    /// merged, and what every test in this crate was written against. The Worker turns it
    /// OFF per venue until that venue is flipped (`workers/api/src/scheme.rs`), so the first
    /// release that READS both schemes WRITES the old one: an expand/contract deploy whose
    /// rollback target (prod ac5ff0c4) still reads every byte it wrote.
    sha: bool,
}

impl Default for Entropy {
    fn default() -> Self {
        Entropy { seed: None, drawn: 0, sha: true }
    }
}

impl Entropy {
    pub(crate) fn set(&mut self, seed: [u8; 32]) {
        self.seed = Some(seed);
        self.drawn = 0;
    }

    /// Ask for sha256 (`true`) or legacy (`false`) records from now on. Asking is not deciding:
    /// `writes_sha256` has the last word.
    pub(crate) fn set_sha256(&mut self, on: bool) {
        self.sha = on;
    }

    /// THE SCHEME MOVES FORWARD ONCE, PER LOG. The caller's choice holds until the log's
    /// newest record is sha256; from then on the log writes sha256 whatever the caller says,
    /// because `walk_check` counts a legacy record NEWER than a sha256 one as `broken` (the
    /// forgery FNV lets through), and a rollback that wrote one would poison the chain for
    /// every later build. Measured 2026-10-31 (`docs/design/MIGRATION-sha256-ids.md` §1): two
    /// FNV records written by ac5ff0c4 after two sha256 ones read back `broken:2` for good.
    /// So a flip is one-way and lives in the bytes, not in a config; the config only starts it.
    /// Reads one record (the newest), never the chain.
    pub(crate) fn writes_sha256(&self, st: &Store) -> bool {
        self.sha || EvLog::walk_until(st, |_| true).first().is_some_and(|r| r.flags & FLAG_SHA256 != 0)
    }

    /// `(payload, id, flags)` for `body` after `prev`, under the scheme `writes_sha256` chose.
    /// Legacy is BYTE FOR BYTE what ac5ff0c4 writes: the body as the payload, FNV over
    /// `prev ‖ payload`, flags 0, no nonce drawn (the counter does not move). Sha256 is M0:
    /// `nonce ‖ body`, `sha256(prev ‖ payload)`, `FLAG_SHA256` (+ `FLAG_NONCE_DERIVED`).
    pub(crate) fn stamp(&mut self, sha: bool, prev: &[u8; 32], body: Vec<u8>) -> (Vec<u8>, [u8; 32], i64) {
        if !sha {
            let id = legacy_id_chained(prev, &body);
            return (body, id, 0);
        }
        let (nonce, derived) = self.nonce(prev, &body);
        let payload = with_nonce(&nonce, &body);
        let id = chain_id(prev, &payload);
        (payload, id, flags_for(derived))
    }

    /// The nonce for the record about to follow `prev`, and whether it was DERIVED (no
    /// seed: sha256 over a domain tag, `prev` and the body -- deterministic, so two logs
    /// written the same way still read the same, and protective of nothing).
    pub(crate) fn nonce(&mut self, prev: &[u8; 32], body: &[u8]) -> ([u8; NONCE_LEN], bool) {
        let (digest, derived) = match self.seed {
            Some(seed) => {
                self.drawn += 1;
                let mut m = Vec::with_capacity(32 + 8 + 32);
                m.extend_from_slice(&seed);
                m.extend_from_slice(&self.drawn.to_le_bytes());
                m.extend_from_slice(prev);
                (bebop_store::nodekey::sha256(&m), false)
            }
            None => {
                let mut m = Vec::with_capacity(20 + 32 + body.len());
                m.extend_from_slice(b"dowiz/nonce/derived");
                m.extend_from_slice(prev);
                m.extend_from_slice(body);
                (bebop_store::nodekey::sha256(&m), true)
            }
        };
        let text = crate::crypto::b64url_encode(&digest[..12]);
        let mut out = [0u8; NONCE_LEN];
        out.copy_from_slice(text.as_bytes());
        (out, derived)
    }
}

/// LEGACY: the id of a record written between the 2026-09-20 cascade and M0 -- four lanes of
/// FNV-1a over `prev ‖ payload`. Detection of an edit, not resistance to a forger: a payload
/// can be chosen to land on any FNV value. Kept to CHECK those records, never to write one.
pub(crate) fn legacy_id_chained(prev: &[u8; 32], payload: &[u8]) -> [u8; 32] {
    let mut buf = Vec::with_capacity(32 + payload.len());
    buf.extend_from_slice(prev);
    buf.extend_from_slice(payload);
    legacy_id(&buf)
}

/// LEGACY: the id of a record written before the 2026-09-20 cascade -- FNV-1a over the
/// payload alone, spread to 32 bytes over four lanes. Kept to check those records.
pub(crate) fn legacy_id(payload: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for lane in 0..4u64 {
        let mut h: u64 = 0xcbf29ce484222325 ^ lane.wrapping_mul(0x9E3779B97F4A7C15);
        for b in payload {
            h ^= *b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        out[lane as usize * 8..lane as usize * 8 + 8].copy_from_slice(&h.to_le_bytes());
    }
    out
}
