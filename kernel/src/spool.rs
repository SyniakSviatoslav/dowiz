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
    records: Vec<Record>,
    next_id: u64,
    /// Backpressure watermark: when pending+claimed count reaches this, the
    /// producer is told to slow down (bidirectional flow control).
    capacity: usize,
}

impl Spool {
    pub fn new(capacity: usize) -> Self {
        Self {
            records: Vec::new(),
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
        self.records.push(Record {
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
        let idx = self.records.iter().position(|r| !r.claimed)?;
        self.records[idx].claimed = true;
        Some(self.records[idx].clone())
    }

    /// ACK (remove) a record by id. After a successful consumer operation the
    /// record is gone — crash-safe only if the adapter persists AFTER this
    /// returns and BEFORE the side-effect is considered durable. Returns true
    /// if a record with `id` existed and was removed.
    pub fn ack(&mut self, id: u64) -> bool {
        if let Some(pos) = self.records.iter().position(|r| r.id == id) {
            self.records.remove(pos);
            true
        } else {
            false
        }
    }

    /// Reclaim a record that was claimed but never acked (consumer crashed /
    /// timed out). Makes it claimable again. Returns true if it was reclaimed.
    pub fn reclaim(&mut self, id: u64) -> bool {
        if let Some(r) = self.records.iter_mut().find(|r| r.id == id) {
            if r.claimed {
                r.claimed = false;
                return true;
            }
        }
        false
    }

    /// Compact: drop any residual claimed-but-not-acked records that the caller
    /// has decided to abandon (e.g. poison after N retries). Keeps ordering of
    /// the rest. Returns the count dropped.
    pub fn compact_drop(&mut self, id: u64) -> bool {
        if let Some(pos) = self.records.iter().position(|r| r.id == id) {
            self.records.remove(pos);
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
}
