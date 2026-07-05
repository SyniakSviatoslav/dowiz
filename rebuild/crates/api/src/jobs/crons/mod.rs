//! S8's cron fleet — `docs/design/rebuild-jobs-s8-council/proposal.md` §6/§7/§8. Every cron here
//! is a thin DEFINER-function CALLER: the money-math/erasure-semantics/dispatch-engine logic is
//! explicitly OUT of S8's scope (§2 "NOT S8" register) — this module owns only the scheduling,
//! the single-flight lock (`crate::jobs::cron::try_with_lock`, named ids from
//! `crate::jobs::advisory_lock`), and the at-least-once-safe calling convention. None of these
//! functions seat a tenant GUC (§8): `SECURITY DEFINER` functions cross every tenant in one pass
//! by design, the structural boundary REBUILD-MAP §8 calls out — the Rust worker CALLS them,
//! never re-derives their cross-tenant authority.

pub mod gdpr_sweep;
pub mod liveness;
pub mod order_timeout_sweep;
pub mod reconciliation;
pub mod refund_reconciler;
pub mod settlement;
