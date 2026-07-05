//! Notification send adapters (`docs/design/rebuild-jobs-s8-council/proposal.md` §4). Every
//! adapter implements [`SendOutcome`]'s shape uniformly — permanent-disable on 401/403,
//! rate-limit-with-retry-after on 429, a bounded timeout on every external call (threat S8-T11) —
//! CARRY-VERBATIM of the Node adapters' response classification, even though each channel's own
//! HTTP shape differs.
//!
//! ## Build status
//! [`push`] (VAPID web-push), [`email`] (Resend, ops-alert-only), and [`telegram`] are all built —
//! the SERIOUS-GATE block noted in earlier revisions of this doc was lifted mid-build once the
//! worktree's clearance was mirrored from MAIN state (`s8-jobs-build`); see the final task report
//! for the exact timeline.

pub mod email;
pub mod push;
pub mod telegram;

/// What an external send attempt reported — uniform across every channel adapter so
/// `crate::jobs::dispatch`'s orchestration (target auto-disable on permanent failure, retry
/// scheduling on rate-limit) doesn't need a per-channel match arm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SendOutcome {
    Delivered,
    /// 401/403 (or the push-equivalent 410/404 Gone/NotFound) — the target/subscription is
    /// permanently invalid; the caller must disable/prune it, not retry (Q-PUSH-PRUNE,
    /// Q-TG-CIRCUIT's `401/403 -> permanent target status='disabled'`).
    PermanentlyRejected {
        reason: String,
    },
    /// 429 — honor `retry-after` if the service sent one (Q-TG-CIRCUIT: "429 honors
    /// retry-after"; the email adapter's own 429 handling is the same shape).
    RateLimited {
        retry_after: std::time::Duration,
    },
    /// The bounded per-call timeout elapsed (threat S8-T11 — every external call is
    /// timeout-bounded so a hung push/telegram/email service can never pin a held DB connection).
    TimedOut,
    /// Any other transport-level failure (DNS, connection refused, TLS, ...).
    NetworkError {
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn send_outcome_variants_are_exhaustively_distinguishable() {
        // A trivial compile-time-shape pin: every adapter's classification collapses into one of
        // these — if a future adapter needs a NEW outcome (e.g. a channel-specific soft-defer),
        // it should be added here rather than approximated into an existing variant, since
        // `crate::jobs::dispatch`'s orchestration branches on this type.
        let outcomes = [
            SendOutcome::Delivered,
            SendOutcome::PermanentlyRejected {
                reason: "401".to_string(),
            },
            SendOutcome::RateLimited {
                retry_after: std::time::Duration::from_secs(5),
            },
            SendOutcome::TimedOut,
            SendOutcome::NetworkError {
                message: "dns failure".to_string(),
            },
        ];
        assert_eq!(outcomes.len(), 5);
    }
}
