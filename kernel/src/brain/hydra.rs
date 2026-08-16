//! hydra — std host shim.
//!
//! The pure drift-gated supervisor core (`TopoEdge`, `candidate_drift`,
//! `INTEGRITY_BAND`, `Hydra<S: EventStore>`) lives in `dowiz_core::hydra`.
//! `FileEventStore` (the std-fs durable append-only store) stays here.

pub use dowiz_core::hydra::*;

use crate::event_log::{EventStore, MeshEvent, StoreError};

/// G4 — std-only durable append-only event store (Воля АНУ closed loop).
///
/// No external DB (sqlx/pgrust offline-uncached). Persists each event as one
/// JSON line to a local file; `insert` appends + `fsync`s (crash-safe). On open,
/// the file is replayed into an in-memory id/tip index so `contains`/`get`/`tip`
/// are O(1). The log is content-addressed and idempotent — a re-inserted id is
/// a no-op. Egress-free: only `std::fs` local IO, no network.
///
/// innovate: this is the durable variant that replaces MemEventStore for the
/// organism's persistent memory. pgrust remains the node-level SQL option; this
/// is the kernel-internal, dependency-free, offline-safe default for the Hydra.
use alloc::collections::BTreeMap;
use std::path::Path;

use crate::vfs::{open_file, OpenMode, StdFile, VfsFile};

pub struct FileEventStore {
    path: std::path::PathBuf,
    by_id: BTreeMap<[u8; 32], MeshEvent>,
    tip: Option<[u8; 32]>,
    count: usize,
    /// Item 61 (gap G5): durability continuous counters — running totals the
    /// group-commit decision can read as a *live* data feed (replacing the one-time
    /// ~637 µs bench number). `events_appended` counts every durable commit (the
    /// `sync_all` path reached); `fsync_calls` counts the `sync_all` calls actually
    /// issued. Both are P3-plane only — they NEVER gate the durability barrier in
    /// `insert` (the `?` short-circuits before any decision logic) and feed no
    /// hash/gate/replay surface (grep-firewall proof, blueprint §4).
    events_appended: u64,
    /// Count of `sync_all` calls issued (the `FileEventStore` `insert` is the only
    /// issuer — `MemEventStore` is in-memory and issues none).
    fsync_calls: u64,
    /// Cached write handle, opened LAZILY on the first successful `insert` —
    /// never at construction. `open()` must keep tolerating a not-yet-existing
    /// file/parent (H1 §4 criterion 4, tested: `file_store_open_failure_surfaces_not_swallowed`
    /// asserts an open failure surfaces via `insert`'s `Err`, not construction).
    /// Reused on every later insert — removes the redundant per-event
    /// open+close syscalls the item-26 audit measured (`strace`: 1 open + 1
    /// write + 1 close per event, only the fsync is load-bearing). This half
    /// is contract-neutral: it changes zero durability semantics, only removes
    /// redundant syscalls around the same barrier.
    handle: Option<StdFile>,
    /// Item 26 group-commit: `sync_all` fires every `batch_size` inserts
    /// instead of every one. **OFF by default** (`batch_size = 1` — today's
    /// exact per-event-fsync behavior, byte-for-byte unchanged: with
    /// `batch_size = 1`, `pending_since_sync` reaches the threshold on every
    /// single insert, so the barrier fires every time, exactly as before).
    /// Opt in via [`FileEventStore::with_batch_size`] — never a silent default
    /// change, per the item-26 audit's own recommendation
    /// (`AUDIT-ITEM-26-batching-measurements-2026-07-19.md` §1: "file a design
    /// proposal for an *opt-in* group-commit mode... do NOT silently change
    /// the default"). Measured throughput at `batch_size = 64`: ~53x (1,513 →
    /// ~93,000 events/s) — see the same audit.
    batch_size: usize,
    /// Writes since the last `sync_all` — the current unsynced batch. Always
    /// 0 immediately after `insert` returns when `batch_size == 1`.
    pending_since_sync: usize,
}

impl FileEventStore {
    /// Open (or create) the append-only log at `path`. Replays existing lines
    /// into the in-memory index. Corrupt/short lines are skipped (forward-
    /// tolerant) — the chain tip is the last *valid* committed event.
    pub fn open<P: AsRef<Path>>(path: P) -> std::io::Result<Self> {
        let path = path.as_ref().to_path_buf();
        let mut by_id = BTreeMap::new();
        let mut tip = None;
        let mut count = 0;
        if path.exists() {
            let text = crate::vfs::read_to_string(&path)?;
            for line in text.lines() {
                if line.is_empty() {
                    continue;
                }
                match serde_json_like_parse(line) {
                    Some(ev) => {
                        let id = ev.event_id();
                        by_id.insert(id, ev.clone());
                        tip = Some(id);
                        count += 1;
                    }
                    None => continue, // skip corrupt line
                }
            }
        }
        Ok(FileEventStore {
            path,
            by_id,
            tip,
            count,
            events_appended: 0,
            fsync_calls: 0,
            handle: None,
            batch_size: 1,
            pending_since_sync: 0,
        })
    }

    /// Opt in to group-commit: `sync_all` fires every `n` inserts instead of
    /// every one. `n = 1` (the default) is today's exact per-event durability,
    /// unchanged. `n > 1` is a REAL durability-contract change — see the
    /// struct-level doc and the module's crash-consistency discussion: up to
    /// `n - 1` acknowledged (`insert` returned `Ok`) events can be lost on a
    /// crash before their batch's `sync_all` fires. Panics if called after any
    /// insert has already buffered an unsynced write, so the barrier cadence
    /// can never change silently mid-stream.
    pub fn with_batch_size(mut self, n: usize) -> Self {
        assert!(n >= 1, "batch_size must be >= 1");
        assert_eq!(
            self.pending_since_sync, 0,
            "with_batch_size must be set before any insert (or right after a flush_pending)"
        );
        self.batch_size = n;
        self
    }

    /// Force a durability barrier now, flushing whatever group-commit has
    /// buffered. Callers doing a graceful shutdown with `batch_size > 1` MUST
    /// call this before exiting — relying on `Drop` is NOT safe (`Drop` cannot
    /// propagate an `fsync` error, and the kernel never swallows one silently).
    /// A no-op returning `Ok(())` if nothing is pending (including the
    /// `batch_size == 1` case, where nothing is ever left pending).
    pub fn flush_pending(&mut self) -> Result<(), StoreError> {
        if self.pending_since_sync == 0 {
            return Ok(());
        }
        let f = self
            .handle
            .as_mut()
            .expect("pending_since_sync > 0 implies handle is Some (set on first write)");
        f.sync_all().map_err(|e| StoreError::Sync(e.to_string()))?;
        self.fsync_calls += 1;
        self.pending_since_sync = 0;
        Ok(())
    }

    /// Item 61 (gap G5): live durability counters — the number of events durably
    /// committed and the number of `sync_all` calls issued so far. Readable without
    /// touching the durability barrier; P3 telemetry only.
    pub fn durability_counters(&self) -> DurabilityCounters {
        DurabilityCounters {
            events_appended: self.events_appended,
            fsync_calls: self.fsync_calls,
            pending_unsynced: self.pending_since_sync as u64,
        }
    }
}

/// Item 61 (gap G5): live durability counters (P3 telemetry). Named fields so callers
/// read intent, never tuple-position guessing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DurabilityCounters {
    /// Number of events durably committed (appended + fsynced) so far.
    pub events_appended: u64,
    /// Number of `sync_all` durability barriers issued so far.
    pub fsync_calls: u64,
    /// Item 26 group-commit: events written and index-acknowledged but not yet
    /// covered by a `sync_all` — the live risk window a crash right now would
    /// lose. Always 0 with the default `batch_size = 1`.
    pub pending_unsynced: u64,
}

/// Minimal hand-rolled JSON parse for the 4 MeshEvent fields (std-only, no
/// serde dependency). Tolerates the exact shape we emit; returns None on
/// mismatch (forward-tolerant replay).
fn serde_json_like_parse(line: &str) -> Option<MeshEvent> {
    // Expected: {"prev":[..32 bytes..],"actor_pubkey":[..32..],"actor_seq":N,"payload":"<hex>"}
    let prev = extract_b256(line, "\"prev\":")?;
    let actor = extract_b256(line, "\"actor_pubkey\":")?;
    let seq = extract_u64(line, "\"actor_seq\":")?;
    let payload = extract_hex(line, "\"payload\":\"")?;
    Some(MeshEvent {
        prev,
        actor_pubkey: actor,
        actor_seq: seq,
        payload,
    })
}

fn extract_b256(s: &str, key: &str) -> Option<[u8; 32]> {
    let i = s.find(key)? + key.len();
    let rest = &s[i..];
    let end = rest.find(']')?;
    let nums = &rest[..end];
    let mut out = [0u8; 32];
    let mut idx = 0;
    for part in nums.split(',') {
        let p = part.trim().trim_start_matches('[');
        if let Ok(v) = p.parse::<u8>() {
            if idx < 32 {
                out[idx] = v;
                idx += 1;
            }
        }
    }
    if idx == 32 {
        Some(out)
    } else {
        None
    }
}

fn extract_u64(s: &str, key: &str) -> Option<u64> {
    let i = s.find(key)? + key.len();
    let rest = &s[i..];
    let end = rest.find(',').or_else(|| rest.find('}'))?;
    rest[..end].trim().parse::<u64>().ok()
}

fn extract_hex(s: &str, key: &str) -> Option<Vec<u8>> {
    let i = s.find(key)? + key.len();
    let rest = &s[i..];
    let end = rest.find('"')?;
    let hex = &rest[..end];
    if hex.len() % 2 != 0 {
        return None;
    }
    Some(
        (0..hex.len())
            .step_by(2)
            .filter_map(|j| u8::from_str_radix(&hex[j..j + 2], 16).ok())
            .collect::<Vec<u8>>(),
    )
}

impl EventStore for FileEventStore {
    fn contains(&self, id: &[u8; 32]) -> bool {
        self.by_id.contains_key(id)
    }
    fn insert(&mut self, id: [u8; 32], ev: MeshEvent) -> Result<(), StoreError> {
        if self.by_id.contains_key(&id) {
            return Ok(()); // idempotent no-op — nothing to persist
        }
        // Append one JSON line + fsync (crash-safe). Uses only std::fs.
        let line = format!(
            "{{\"prev\":{:?},\"actor_pubkey\":{:?},\"actor_seq\":{},\"payload\":\"{}\"}}\n",
            ev.prev,
            ev.actor_pubkey,
            ev.actor_seq,
            ev.payload
                .iter()
                .map(|b| format!("{:02x}", b))
                .collect::<String>()
        );
        // Each IO step now propagates a typed StoreError instead of `let _ =`
        // swallowing it — an open failure no longer falls through silently.
        // The handle is opened LAZILY (only on the first insert that reaches
        // here) and cached thereafter — construction (`open()`) keeps
        // tolerating a not-yet-existing file/parent; only an insert can ever
        // surface `StoreError::Open` (H1 §4 criterion 4, unchanged).
        if self.handle.is_none() {
            let f = open_file(&self.path, OpenMode::Append)
                .map_err(|e| StoreError::Open(e.to_string()))?;
            self.handle = Some(f);
        }
        let f = self
            .handle
            .as_mut()
            .expect("handle is Some — just set above or cached from a prior insert");
        f.write_all(line.as_bytes())
            .map_err(|e| StoreError::Write(e.to_string()))?;
        f.flush().map_err(|e| StoreError::Flush(e.to_string()))?;

        // Item 26 group-commit: fsync every `batch_size` inserts. With the
        // default `batch_size = 1` this fires on EVERY insert — byte-for-byte
        // the same barrier cadence as before this change.
        self.pending_since_sync += 1;
        if self.pending_since_sync >= self.batch_size {
            f.sync_all().map_err(|e| StoreError::Sync(e.to_string()))?;
            self.fsync_calls += 1;
            self.pending_since_sync = 0;
        }
        // Order is load-bearing (H1 §2.2): the in-memory index advances only
        // after the WRITE succeeds (always true at this point — every `?`
        // above already returned). With `batch_size = 1` this is exactly
        // equivalent to "after sync_all succeeds" (today's invariant,
        // unchanged, since the sync above always fires in that case). With
        // `batch_size > 1` this is the explicit, documented acknowledged-
        // before-durable tradeoff `with_batch_size`'s doc names: the index
        // (and therefore this method's own idempotent-duplicate check, the
        // `contains_key` guard at the top) reflects WRITTEN state, which may
        // not yet be FSYNCED when the batch hasn't closed — necessary so a
        // duplicate insert() within the same unsynced batch is still detected
        // as a no-op rather than double-written to the file.
        self.by_id.insert(id, ev);
        self.tip = Some(id);
        self.count += 1;
        // Item 61 (gap G5): the durability counters advance alongside the
        // index — `events_appended` counts writes (matches `count`/`by_id`);
        // `fsync_calls` only increments when the barrier above actually fired.
        // P3 telemetry; never gates the barrier itself.
        self.events_appended += 1;
        Ok(())
    }
    fn get(&self, id: &[u8; 32]) -> Option<MeshEvent> {
        self.by_id.get(id).cloned()
    }
    fn len(&self) -> usize {
        self.count
    }
    fn tip(&self) -> Option<[u8; 32]> {
        self.tip
    }
    fn set_tip(&mut self, id: [u8; 32]) {
        self.tip = Some(id);
    }
    fn ids(&self) -> Vec<[u8; 32]> {
        self.by_id.keys().copied().collect()
    }
}

#[cfg(test)]
mod file_store_tests {
    use super::*;
    use std::env::temp_dir;
    use crate::vfs as fs;

    fn tmp_path(tag: &str) -> std::path::PathBuf {
        let mut p = temp_dir();
        p.push(format!(
            "hydra-volya-anu-{}-{}.log",
            tag,
            std::process::id()
        ));
        let _ = fs::remove_file(&p);
        p
    }

    /// G4 — durable: events survive a reopen (replay), idempotent re-insert,
    /// and `get` retrieves by content-id. Egress-free (std::fs only).
    #[test]
    fn file_store_survives_reopen_and_replays() {
        let path = tmp_path("reopen");
        {
            let mut s = FileEventStore::open(&path).unwrap();
            let ev = MeshEvent {
                prev: [0u8; 32],
                actor_pubkey: [7u8; 32],
                actor_seq: 1,
                payload: b"genesis-intent".to_vec(),
            };
            let id = ev.event_id();
            s.insert(id, ev.clone()).expect("insert durable");
            assert!(s.contains(&id));
            assert_eq!(s.get(&id), Some(ev.clone()));
            // Re-insert same id — idempotent no-op (count stays 1).
            s.insert(id, ev).expect("idempotent re-insert ok");
            assert_eq!(s.len(), 1);
        }
        // Reopen: replay must restore the event.
        let s2 = FileEventStore::open(&path).unwrap();
        assert_eq!(s2.len(), 1, "event replayed from disk");
        let ev = MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: [7u8; 32],
            actor_seq: 1,
            payload: b"genesis-intent".to_vec(),
        };
        assert!(s2.contains(&ev.event_id()));
        let _ = fs::remove_file(&path);
    }

    /// G4 — the organism's full closed loop persists across restart: commit via
    /// Hydra with a FileEventStore, then boot_verify after reopening.
    #[test]
    fn hydra_durable_closed_loop_across_restart() {
        let path = tmp_path("loop");
        let base = vec![
            TopoEdge {
                from: 0,
                to: 1,
                weight: 1.0,
            },
            TopoEdge {
                from: 1,
                to: 2,
                weight: 1.0,
            },
        ];
        let committed_id;
        {
            let mut h = Hydra::new(FileEventStore::open(&path).unwrap(), 3, base.clone());
            let ev = MeshEvent {
                prev: [0u8; 32],
                actor_pubkey: [3u8; 32],
                actor_seq: 1,
                payload: b"self-mutation-A".to_vec(),
            };
            let delta = vec![TopoEdge {
                from: 2,
                to: 0,
                weight: 0.3,
            }];
            let (out, _dec) = h
                .commit(ev.clone(), &delta, false, |_| Ok::<u64, String>(1))
                .expect("damped mutation commits");
            committed_id = match out {
                crate::event_log::AppendOutcome::Committed(id) => id,
                _ => panic!("expected committed"),
            };
        }
        // Reopen: the organism re-bootstraps from durable state.
        let h2 = Hydra::new(FileEventStore::open(&path).unwrap(), 3, base);
        assert!(
            h2.log().contains(&committed_id),
            "event persisted across restart"
        );
        assert_eq!(
            h2.boot_verify(),
            0.0,
            "baseline still acyclic after restart"
        );
        let _ = fs::remove_file(&path);
    }

    /// H1 §4 criterion 4 — `FileEventStore` no longer swallows. Point it at a
    /// path whose parent directory does not exist so `OpenOptions::open` fails →
    /// `insert` returns `Err(StoreError::Open(_))`; the in-memory by_id/tip/count
    /// do NOT advance; the caller sees `Err`, never a fabricated success.
    /// Falsifies the old `if let Ok(f)` fall-through at the former `hydra.rs:840`.
    ///
    /// Judgment call (flagged): the blueprint (§4 criterion 4) says "read-only
    /// directory". A chmod 0555 dir does NOT make `open` fail for a root test
    /// runner (root bypasses the mode bits), so a *missing parent directory* is
    /// used instead — it fails `open` deterministically for any uid, exercising
    /// the identical `StoreError::Open` pole the criterion targets.
    #[test]
    fn file_store_open_failure_surfaces_not_swallowed() {
        let bogus = temp_dir()
            .join(format!("h1-missing-parent-{}", std::process::id()))
            .join("evlog.log");
        // Ensure the parent directory truly does not exist.
        let _ = fs::remove_dir_all(bogus.parent().unwrap());
        // open() lazily tolerates a not-yet-existing file (nothing to replay).
        let mut s = FileEventStore::open(&bogus).expect("open tolerates a missing file");
        let ev = MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: [5u8; 32],
            actor_seq: 1,
            payload: b"x".to_vec(),
        };
        let id = ev.event_id();
        let res = s.insert(id, ev);
        assert!(
            matches!(res, Err(StoreError::Open(_))),
            "open failure MUST surface as Err(StoreError::Open); got {res:?}"
        );
        // The in-memory index MUST NOT advance on a failed open.
        assert!(!s.contains(&id), "by_id must not advance on open failure");
        assert!(s.tip().is_none(), "tip must not advance on open failure");
        assert_eq!(s.len(), 0, "count must not advance on open failure");
    }

    /// Item 61 (gap G5) — durability continuous counters. Each `insert` that lands an
    /// event ALSO bumps `events_appended`; the trailing `sync_all` bumps `fsync_calls`.
    /// We assert BOTH: the counters move AND — critically — the index count is consistent
    /// with `events_appended` (the counter never lies about what was durably written).
    /// The counters are P3-plane telemetry: they never gate the `sync_all` durability
    /// barrier (green testing of the existing no-silent-failure invariants still holds).
    #[test]
    fn durability_counters_advance_on_append_and_fsync() {
        let path = tmp_path("item61-counters");
        let mut s = FileEventStore::open(&path).unwrap();
        assert_eq!(s.durability_counters().events_appended, 0);
        assert_eq!(s.durability_counters().fsync_calls, 0);

        let mk = |seq: u8| MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: [9u8; 32],
            actor_seq: seq as u64,
            payload: vec![seq],
        };
        for seq in 1..=3u8 {
            let ev = mk(seq);
            let id = ev.event_id();
            s.insert(id, ev).expect("append+fsync");
        }
        let c = s.durability_counters();
        assert_eq!(c.events_appended, 3, "three events appended → counter = 3");
        assert_eq!(c.fsync_calls, 3, "each insert fsyncs → fsync counter = 3");
        assert_eq!(s.len(), 3, "index count matches appended counter");

        let _ = fs::remove_file(&path);
    }

    /// Item 26 group-commit — `with_batch_size(3)` defers `sync_all` until the
    /// 3rd insert closes the batch. Proves the barrier cadence actually
    /// changes (not just that the fields exist): 2 inserts leave `fsync_calls`
    /// at 0 and `pending_unsynced` at 2; the 3rd insert fires the barrier.
    #[test]
    fn with_batch_size_defers_fsync_until_batch_closes() {
        let path = tmp_path("item26-batch3");
        let mut s = FileEventStore::open(&path).unwrap().with_batch_size(3);

        let mk = |seq: u8| MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: [11u8; 32],
            actor_seq: seq as u64,
            payload: vec![seq],
        };

        s.insert(mk(1).event_id(), mk(1)).expect("insert 1");
        let c = s.durability_counters();
        assert_eq!(c.fsync_calls, 0, "batch not closed yet — no sync_all");
        assert_eq!(c.pending_unsynced, 1);

        s.insert(mk(2).event_id(), mk(2)).expect("insert 2");
        let c = s.durability_counters();
        assert_eq!(c.fsync_calls, 0, "still short of batch_size=3");
        assert_eq!(c.pending_unsynced, 2);

        s.insert(mk(3).event_id(), mk(3))
            .expect("insert 3 closes batch");
        let c = s.durability_counters();
        assert_eq!(c.fsync_calls, 1, "3rd insert closes the batch → 1 sync_all");
        assert_eq!(c.pending_unsynced, 0, "pending resets after the barrier");
        assert_eq!(c.events_appended, 3);
        assert_eq!(s.len(), 3, "all 3 events index-visible despite 1 fsync");

        let _ = fs::remove_file(&path);
    }

    /// Item 26 — `flush_pending` forces the durability barrier early (before
    /// the batch naturally closes) and is a no-op when nothing is buffered.
    #[test]
    fn flush_pending_forces_barrier_and_is_noop_when_empty() {
        let path = tmp_path("item26-flush");
        let mut s = FileEventStore::open(&path).unwrap().with_batch_size(10);

        // No writes yet — flush_pending must be a true no-op.
        s.flush_pending().expect("no-op flush ok");
        assert_eq!(s.durability_counters().fsync_calls, 0);

        let ev = MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: [12u8; 32],
            actor_seq: 1,
            payload: b"partial-batch".to_vec(),
        };
        let id = ev.event_id();
        s.insert(id, ev).expect("insert 1 of 10");
        assert_eq!(s.durability_counters().pending_unsynced, 1);
        assert_eq!(s.durability_counters().fsync_calls, 0);

        s.flush_pending().expect("forced flush");
        let c = s.durability_counters();
        assert_eq!(c.fsync_calls, 1, "flush_pending forced a sync_all");
        assert_eq!(c.pending_unsynced, 0);

        // Reopen: the flushed event must have actually survived to disk.
        let s2 = FileEventStore::open(&path).unwrap();
        assert!(
            s2.contains(&id),
            "flush_pending durably persisted the event"
        );

        let _ = fs::remove_file(&path);
    }

    /// Item 26 — `with_batch_size` panics if called after a write is already
    /// buffered, so the barrier cadence can never change silently mid-stream.
    #[test]
    #[should_panic(expected = "with_batch_size must be set before any insert")]
    fn with_batch_size_panics_if_a_write_is_already_pending() {
        let path = tmp_path("item26-panic");
        let mut s = FileEventStore::open(&path).unwrap().with_batch_size(5);
        let ev = MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: [13u8; 32],
            actor_seq: 1,
            payload: b"x".to_vec(),
        };
        s.insert(ev.event_id(), ev)
            .expect("insert leaves 1 pending");
        let _ = s.with_batch_size(2); // must panic — pending_since_sync != 0
    }

    /// Item 26 — `batch_size = 1` (the default) is byte-for-byte the old
    /// per-event-fsync cadence: every insert closes its own batch of 1.
    #[test]
    fn default_batch_size_one_fsyncs_every_insert() {
        let path = tmp_path("item26-default");
        let mut s = FileEventStore::open(&path).unwrap(); // no with_batch_size call

        let mk = |seq: u8| MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: [14u8; 32],
            actor_seq: seq as u64,
            payload: vec![seq],
        };
        for seq in 1..=4u8 {
            let ev = mk(seq);
            s.insert(ev.event_id(), ev).expect("insert");
            let c = s.durability_counters();
            assert_eq!(
                c.pending_unsynced, 0,
                "batch_size=1 never leaves anything pending"
            );
            assert_eq!(
                c.fsync_calls, seq as u64,
                "every insert fires its own sync_all"
            );
        }

        let _ = fs::remove_file(&path);
    }

    /// Item 5 (cross-mesh replication) — `mesh_replication::reconcile` is
    /// generic over `EventStore`, not just `MemEventStore`; prove it converges
    /// two independent, disk-backed `FileEventStore`s the same way. This is
    /// the closest this crate gets to a real 2-node scenario without an
    /// actual transport: two separate files (durable, `std::fs`-only) stand
    /// in for two separate nodes.
    #[test]
    fn file_stores_reconcile_to_identical_folded_state() {
        use crate::mesh_replication::{reconcile, MerkleLog};

        let path_a = tmp_path("item5-node-a");
        let path_b = tmp_path("item5-node-b");
        let mut node_a = FileEventStore::open(&path_a).unwrap();
        let mut node_b = FileEventStore::open(&path_b).unwrap();

        let a_ev = MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: [21u8; 32],
            actor_seq: 1,
            payload: b"node-a-authored".to_vec(),
        };
        node_a.insert(a_ev.event_id(), a_ev).unwrap();

        let b_ev = MeshEvent {
            prev: [0u8; 32],
            actor_pubkey: [22u8; 32],
            actor_seq: 1,
            payload: b"node-b-authored".to_vec(),
        };
        node_b.insert(b_ev.event_id(), b_ev).unwrap();

        reconcile(&mut node_a, &node_b).expect("A pulls from B");
        reconcile(&mut node_b, &node_a).expect("B pulls from A");

        assert_eq!(
            MerkleLog::from_store(&node_a).root(),
            MerkleLog::from_store(&node_b).root(),
            "two disk-backed nodes converge to the same Merkle root"
        );
        assert_eq!(node_a.len(), 2);
        assert_eq!(node_b.len(), 2);

        let _ = fs::remove_file(&path_a);
        let _ = fs::remove_file(&path_b);
    }
}
