//! `breaker/replay.rs` — the golden replay-probe store (Blueprint A §2.3).
//!
//! Pairs captured while `invariant_verified == false` are **quarantined** and
//! never used to gate a Half-Open→Closed transition. The probe seam exists so
//! the Half-Open state has somewhere to read golden digests, but it ships
//! **disarmed**: `arm_truthfulness` is a no-op that writes `AuditKind::Disarm`
//! (per item-9 §7.3 / Blueprint A §4 — detreduce is a non-goal deferred to a
//! follow-on). Replay probes may only gate a close once `detreduce`'s
//! `verified_invariant()` is true; until then `truthfulness_fail` is masked to 0
//! and the probe pool is inert.
//!
//! Pure `std`, zero external dependencies.

use crate::event_log::sha3_256;

/// A golden input→output pair used as a replay probe. `key` is
/// `sha3(input_bytes ‖ condition_bytes)`; a probe passes iff the freshly
/// computed output digest equals `output_digest` — bitwise, no tolerance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GoldenPair {
    pub key: [u8; 32],
    pub input_digest: [u8; 32],
    pub output_digest: [u8; 32],
    pub captured_at_seq: u64,
    /// Was detreduce verified when this pair was captured? If false, the pair is
    /// quarantined and MUST NOT gate a close.
    pub invariant_verified: bool,
}

impl GoldenPair {
    /// Build a golden pair. `key = sha3(input ‖ condition)`.
    pub fn new(
        input_bytes: &[u8],
        condition_bytes: &[u8],
        output_bytes: &[u8],
        captured_at_seq: u64,
        invariant_verified: bool,
    ) -> Self {
        let key = {
            let mut buf = Vec::with_capacity(input_bytes.len() + condition_bytes.len());
            buf.extend_from_slice(input_bytes);
            buf.extend_from_slice(condition_bytes);
            sha3_256(&buf)
        };
        GoldenPair {
            key,
            input_digest: sha3_256(input_bytes),
            output_digest: sha3_256(output_bytes),
            captured_at_seq,
            invariant_verified,
        }
    }

    /// Does the freshly computed output match the golden digest, bitwise?
    pub fn probe_matches(&self, fresh_output: &[u8]) -> bool {
        self.output_digest == sha3_256(fresh_output)
    }
}

/// The golden replay store. The probe pool is **empty by default** (no captured
/// pairs ship). `arm_truthfulness` is a no-op writing `AuditKind::Disarm` until
/// detreduce lands — the seam exists, the corpus does not.
pub struct GoldenStore {
    pairs: Vec<GoldenPair>,
    armed: bool,
    disarm_event: Option<([u8; 32], u64)>, // (marker, seq) when disarmed
}

impl GoldenStore {
    /// An empty, disarmed store (the honest default for item-9).
    pub fn new() -> Self {
        GoldenStore {
            pairs: Vec::new(),
            armed: false,
            disarm_event: None,
        }
    }

    /// Add a captured golden pair. A pair with `invariant_verified == false` is
    /// still stored but **excluded from `probe_pool()`** (the safety property).
    pub fn add(&mut self, pair: GoldenPair) {
        self.pairs.push(pair);
    }

    /// The usable probe pool: ONLY pairs captured while the invariant was verified.
    /// Quarantined pairs (invariant_verified == false) are never returned.
    pub fn probe_pool(&self) -> Vec<&GoldenPair> {
        self.pairs
            .iter()
            .filter(|p| p.invariant_verified)
            .collect()
    }

    /// Whether truthfulness probing is armed (requires detreduce verification).
    pub fn is_armed(&self) -> bool {
        self.armed
    }

    /// Arm the truthfulness probe. In item-9 this is a **no-op**: detreduce is a
    /// deferred non-goal, so arming writes a `Disarm` marker instead of enabling
    /// the probe (Blueprint A §4). Returns `false` to signal "still disarmed".
    pub fn arm_truthfulness(&mut self, seq: u64) -> bool {
        // No-op: record the disarm state; the probe pool stays inert/empty.
        self.disarm_event = Some((sha3_256(b"disarm"), seq));
        self.armed = false;
        false
    }

    /// Run a fresh-output probe against the (armed, verified) pool. Returns
    /// `Ok(true)` only if EVERY armed pair matches bitwise. With an empty /
    /// disarmed pool, probing reports "no gate" (`None`) — it never fabricates a
    /// pass that would close the breaker on unverified evidence.
    pub fn probe_all(&self, fresh_output: &[u8]) -> Option<bool> {
        if !self.armed {
            return None; // disarmed ⇒ do not gate
        }
        let pool = self.probe_pool();
        if pool.is_empty() {
            return None; // no verified pairs ⇒ do not gate
        }
        Some(pool.iter().all(|p| p.probe_matches(fresh_output)))
    }
}

impl Default for GoldenStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quarantined_pair_excluded_from_probe_pool() {
        // RED-first against a "gate on any pair" impl: a pair with
        // invariant_verified==false MUST NOT enter the probe pool.
        let mut store = GoldenStore::new();
        store.add(GoldenPair::new(b"in", b"cond", b"out", 0, false));
        assert!(store.probe_pool().is_empty(), "quarantined pair must be excluded");
    }

    #[test]
    fn verified_pair_enters_probe_pool() {
        let mut store = GoldenStore::new();
        store.add(GoldenPair::new(b"in", b"cond", b"out", 0, true));
        assert_eq!(store.probe_pool().len(), 1);
    }

    #[test]
    fn arm_truthfulness_is_noop_disarmed() {
        let mut store = GoldenStore::new();
        store.add(GoldenPair::new(b"in", b"cond", b"out", 0, true));
        // Even with a verified pair present, arming must report disarmed (item-9
        // non-goal: detreduce not wired).
        assert!(!store.arm_truthfulness(1));
        assert!(!store.is_armed());
        // Probing therefore cannot gate a close.
        assert_eq!(store.probe_all(b"out"), None);
    }

    #[test]
    fn probe_matches_bitwise() {
        let p = GoldenPair::new(b"input", b"c", b"output", 0, true);
        assert!(p.probe_matches(b"output"));
        assert!(!p.probe_matches(b"outputX")); // bitwise, no tolerance
    }
}
