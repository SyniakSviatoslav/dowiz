//! S5 order-state — the shell-side remainder after Phase-Zero Step 3.
//!
//! The pure lifecycle DECISIONS (actor-gate, `updateOrderStatus` fold effects, CC-1 strand guards,
//! honest-dispatch gate, and the create-idempotency branch) were relocated into the sovereign core
//! (`dowiz-core`: `domain::kernel::policy` + `domain::kernel::idempotency`) — they carry no float,
//! clock, entropy, or IO, so they belong with the `decide`/`fold` law. They are RE-EXPORTED here under
//! the historical `state::…` path so every existing call site (`pg.rs`) is unchanged.
//!
//! What STAYS in the shell: [`classify_pg_error`]/[`PgErrorClass`]. SQLSTATE is Postgres vocabulary,
//! and platform words never enter the core (the split the sovereign gate enforces by construction).

// Pure decisions, now sovereign-core — re-exported so `super::state::assert_owner_target_allowed`,
// `state::transition_effects`, `state::cc1_strand_guard`, `state::needs_honest_dispatch`,
// `state::idempotency_decision`, and the `BindingState`/`ExistingKey`/`IdempotencyDecision` types all
// keep resolving exactly as before.
// Only the decisions the shell actually calls are re-exported (the rest of `policy` —
// `LifecycleEvent`, `TransitionEffects`, `status_at_column` — is reached via the return value of
// `transition_effects`, so importing the names here would be dead. Any future direct caller imports
// them from their real home, `domain::kernel::policy`).
pub use domain::kernel::idempotency::{ExistingKey, IdempotencyDecision, idempotency_decision};
pub use domain::kernel::policy::{
    BindingState, assert_owner_target_allowed, cc1_strand_guard, needs_honest_dispatch,
    transition_effects,
};

// ─────────────────────── Transient-PG classification (orders.ts:718-728) ───────────────────────
// SQLSTATE is Postgres vocabulary → this is the ONE piece of the old state.rs that STAYS in the shell.

/// How a Postgres error at commit maps to an HTTP outcome on the create hot path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PgErrorClass {
    /// `23505` unique violation → 409 IDEMPOTENCY_CONFLICT (the idempotency-key race, `orders.ts:719`).
    Conflict,
    /// Transient contention (serialization/deadlock/statement-timeout/conn-drop) → **503 retryable**
    /// (`orders.ts:724`) — a graceful "try again", never a scary 500.
    Transient,
    /// Anything else → 500 INTERNAL.
    Other,
}

/// The transient SQLSTATE set carried VERBATIM (`orders.ts:724`): serialization_failure,
/// deadlock_detected, query_canceled (the 4.5s statement-timeout fuse), too_many_connections, and
/// the connection-exception family.
const TRANSIENT_SQLSTATES: [&str; 7] = [
    "40001", "40P01", "57014", "53300", "08006", "08003", "08000",
];

/// Classifies a Postgres SQLSTATE for the create hot path (`orders.ts:718-728`).
pub fn classify_pg_error(sqlstate: &str) -> PgErrorClass {
    if sqlstate == "23505" {
        PgErrorClass::Conflict
    } else if TRANSIENT_SQLSTATES.contains(&sqlstate) {
        PgErrorClass::Transient
    } else {
        PgErrorClass::Other
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pg_unique_violation_is_conflict() {
        assert_eq!(classify_pg_error("23505"), PgErrorClass::Conflict);
    }

    #[test]
    fn transient_sqlstates_are_retryable_503() {
        for code in [
            "40001", "40P01", "57014", "53300", "08006", "08003", "08000",
        ] {
            assert_eq!(
                classify_pg_error(code),
                PgErrorClass::Transient,
                "{code} must be transient"
            );
        }
    }

    #[test]
    fn other_sqlstate_is_500() {
        assert_eq!(classify_pg_error("42703"), PgErrorClass::Other); // undefined_column
        assert_eq!(classify_pg_error("23503"), PgErrorClass::Other); // FK violation
    }
}
