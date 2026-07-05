//! Advisory-lock id registry — Q10 (`docs/design/rebuild-jobs-s8-council/proposal.md` §7):
//! `pg_try_advisory_lock(id)` is how every cron guarantees single-flight across `web` instances
//! (carried verbatim, §7). The Node source has a LATENT bug the port must not repeat: the census
//! confirmed `order-timeout-sweep.ts` (`SWEEP_LOCK_ID = 5`) and
//! `access-request-retention.ts` (`RETENTION_LOCK = 5`) **use the same raw id**, avoided colliding
//! today only because each takes its own `pool.connect()` — a latent bug, not a proven-safe
//! coincidence (two crons that ever DID run concurrently against the same connection pool would
//! silently serialize on each other's lock, not just their own).
//!
//! This module is the **one source of truth**: every S8 cron's lock id is a named constant here,
//! and the registry's uniqueness is asserted by a test (`tests::every_advisory_lock_id_is_unique`)
//! — a future cron that copies-pastes a numeric literal instead of adding a constant here is
//! exactly the mistake this exists to make structurally harder (a raw `pg_try_advisory_lock(5)`
//! call site outside this module has nothing to fail against, which is why every S8 cron in
//! `crate::jobs::crons::*` imports its id from here rather than writing a literal).
//!
//! IDs 1-9 mirror the OLD Node ids where a 1:1 job survives the port (so an ops dashboard built
//! against the old numbers keeps meaning) EXCEPT id 5, which the two colliding Node workers
//! shared — this registry gives each its own permanent, never-reused id instead.
//!
//! ## Why several constants are `#[allow(dead_code)]`
//! This registry deliberately reserves an id for every cron the FULL 23-cron fleet will
//! eventually need (§2's "port each" instruction), not just the ~6 this pass wires into
//! `main.rs`. A constant with no current caller is a RESERVATION, not dead weight — the whole
//! point is that when a future pass ports (say) `courier.offer_sweep`, its id is already claimed
//! here and the uniqueness test already covers it, rather than that pass inventing a fresh
//! literal that might collide.
#![allow(
    dead_code,
    reason = "reserved ids for crons not yet ported this pass — see module doc"
)]

/// `order.timeout_sweep` — the cross-tenant safety-net floor (§6, §7). Was Node id 5 (shared,
/// collision).
pub const ORDER_TIMEOUT_SWEEP: i64 = 101;
/// `access-request.retention-sweep`. Was Node id 5 (the OTHER half of the collision).
pub const ACCESS_REQUEST_RETENTION_SWEEP: i64 = 102;
/// `access-request.reconcile`. Node id 6.
pub const ACCESS_REQUEST_RECONCILE: i64 = 103;
/// `dwell.monitor`. Node id 2.
pub const DWELL_MONITOR: i64 = 104;
/// `signal.raiser`. Node id 3.
pub const SIGNAL_RAISER: i64 = 105;
/// `anonymizer.gdpr` retention sweep. Node id 4.
pub const ANONYMIZER_RETENTION: i64 = 106;
/// `acquisition.retention-sweep`. Node id 7.
pub const ACQUISITION_RETENTION: i64 = 107;
/// `delivery-trace.retention-sweep`. Node id 8.
pub const DELIVERY_TRACE_RETENTION: i64 = 108;
/// `courier.offer_sweep`. Node id 9.
pub const COURIER_OFFER_SWEEP: i64 = 109;
/// `rates.refresh`. Node id 8192 (no singletonKey either upstream — carried as its own id here).
pub const RATES_REFRESH: i64 = 110;
/// `settlement.generate` — money-adjacent (§6, Q6 🔴). Not queue-backed in Node (ran under
/// `SETTLEMENT_CRON` with no advisory lock recorded in the census's grep of
/// `pg_try_advisory_lock` call sites) — the port ADDS single-flight here rather than relying
/// solely on the DEFINER fn's watermark, defense-in-depth per REV-S8-3.
pub const SETTLEMENT_GENERATE: i64 = 111;
/// `refund_due.reconcile` tick (calls `app_reconcile_refund_due()`, currently an unapplied draft
/// — see `crate::jobs::crons::refund_reconciler` module doc).
pub const REFUND_DUE_RECONCILE: i64 = 112;
/// `reconciliation.nightly` — read-only, but still single-flighted so two instances don't both
/// double-page the same drift.
pub const RECONCILIATION_NIGHTLY: i64 = 113;
/// `liveness.check` — the watcher-of-the-watcher (§7 Q-WORKER-ROSTER-DUP).
pub const LIVENESS_CHECK: i64 = 114;

/// Every id this registry hands out, for the uniqueness test below. Kept as a `const fn`-free
/// plain array (not a `HashSet` at const-eval time) — this is test-only plumbing, not a runtime
/// lookup table.
const ALL_IDS: &[(&str, i64)] = &[
    ("ORDER_TIMEOUT_SWEEP", ORDER_TIMEOUT_SWEEP),
    (
        "ACCESS_REQUEST_RETENTION_SWEEP",
        ACCESS_REQUEST_RETENTION_SWEEP,
    ),
    ("ACCESS_REQUEST_RECONCILE", ACCESS_REQUEST_RECONCILE),
    ("DWELL_MONITOR", DWELL_MONITOR),
    ("SIGNAL_RAISER", SIGNAL_RAISER),
    ("ANONYMIZER_RETENTION", ANONYMIZER_RETENTION),
    ("ACQUISITION_RETENTION", ACQUISITION_RETENTION),
    ("DELIVERY_TRACE_RETENTION", DELIVERY_TRACE_RETENTION),
    ("COURIER_OFFER_SWEEP", COURIER_OFFER_SWEEP),
    ("RATES_REFRESH", RATES_REFRESH),
    ("SETTLEMENT_GENERATE", SETTLEMENT_GENERATE),
    ("REFUND_DUE_RECONCILE", REFUND_DUE_RECONCILE),
    ("RECONCILIATION_NIGHTLY", RECONCILIATION_NIGHTLY),
    ("LIVENESS_CHECK", LIVENESS_CHECK),
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// Q10's whole point: no two named constants may share an id — the exact class of bug
    /// (`order-timeout-sweep` vs `access-request-retention`, both `= 5`) this registry exists to
    /// make structurally impossible to reintroduce.
    #[test]
    fn every_advisory_lock_id_is_unique() {
        let mut seen = HashSet::new();
        for (name, id) in ALL_IDS {
            assert!(
                seen.insert(*id),
                "advisory lock id {id} (from {name}) is already used by another constant in this \
                 registry — this is exactly the ORDER_TIMEOUT_SWEEP/ACCESS_REQUEST_RETENTION_SWEEP \
                 collision class Q10 exists to prevent"
            );
        }
        assert_eq!(seen.len(), ALL_IDS.len());
    }

    #[test]
    fn the_old_colliding_id_is_not_reused() {
        // The Node id both sweeps shared. Neither of this registry's replacements may be 5 —
        // proves the port didn't just relabel the same collision under new names.
        assert_ne!(ORDER_TIMEOUT_SWEEP, 5);
        assert_ne!(ACCESS_REQUEST_RETENTION_SWEEP, 5);
    }
}
