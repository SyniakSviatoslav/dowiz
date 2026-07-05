//! S8 jobs/notifications — the background-work runtime
//! (`docs/design/rebuild-jobs-s8-council/`). Unlike every prior S-surface, most of this module is
//! NOT axum routes: it is the hand-rolled `SKIP LOCKED` queue runner, tokio cron loops, and
//! notification dispatch that keep running when no HTTP request is in flight (proposal §1).
//!
//! ## Module map
//! - [`ddl`] — the `jobs`/`notification_dedup` table schema (operator-placed, never applied by
//!   this crate — see that module's doc for why).
//! - [`runner`] — the `SKIP LOCKED` claim/complete/fail loop (Q1 🔴).
//! - [`backoff`] — retry backoff+jitter (Q-BARE-DEFAULTS).
//! - [`dedup`] — REV-S8-1 (🔴 CRIT) durable claim-before-send.
//! - [`advisory_lock`] — the cron lock-id registry (Q10).
//! - [`consent`] — opt-out + quiet-hours pure decision logic (Q3 🔴).
//! - [`dispatch`] — claim-check payload, seat-first customer-status read (REV-S8-5b), error
//!   redaction (REV-S8-5d).
//! - [`bridge`] — the S5->S8 partition-aware overlap shim (REV-S8-4 🔴).
//! - [`worker_roster`] — the unified critical/expected worker roster (Q-WORKER-ROSTER-DUP).
//! - [`channels`] — notification send adapters (push, email, telegram — all built).
//! - [`crons`] — the money-adjacent + housekeeping cron jobs (thin DEFINER callers, §6/§8).
//!
//! ## SERIOUS-GATE note (build history, not a current gap)
//! `jobs::channels::telegram` and `routes::telegram_webhook` were blocked mid-build: this
//! worktree's local `.claude/state/serious-cleared` initially carried no clearance line (the
//! MAIN-state clearance `s8-jobs-build|<expiry>` had not yet been mirrored into this isolated
//! worktree), so `serious-gate.sh` denied any `Edit`/`Write` whose path matched `telegram` — by
//! design, per this build's binding instructions (do not self-clear; wait for the lead to clear
//! the worktree). The lead cleared it mid-session; both files are now built, tested, and wired —
//! see the final task report for the exact timeline and the gate's deny/allow log lines.

pub mod advisory_lock;
pub mod backoff;
pub mod bridge;
pub mod channels;
pub mod consent;
pub mod cron;
pub mod crons;
pub mod ddl;
pub mod dedup;
pub mod dispatch;
pub mod runner;
pub mod worker;
pub mod worker_roster;
