//! spool.rs — pure state machine for a crash-safe async work queue.
//!
//! ## Why (operator mandate 2026-07-15: system speed = slowest part)
//! Synchronous network/IO on the work critical path serializes the agent behind
//! the slowest edge. The fix is the proven telemetry pattern: the producer
//! appends ONE record (microseconds, never blocks) and returns; a background
//! drainer claims records in order, does the slow work, and ACKs them. A crash
//! mid-drain loses nothing — un-ACKed records stay and are retried.
//!
//! This module is the **pure** half: it owns the queue's in-memory state and the
//! deterministic transitions (append / claim-next / ack / compact), with NO file
//! or network I/O. The I/O adapter (JSONL file marshal + drainer) lives outside
//! the kernel (pure-std firewall), exactly like `telemetry-spool`. Keeping the
//! state machine here makes the queue's correctness Verified-by-Math.
//!
//! ## BIDIRECTIONAL contract
//! Producer → Spool (append, fire-and-forget). Spool → Consumer (claim, in
//! order). Consumer → Spool (ack). Spool → Producer (optional backpressure
//! signal: `is_full`). This is the kernel-native async channel reused by every
//! subsystem (reporting, governance events, mesh sync), not just Telegram.

use std::collections::VecDeque;

/// A queued record. `id` is a monotonic producer sequence (ordering key);
/// `payload` is opaque to the spool (the consumer interprets it).
#[derive(Debug, Clone, PartialEq)]
pub struct Record {
    pub id: u64,
    pub payload: String,
    /// Set when `claim_next` hands it to a consumer; cleared on `ack` or
    /// `reclaim_stuck`. Distinguishes "pending" from "in-flight".
    pub claimed: bool,
}

/// Crash-safe spool state. Pure: the adapter is responsible for persisting
/// `records` (e.g. as JSONL) and for calling `claim_next`/`ack` in lock-step.
#[derive(Debug, Clone, Default)]
pub struct Spool {
    /// FIFO ring of live (un-acked) records, front = oldest. Replaces `Vec<Record>`
    /// (P77 B1): `push_back` on append; `pop_front` on FIFO ack (O(1), no shift);
    /// order-preserving `remove(pos)` only on the rare out-of-order ack/drop.
    /// `Index<usize>` stays logical-front-relative == the old `Vec`, so the
    /// white-box `records[i]` assertions in the tests are unchanged.
    records: VecDeque<Record>,
    /// Claim cursor: the `records` index of the next `claim_next` candidate. All
    /// indices `< claim_cursor` are `claimed` (contiguous claimed prefix), so
    /// `claim_next` advances the cursor instead of re-scanning — O(1) amortized
    /// across a full drain. Rewinds on `reclaim` (CI-4), decrements on `pop_front`
    /// (CI-2) / order-preserving `remove` (CI-3). See AGENTS/blueprint P77 §3.1.
    claim_cursor: usize,
    next_id: u64,
    /// Backpressure watermark: when pending+claimed count reaches this, the
    /// producer is told to slow down (bidirectional flow control).
    capacity: usize,
}

impl Spool {
    pub fn new(capacity: usize) -> Self {
        Self {
            records: VecDeque::new(),
            claim_cursor: 0,
            next_id: 0,
            capacity: capacity.max(1),
        }
    }

    /// Number of records not yet acked (pending + in-flight).
    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Backpressure signal for the producer: true when the queue is at capacity.
    pub fn is_full(&self) -> bool {
        self.records.len() >= self.capacity
    }

    /// Append a record. Returns its id. Returns `None` (and drops the record)
    /// when backpressure is engaged — the producer must retry later. This is the
    /// bidirectional "slow down" signal that keeps the queue bounded under load.
    pub fn append(&mut self, payload: &str) -> Option<u64> {
        if self.is_full() {
            return None;
        }
        let id = self.next_id;
        self.next_id += 1;
        self.records.push_back(Record {
            id,
            payload: payload.to_string(),
            claimed: false,
        });
        Some(id)
    }

    /// Claim the lowest-id un-acked, un-claimed record (strict FIFO). The
    /// consumer is now responsible for it until `ack`. Returns a COPY so the
    /// spool can be reclaimed on crash without the consumer holding a ref.
    /// Returns `None` when empty.
    pub fn claim_next(&mut self) -> Option<Record> {
        // Advance the cursor past any already-claimed records (CI-1: a claimed
        // prefix sits before the cursor). O(1) amortized across a full drain —
        // the cursor only ever moves forward except when `reclaim` rewinds it.
        while self.claim_cursor < self.records.len() && self.records[self.claim_cursor].claimed {
            self.claim_cursor += 1;
        }
        if self.claim_cursor >= self.records.len() {
            return None;
        }
        let idx = self.claim_cursor;
        self.records[idx].claimed = true;
        Some(self.records[idx].clone())
    }

    /// ACK (remove) a record by id. After a successful consumer operation the
    /// record is gone — crash-safe only if the adapter persists AFTER this
    /// returns and BEFORE the side-effect is considered durable. Returns true
    /// if a record with `id` existed and was removed.
    pub fn ack(&mut self, id: u64) -> bool {
        // FIFO fast-path: the front record is the one being acked (the normal
        // drainer shape) ⇒ `pop_front` is O(1) with no trailing shift, so
        // draining N records is O(N) total instead of O(N²). (CI-2)
        if self.records.front().map(|r| r.id) == Some(id) {
            self.records.pop_front();
            self.claim_cursor = self.claim_cursor.saturating_sub(1);
            return true;
        }
        // Out-of-order ack (rare): order-preserving `remove` keeps FIFO order of
        // the survivors; rewind the cursor if we yanked a record before it (CI-3).
        if let Some(pos) = self.records.iter().position(|r| r.id == id) {
            self.records.remove(pos);
            if pos < self.claim_cursor {
                self.claim_cursor -= 1;
            }
            true
        } else {
            false
        }
    }

    /// Reclaim a record that was claimed but never acked (consumer crashed /
    /// timed out). Makes it claimable again. Returns true if it was reclaimed.
    pub fn reclaim(&mut self, id: u64) -> bool {
        // Find the record's position, un-claim it, and rewind the cursor so
        // `claim_next` re-serves it (crash recovery, CI-4). A scan is fine here:
        // reclaim is the crash-recovery path, not the drain hot loop.
        if let Some(pos) = self.records.iter().position(|r| r.id == id) {
            let r = &mut self.records[pos];
            if r.claimed {
                r.claimed = false;
                self.claim_cursor = self.claim_cursor.min(pos);
                return true;
            }
        }
        false
    }

    /// Compact: drop any residual claimed-but-not-acked records that the caller
    /// has decided to abandon (e.g. poison after N retries). Keeps ordering of
    /// the rest. Returns the count dropped.
    pub fn compact_drop(&mut self, id: u64) -> bool {
        // Same FIFO-fast-path / out-of-order-fallback removal shape as `ack`,
        // but without the crash-recovery claim semantics (just drops the record).
        if self.records.front().map(|r| r.id) == Some(id) {
            self.records.pop_front();
            self.claim_cursor = self.claim_cursor.saturating_sub(1);
            return true;
        }
        if let Some(pos) = self.records.iter().position(|r| r.id == id) {
            self.records.remove(pos);
            if pos < self.claim_cursor {
                self.claim_cursor -= 1;
            }
            true
        } else {
            false
        }
    }

    /// Peek pending (un-claimed) count — the consumer's work backlog.
    pub fn pending(&self) -> usize {
        self.records.iter().filter(|r| !r.claimed).count()
    }

    /// Peek in-flight (claimed, un-acked) count — the consumer's outstanding set.
    pub fn in_flight(&self) -> usize {
        self.records.iter().filter(|r| r.claimed).count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── GREEN: append returns sequential ids; len tracks them. ──
    #[test]
    fn append_sequential_ids_and_len() {
        let mut s = Spool::new(8);
        assert_eq!(s.append("a"), Some(0));
        assert_eq!(s.append("b"), Some(1));
        assert_eq!(s.append("c"), Some(2));
        assert_eq!(s.len(), 3);
    }

    // ── GREEN: strict FIFO claim, then ack removes in any order but keeps order
    //    of remaining. ──
    #[test]
    fn claim_fifo_then_ack() {
        let mut s = Spool::new(8);
        s.append("a");
        s.append("b");
        s.append("c");
        let r0 = s.claim_next().unwrap();
        assert_eq!(r0.id, 0);
        let r1 = s.claim_next().unwrap();
        assert_eq!(r1.id, 1);
        assert_eq!(s.in_flight(), 2);
        assert_eq!(s.pending(), 1);
        // ack the middle one first — remaining keep their ids/order.
        assert!(s.ack(1));
        assert_eq!(s.len(), 2);
        assert_eq!(s.records[0].id, 0);
        assert_eq!(s.records[1].id, 2);
        assert!(s.ack(0));
        assert!(s.ack(2));
        assert!(s.is_empty());
    }

    // ── GREEN: backpressure — once at capacity, append returns None (drop,
    //    producer must retry). is_full flips exactly at the watermark. ──
    #[test]
    fn backpressure_at_capacity() {
        let mut s = Spool::new(2);
        assert!(!s.is_full());
        assert_eq!(s.append("a"), Some(0));
        assert_eq!(s.append("b"), Some(1));
        assert!(s.is_full());
        assert_eq!(s.append("c"), None); // rejected, NOT silently queued
        assert_eq!(s.len(), 2);
    }

    // ── GREEN (crash-safety): a claimed-but-unacked record can be reclaimed and
    //    re-claimed after a consumer crash — no record lost, no double-claim. ──
    #[test]
    fn crash_reclaim_recovers_inflight() {
        let mut s = Spool::new(8);
        s.append("a");
        s.append("b");
        let r = s.claim_next().unwrap();
        assert_eq!(r.id, 0);
        assert_eq!(s.in_flight(), 1);
        // consumer crashes before ack → reclaimer (drainer restart) frees it.
        assert!(s.reclaim(0));
        assert_eq!(s.in_flight(), 0);
        let again = s.claim_next().unwrap();
        assert_eq!(again.id, 0); // same record re-claimed, not skipped
        assert!(s.ack(0));
        assert_eq!(s.len(), 1);
    }

    // ── GREEN: ack of an unknown id is a no-op (idempotent, fail-closed). ──
    #[test]
    fn ack_unknown_is_noop() {
        let mut s = Spool::new(8);
        assert!(!s.ack(99));
        s.append("x");
        assert!(!s.ack(42));
        assert_eq!(s.len(), 1);
    }

    // ── GREEN (bidirectional): capacity=1 forces producer/consumer handoff
    //    — every append must be acked before the next is accepted. ──
    #[test]
    fn capacity_one_handshake() {
        let mut s = Spool::new(1);
        assert_eq!(s.append("only"), Some(0));
        assert_eq!(s.append("blocked"), None); // full
        let r = s.claim_next().unwrap();
        assert_eq!(r.id, 0);
        assert!(s.ack(0));
        assert_eq!(s.append("now"), Some(1)); // accepted after drain
        assert_eq!(s.len(), 1);
    }

    // ── GREEN (P77 B1 differential): new VecDeque+claim-cursor impl must be
    //    byte-identical to the original Vec+position+remove impl across a random
    //    op sequence. A reference model re-implements the OLD semantics; any
    //    cursor desync surfaces as a divergent claim/ack/len observation. ──
    #[test]
    fn spool_equiv_random_ops() {
        // Deterministic LCG so the sequence is reproducible.
        let mut seed: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut rng = || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            seed >> 33
        };

        // Reference model: the OLD spool semantics on a plain Vec (must match).
        struct Ref {
            recs: Vec<Record>,
            next_id: u64,
            cap: usize,
        }
        impl Ref {
            fn new(cap: usize) -> Self {
                Ref {
                    recs: Vec::new(),
                    next_id: 0,
                    cap: cap.max(1),
                }
            }
            fn append(&mut self, p: &str) -> Option<u64> {
                if self.recs.len() >= self.cap {
                    return None;
                }
                let id = self.next_id;
                self.next_id += 1;
                self.recs.push(Record {
                    id,
                    payload: p.to_string(),
                    claimed: false,
                });
                Some(id)
            }
            fn claim_next(&mut self) -> Option<u64> {
                let idx = self.recs.iter().position(|r| !r.claimed)?;
                self.recs[idx].claimed = true;
                Some(self.recs[idx].id)
            }
            fn ack(&mut self, id: u64) -> bool {
                if let Some(pos) = self.recs.iter().position(|r| r.id == id) {
                    self.recs.remove(pos);
                    true
                } else {
                    false
                }
            }
            fn reclaim(&mut self, id: u64) -> bool {
                if let Some(r) = self.recs.iter_mut().find(|r| r.id == id) {
                    if r.claimed {
                        r.claimed = false;
                        return true;
                    }
                }
                false
            }
            fn compact_drop(&mut self, id: u64) -> bool {
                if let Some(pos) = self.recs.iter().position(|r| r.id == id) {
                    self.recs.remove(pos);
                    true
                } else {
                    false
                }
            }
            fn len(&self) -> usize {
                self.recs.len()
            }
            fn pending(&self) -> usize {
                self.recs.iter().filter(|r| !r.claimed).count()
            }
            fn in_flight(&self) -> usize {
                self.recs.iter().filter(|r| r.claimed).count()
            }
            fn is_full(&self) -> bool {
                self.recs.len() >= self.cap
            }
        }

        let mut new = Spool::new(64);
        let mut old = Ref::new(64);
        for _ in 0..5000 {
            match rng() % 5 {
                0 => {
                    let p = format!("p{}", rng());
                    assert_eq!(new.append(&p), old.append(&p), "append must agree");
                }
                1 => {
                    let n = new.claim_next().map(|r| r.id);
                    let o = old.claim_next();
                    assert_eq!(n, o, "claim_next must agree");
                }
                2 => {
                    let id = rng() % 64;
                    assert_eq!(new.ack(id), old.ack(id), "ack({id}) must agree");
                }
                3 => {
                    let id = rng() % 64;
                    assert_eq!(new.reclaim(id), old.reclaim(id), "reclaim({id}) must agree");
                }
                _ => {
                    let id = rng() % 64;
                    assert_eq!(
                        new.compact_drop(id),
                        old.compact_drop(id),
                        "compact_drop({id}) must agree"
                    );
                }
            }
            assert_eq!(new.len(), old.len(), "len must agree");
            assert_eq!(new.pending(), old.pending(), "pending must agree");
            assert_eq!(new.in_flight(), old.in_flight(), "in_flight must agree");
            assert_eq!(new.is_full(), old.is_full(), "is_full must agree");
        }
    }

    // ── GREEN (P77 B1): out-of-order ack keeps surviving order + cursor sound. ──
    #[test]
    fn spool_ack_out_of_order_preserves_order() {
        let mut s = Spool::new(8);
        s.append("a");
        s.append("b");
        s.append("c");
        let r0 = s.claim_next().unwrap();
        let r1 = s.claim_next().unwrap();
        let r2 = s.claim_next().unwrap();
        assert_eq!((r0.id, r1.id, r2.id), (0, 1, 2));
        assert!(s.ack(1)); // middle removed first
        assert_eq!(s.records[0].id, 0);
        assert_eq!(s.records[1].id, 2);
        assert!(s.ack(0));
        assert!(s.ack(2));
        assert!(s.is_empty());
    }

    // ── GREEN (P77 B1): reclaim rewinds the cursor so the same id re-claims. ──
    #[test]
    fn spool_reclaim_rewinds_cursor() {
        let mut s = Spool::new(8);
        s.append("a");
        let r = s.claim_next().unwrap();
        assert_eq!(r.id, 0);
        assert!(s.reclaim(0));
        let again = s.claim_next().unwrap();
        assert_eq!(again.id, 0);
        assert!(s.ack(0));
    }
}
