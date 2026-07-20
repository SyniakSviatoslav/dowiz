//! Item 5 (regret-min synthesis §5.2, "build real cross-mesh data replication
//! now, not the interim single-node option") — MESH-07 parity, native and
//! zero-dep, over this crate's own [`EventStore`]/[`MeshEvent`] types.
//!
//! Design reference (NOT a dependency — 2026-07-19 operator ruling,
//! `SPACE-GRADE-KERNEL-EXECUTION-ROADMAP-2026-07-19.md` §0: "REIMPLEMENT IN
//! DOWIZ, ZERO-DEP — bebop's proven mesh-node/proto-wire/proto-cap serves as
//! design reference/parity oracle only, not a linked dependency," a ruling
//! that names `mesh-adapter`'s sibling path-deps by number): bebop2's
//! `proto-wire/src/sync_pull.rs` `MerkleLog` / `SyncPeer` / `PullRequest` /
//! `IngestResult`. Hand-reimplemented here against `dowiz_kernel::event_log`'s
//! own types — not imported, no path dependency.
//!
//! # The mechanism
//!
//! Two nodes that diverged while offline reconnect and run a *pull*: each
//! computes the other's [`MerkleLog`] root over its content-ids. A matching
//! root is a cheap proof of convergence — no data has to move. A differing
//! root triggers a pull: the requester builds a [`PullRequest`] (its
//! per-actor watermark — the highest `actor_seq` it has already folded for
//! each actor), the peer answers with [`pull`] (every event whose
//! `actor_seq` is past that watermark), and the requester folds the answer
//! with [`ingest`] — idempotent by content-id, since `dowiz-kernel` events
//! are already content-addressed (`MeshEvent::event_id`). A replayed event is
//! therefore a structural no-op: the merged id-set is a signed G-Set CvRDT
//! (union is commutative, associative, idempotent), so repeated or
//! out-of-order pulls always converge to the same folded set regardless of
//! network ordering or retries — the [`reconcile`] tests below prove this
//! directly against the MESH-07 RED-test criterion: two nodes diverge
//! offline, reconnect, pull, and land on an *identical folded event set*.
//!
//! # What this module is NOT
//!
//! Transport — how bytes actually move node-to-node — is explicitly out of
//! scope, the same anti-scope `mesh-adapter/src/lib.rs` already states ("no
//! transport, no storage"). This module is the pure, synchronous, std-only
//! reconciliation ALGORITHM: feed it two [`EventStore`]s — in-process, over a
//! socket, over a file drop, the algorithm does not care — and it proves
//! convergence. Wiring it to a real socket/QUIC transport is a separate,
//! later port (consistent with this crate's existing ports/adapters split —
//! `ports::tool`, `ports::payment`, `mesh-adapter` itself); async I/O has no
//! place in the kernel's deterministic, sync-only core (MANIFESTO C2).
//!
//! Signature verification is also out of scope here — a deliberate split
//! `crate::event_log::EventLog`'s own doc already names ("the network layer
//! never re-runs decide — it only verifies signatures"). [`ingest`] operates
//! on [`EventStore`] directly, bypassing `EventLog::append`'s local-first
//! `prev`-rebinding (a synced event carries its own `prev` from its origin
//! node and must be stored verbatim, never re-chained onto this node's tip).
//! Verifying a synced event's signature before it reaches [`ingest`] is the
//! caller's job (`crate::mesh`'s signed-log primitive, `pq` feature).
//!
//! Egress-free: no network, no external crate, `std` only.

use std::collections::HashMap;

use crate::event_log::{sha3_256, EventStore, MeshEvent, StoreError};

/// A Merkle digest of an event-log's content-ids. Leaves are the sorted set
/// of ids; the root is a recursive pair-hash (`sha3_256(left || right)`, an
/// odd final leaf pairs with itself). Empty log => zero root. Two nodes
/// holding the same folded set produce the same root regardless of insertion
/// order — a matching root is a cheap proof of convergence.
#[derive(Debug, Clone, Default)]
pub struct MerkleLog {
    leaves: Vec<[u8; 32]>,
    seen: std::collections::HashSet<[u8; 32]>,
}

impl MerkleLog {
    /// Empty digest.
    pub fn new() -> Self {
        MerkleLog::default()
    }

    /// Whether `id` is already in the digest (content-addressed dedup).
    pub fn contains(&self, id: &[u8; 32]) -> bool {
        self.seen.contains(id)
    }

    /// Number of leaves.
    pub fn len(&self) -> usize {
        self.leaves.len()
    }

    /// Whether empty.
    pub fn is_empty(&self) -> bool {
        self.leaves.is_empty()
    }

    /// Add a content-id (idempotent: a duplicate does not change the
    /// set/root).
    pub fn add(&mut self, id: [u8; 32]) {
        if self.seen.insert(id) {
            self.leaves.push(id);
            self.leaves.sort_unstable();
        }
    }

    /// Current Merkle root. Stable for a given set of leaves regardless of
    /// the order they were `add`ed in.
    pub fn root(&self) -> [u8; 32] {
        if self.leaves.is_empty() {
            return [0u8; 32];
        }
        let mut level: Vec<[u8; 32]> = self.leaves.clone();
        while level.len() > 1 {
            let mut next = Vec::with_capacity(level.len().div_ceil(2));
            let mut i = 0;
            while i < level.len() {
                let left = level[i];
                let right = if i + 1 < level.len() {
                    level[i + 1]
                } else {
                    level[i] // odd leaf pairs with itself
                };
                let mut buf = Vec::with_capacity(64);
                buf.extend_from_slice(&left);
                buf.extend_from_slice(&right);
                next.push(sha3_256(&buf));
                i += 2;
            }
            level = next;
        }
        level[0]
    }

    /// Build a digest from any [`EventStore`]'s current content-ids
    /// (`EventStore::ids`).
    pub fn from_store<S: EventStore>(store: &S) -> Self {
        let mut log = MerkleLog::new();
        for id in store.ids() {
            log.add(id);
        }
        log
    }
}

/// A pull request: the requester's per-actor watermark. A peer answers with
/// every event whose `actor_seq` is strictly greater than the requester's
/// recorded `last_seq` for that actor (0 — "ask for everything" — if the
/// actor is unknown to the requester).
#[derive(Debug, Clone, Default)]
pub struct PullRequest {
    pub watermark: HashMap<[u8; 32], u64>,
}

impl PullRequest {
    /// Empty request — asks for everything.
    pub fn new() -> Self {
        PullRequest::default()
    }

    /// Set the watermark for one actor.
    pub fn with_watermark(mut self, actor: [u8; 32], last_seq: u64) -> Self {
        self.watermark.insert(actor, last_seq);
        self
    }

    /// Build a request reflecting a store's current per-actor watermark (the
    /// max `actor_seq` folded so far for each actor). Requires
    /// [`EventStore::get`] (the default `None` degrades this to "actor
    /// unknown" for every event, i.e. an effectively empty request — never a
    /// false/stale watermark).
    pub fn from_store<S: EventStore>(store: &S) -> Self {
        let mut watermark: HashMap<[u8; 32], u64> = HashMap::new();
        for id in store.ids() {
            if let Some(ev) = store.get(&id) {
                let e = watermark.entry(ev.actor_pubkey).or_insert(0);
                if ev.actor_seq > *e {
                    *e = ev.actor_seq;
                }
            }
        }
        PullRequest { watermark }
    }
}

/// Outcome of folding a batch of pulled events into a store.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct IngestResult {
    /// New events folded into the store.
    pub added: usize,
    /// Events already present (content-id dup) — idempotent no-op.
    pub dup: usize,
}

/// Answer a pull request against a source store: every event whose
/// `actor_seq` is past the requester's watermark for that actor. Requires
/// [`EventStore::get`] — a store that cannot read bodies back answers empty
/// (degrades closed: it never fabricates an event it cannot prove it holds).
pub fn pull<S: EventStore>(source: &S, req: &PullRequest) -> Vec<MeshEvent> {
    let mut out = Vec::new();
    for id in source.ids() {
        if let Some(ev) = source.get(&id) {
            let last = req.watermark.get(&ev.actor_pubkey).copied().unwrap_or(0);
            if ev.actor_seq > last {
                out.push(ev);
            }
        }
    }
    out
}

/// Fold a batch of pulled events into a target store. Idempotent by
/// content-id (a duplicate is a no-op, matching [`EventStore::insert`]'s own
/// contract) — this is the G-Set CvRDT merge: the union of two
/// content-addressed id sets is commutative, associative, and idempotent, so
/// repeated or out-of-order ingests always converge to the same folded set.
///
/// A store fault on any single insert propagates immediately (H1: no silent
/// partial success). Retry is safe: every event already landed before the
/// fault is a `dup` on the retry, never re-applied.
pub fn ingest<S: EventStore>(
    target: &mut S,
    events: &[MeshEvent],
) -> Result<IngestResult, StoreError> {
    let mut res = IngestResult::default();
    for ev in events {
        let id = ev.event_id();
        if target.contains(&id) {
            res.dup += 1;
            continue;
        }
        target.insert(id, ev.clone())?;
        res.added += 1;
    }
    Ok(res)
}

/// One full reconciliation round: pull whatever `local` does not yet have
/// from `remote`, then fold it in. Matching [`MerkleLog`] roots before and
/// after is the cheap "did anything change" check; the stronger property
/// this proves is convergence of the actual folded event SET (every
/// content-id present on `remote` is now present on `local` too).
pub fn reconcile<S: EventStore>(local: &mut S, remote: &S) -> Result<IngestResult, StoreError> {
    let req = PullRequest::from_store(local);
    let events = pull(remote, &req);
    ingest(local, &events)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_log::MemEventStore;

    fn ev(actor: u8, seq: u64, payload: &[u8]) -> MeshEvent {
        MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: [actor; 32],
            actor_seq: seq,
            payload: payload.to_vec(),
        }
    }

    // ── MerkleLog — parity with bebop2's own MerkleLog test names/intent ──

    #[test]
    fn merkle_root_is_set_stable_and_empty_is_zero() {
        let mut a = MerkleLog::new();
        let mut b = MerkleLog::new();
        assert_eq!(a.root(), [0u8; 32], "empty root is zero");

        let ids = [[1u8; 32], [2u8; 32], [3u8; 32]];
        for id in &ids {
            a.add(*id);
        }
        let mut perm = ids;
        perm.reverse();
        for id in &perm {
            b.add(*id);
        }
        assert_eq!(a.root(), b.root(), "root is order-independent");
        assert_eq!(a.len(), 3);
    }

    #[test]
    fn merkle_root_changes_when_set_changes() {
        let mut a = MerkleLog::new();
        a.add([1u8; 32]);
        a.add([2u8; 32]);
        let r1 = a.root();
        a.add([3u8; 32]);
        assert_ne!(r1, a.root(), "adding a leaf changes the root");
    }

    #[test]
    fn merkle_add_is_idempotent() {
        let mut a = MerkleLog::new();
        a.add([9u8; 32]);
        let r1 = a.root();
        a.add([9u8; 32]); // duplicate
        assert_eq!(a.len(), 1, "duplicate add does not grow the leaf set");
        assert_eq!(r1, a.root(), "duplicate add does not change the root");
    }

    #[test]
    fn merkle_odd_leaf_count_pairs_with_itself_and_stays_deterministic() {
        let mut a = MerkleLog::new();
        for i in 0..5u8 {
            a.add([i; 32]);
        }
        let r1 = a.root();
        let mut b = MerkleLog::new();
        for i in (0..5u8).rev() {
            b.add([i; 32]);
        }
        assert_eq!(r1, b.root(), "odd-length tree is still order-independent");
    }

    #[test]
    fn merkle_from_store_matches_manual_digest() {
        let mut store = MemEventStore::new();
        let e1 = ev(1, 1, b"a");
        let e2 = ev(1, 2, b"b");
        store.insert(e1.event_id(), e1.clone()).unwrap();
        store.insert(e2.event_id(), e2.clone()).unwrap();

        let mut manual = MerkleLog::new();
        manual.add(e1.event_id());
        manual.add(e2.event_id());

        assert_eq!(MerkleLog::from_store(&store).root(), manual.root());
    }

    // ── PullRequest / pull / ingest ──

    #[test]
    fn pull_request_from_store_reflects_max_actor_seq() {
        let mut store = MemEventStore::new();
        for seq in [1u64, 3, 2] {
            let e = ev(7, seq, b"x");
            store.insert(e.event_id(), e).unwrap();
        }
        let req = PullRequest::from_store(&store);
        assert_eq!(
            req.watermark.get(&[7u8; 32]),
            Some(&3),
            "max seq, not last-inserted"
        );
    }

    #[test]
    fn pull_answers_only_events_past_the_watermark() {
        let mut store = MemEventStore::new();
        for seq in 1..=5u64 {
            let e = ev(1, seq, b"x");
            store.insert(e.event_id(), e).unwrap();
        }
        let req = PullRequest::new().with_watermark([1u8; 32], 3);
        let answered = pull(&store, &req);
        assert_eq!(answered.len(), 2, "only seq 4 and 5 are past watermark 3");
        assert!(answered.iter().all(|e| e.actor_seq > 3));
    }

    #[test]
    fn ingest_is_idempotent_on_replay() {
        let mut target = MemEventStore::new();
        let batch = vec![ev(2, 1, b"a"), ev(2, 2, b"b")];
        let r1 = ingest(&mut target, &batch).unwrap();
        assert_eq!(r1, IngestResult { added: 2, dup: 0 });

        // Replay the exact same batch — must be a pure no-op (dup), not a
        // second append.
        let r2 = ingest(&mut target, &batch).unwrap();
        assert_eq!(r2, IngestResult { added: 0, dup: 2 });
        assert_eq!(target.len(), 2, "replay never grows the store");
    }

    // ── reconcile — the MESH-07 RED-test criterion itself ──

    /// Two nodes diverge while offline (each authors events the other never
    /// sees), then reconnect and reconcile. Asserts the RED-test criterion
    /// named by MESH-07's own blueprint verbatim: identical folded event set
    /// on both sides afterward — not just a matching Merkle root.
    #[test]
    fn two_nodes_diverge_offline_reconnect_pull_identical_folded_state() {
        let mut node_a = MemEventStore::new();
        let mut node_b = MemEventStore::new();

        // Shared history before the two nodes went offline.
        let shared = ev(0, 1, b"shared-genesis");
        node_a.insert(shared.event_id(), shared.clone()).unwrap();
        node_b.insert(shared.event_id(), shared.clone()).unwrap();

        // Diverge: A authors two events actor-1 never told B about; B
        // authors one event actor-2 never told A about. Classic offline
        // split-brain.
        let a_only_1 = ev(1, 1, b"a-authored-1");
        let a_only_2 = ev(1, 2, b"a-authored-2");
        node_a
            .insert(a_only_1.event_id(), a_only_1.clone())
            .unwrap();
        node_a
            .insert(a_only_2.event_id(), a_only_2.clone())
            .unwrap();

        let b_only = ev(2, 1, b"b-authored-1");
        node_b.insert(b_only.event_id(), b_only.clone()).unwrap();

        assert_ne!(
            MerkleLog::from_store(&node_a).root(),
            MerkleLog::from_store(&node_b).root(),
            "diverged nodes must NOT already agree — sanity check on the test setup"
        );

        // Reconnect: bidirectional reconcile (A pulls from B, then B pulls
        // from A) — the real-world sequence, since either side may initiate.
        reconcile(&mut node_a, &node_b).expect("A pulls from B");
        reconcile(&mut node_b, &node_a).expect("B pulls from A");

        let root_a = MerkleLog::from_store(&node_a).root();
        let root_b = MerkleLog::from_store(&node_b).root();
        assert_eq!(root_a, root_b, "Merkle roots converge after reconnect");

        let mut ids_a = node_a.ids();
        let mut ids_b = node_b.ids();
        ids_a.sort_unstable();
        ids_b.sort_unstable();
        assert_eq!(ids_a, ids_b, "identical folded event set on both sides");
        assert_eq!(node_a.len(), 4, "genesis + 2 from A + 1 from B");
        assert_eq!(node_b.len(), 4);

        // A third, never-before-seen reconcile is a pure no-op — convergence
        // is stable, not a one-shot coincidence.
        let noop = reconcile(&mut node_a, &node_b).unwrap();
        assert_eq!(
            noop,
            IngestResult { added: 0, dup: 0 },
            "already-converged reconcile pulls nothing new"
        );
    }

    /// Convergence holds regardless of which side initiates first (order of
    /// the two `reconcile` calls does not matter) — required for a real
    /// G-Set CvRDT claim, not just a happy-path check.
    #[test]
    fn reconcile_converges_regardless_of_initiator_order() {
        let build = || {
            let mut a = MemEventStore::new();
            let mut b = MemEventStore::new();
            let a_ev = ev(1, 1, b"a");
            let b_ev = ev(2, 1, b"b");
            a.insert(a_ev.event_id(), a_ev).unwrap();
            b.insert(b_ev.event_id(), b_ev).unwrap();
            (a, b)
        };

        let (mut a1, mut b1) = build();
        reconcile(&mut a1, &b1).unwrap();
        reconcile(&mut b1, &a1).unwrap();

        let (mut b2, mut a2) = {
            let (a, b) = build();
            (b, a)
        };
        reconcile(&mut b2, &a2).unwrap();
        reconcile(&mut a2, &b2).unwrap();

        let mut ids1 = a1.ids();
        ids1.sort_unstable();
        let mut ids2 = a2.ids();
        ids2.sort_unstable();
        assert_eq!(ids1, ids2, "convergence is independent of initiator order");
        assert_eq!(
            MerkleLog::from_store(&a1).root(),
            MerkleLog::from_store(&b2).root()
        );
    }
}
