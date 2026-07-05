//! The `jobs` table schema — the ONE new table the S8 hand-rolled queue owns
//! (`docs/design/rebuild-jobs-s8-council/proposal.md` §3, §10, §11 Q-SKIP-LOCKED).
//!
//! ## Why this is Rust source, not a `.sql` file under `packages/db/migrations/`
//! §10 of the council packet is explicit: **"S8 does not author or apply any migration — it ...
//! provides the schema for its own `jobs` table for the operator to place."** `migrations/` is a
//! hard-blocked red-line zone in this repo (`protect-paths.sh`/`guard-bash.sh` — no override
//! exists, unlike `serious-gate.sh`'s council-clearance path) and the DB is frozen for anything
//! this build doesn't own end-to-end. So the schema lives here as a **pinned string constant**
//! (same pattern as `db.rs`'s `SET_TENANT_STATEMENT`/`SET_USER_STATEMENT`): unit-tested without a
//! live database, and the OPERATOR copies it verbatim into a forward-only, additive
//! `packages/db/migrations/<timestamp>_s8-jobs-table.ts` migration, staging-first, per §10's own
//! instruction. This module never executes the DDL itself.
//!
//! ## Design — two tables, two distinct jobs
//! - **`jobs`** — the SKIP LOCKED queue itself (§3.1-3.3). `idempotency_key` is a NULLABLE column
//!   with a PARTIAL unique index (`WHERE idempotency_key IS NOT NULL`) — this is the
//!   ENQUEUE-time dedup (§3.4: "a duplicate producer-enqueue is a no-op insert"), which does
//!   NOTHING to protect a single claimed row from a crash-after-send double-run (that's a
//!   different failure mode — same row, retried — see `notification_dedup` below and REV-S8-1).
//! - **`notification_dedup`** — the REV-S8-1 (CRIT) fix: a durable Postgres CAS for
//!   "claim-before-send," replacing the Node worker's in-memory `Set` (`crate::jobs::dedup`'s
//!   module doc has the full crash-recovery argument). A `PRIMARY KEY` on `dedup_key` IS the
//!   guard — `INSERT ... ON CONFLICT (dedup_key) DO NOTHING RETURNING dedup_key` either claims it
//!   (0 prior rows) or proves someone already did (this exact durability property does not exist
//!   anywhere in the current schema: `notification_outbox_audit` — the table the old dedup
//!   THOUGHT was backing it — has no unique constraint on anything, mig
//!   `1790000000007_notification-outbox-audit.ts`, confirmed by direct read).
//!
//! Both constants below are `#[allow(dead_code)]` by design: they are reference text for a HUMAN
//! (the operator placing a migration file) and for this module's own pinning tests, never
//! `sqlx`-executed by this crate at runtime — see "Why this is Rust source" above for why that is
//! the correct shape, not a gap.
#![allow(
    dead_code,
    reason = "operator-placed reference DDL, never executed by this crate at runtime"
)]

/// Operator-placed verbatim into a NEW, forward-only, additive migration file (never edits an
/// existing one; touches no business table). See module doc for why this lives here as a string,
/// not a `.sql`/`packages/db/migrations/` file.
pub const JOBS_TABLE_DDL: &str = r#"
CREATE TABLE jobs (
  id BIGSERIAL PRIMARY KEY,
  queue_name TEXT NOT NULL,
  payload JSONB NOT NULL,
  state TEXT NOT NULL DEFAULT 'queued' CHECK (state IN ('queued', 'active', 'completed', 'failed')),
  priority SMALLINT NOT NULL DEFAULT 0,
  run_after TIMESTAMPTZ NOT NULL DEFAULT now(),
  locked_until TIMESTAMPTZ,
  attempts INTEGER NOT NULL DEFAULT 0,
  max_attempts INTEGER NOT NULL DEFAULT 8,
  last_error TEXT,
  idempotency_key TEXT,
  started_at TIMESTAMPTZ,
  completed_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- Enqueue-time dedup (§3.4) — a duplicate producer INSERT with the same idempotency_key is a
-- silent no-op via `ON CONFLICT (idempotency_key) WHERE idempotency_key IS NOT NULL DO NOTHING`.
CREATE UNIQUE INDEX jobs_idempotency_key_uq ON jobs (idempotency_key) WHERE idempotency_key IS NOT NULL;

-- The claim-loop predicate (crate::jobs::runner::CLAIM_SQL) scans exactly this shape: ready
-- 'queued' work OR a reclaimable 'active' job whose visibility timeout lapsed.
CREATE INDEX jobs_claim_idx ON jobs (priority DESC, run_after) WHERE state IN ('queued', 'active');
"#;

/// REV-S8-1 (CRIT) — the durable claim-before-send ledger. See module doc + `crate::jobs::dedup`.
pub const NOTIFICATION_DEDUP_TABLE_DDL: &str = r#"
CREATE TABLE notification_dedup (
  dedup_key TEXT PRIMARY KEY,
  job_id BIGINT,
  claimed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jobs_table_ddl_has_no_unique_constraint_gap() {
        // The exact bug class this schema exists to NOT repeat: notification_outbox_audit has
        // an ON CONFLICT DO NOTHING with nothing backing it. Pin that this table's dedup column
        // DOES have a real unique index.
        assert!(JOBS_TABLE_DDL.contains("CREATE UNIQUE INDEX jobs_idempotency_key_uq"));
        assert!(JOBS_TABLE_DDL.contains("idempotency_key IS NOT NULL"));
    }

    #[test]
    fn jobs_table_ddl_state_check_matches_the_claim_loop_states() {
        for state in ["queued", "active", "completed", "failed"] {
            assert!(
                JOBS_TABLE_DDL.contains(state),
                "the CHECK constraint must list every state crate::jobs::runner transitions through"
            );
        }
    }

    #[test]
    fn notification_dedup_ddl_has_a_primary_key_on_dedup_key() {
        // The load-bearing property: PRIMARY KEY (not a plain index, not ON CONFLICT DO NOTHING
        // with no arbiter) is what makes claim-before-send an atomic CAS.
        assert!(NOTIFICATION_DEDUP_TABLE_DDL.contains("dedup_key TEXT PRIMARY KEY"));
    }

    #[test]
    fn neither_ddl_string_is_a_sql_file_on_disk() {
        // Structural pin, not a filesystem check: this module's whole POINT is that the DDL text
        // lives in a `.rs` constant, never a `packages/db/migrations/*.sql` file this build could
        // author. Nothing to assert against the filesystem — the doc comment is the contract; this
        // test just anchors that both constants still exist and are non-empty (a future accidental
        // deletion would show up as some OTHER test's DDL assertions failing, but an empty-string
        // regression deserves its own explicit failure).
        assert!(!JOBS_TABLE_DDL.trim().is_empty());
        assert!(!NOTIFICATION_DEDUP_TABLE_DDL.trim().is_empty());
    }
}
