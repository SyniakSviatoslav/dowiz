//! Order-lifecycle POLICY — the pure decisions that sit AROUND the frozen `order_status` matrix
//! (REV-S5-9). None of this re-derives the 10×10 matrix (that is [`crate::assert_transition`],
//! byte-frozen); this is the RICH mutator's fold logic, the actor-gate, the CC-1 strand guards, and
//! the honest-dispatch gate — all side-effect-free decisions the `api` shell's handlers/repos consume.
//! Ports `apps/api/src/lib/orderStatusService.ts` and `lib/orderAuthz.ts`, and the `orders.ts` CC-1
//! arms.
//!
//! Extracted into the sovereign core in Phase-Zero Step 3 (was `crates/api/.../orders/state.rs`).
//! It carries NO float, NO clock, NO entropy, NO IO — the wasm sovereignty gate proves it. The one
//! part of the old `state.rs` that did NOT move is `classify_pg_error`/`PgErrorClass`: SQLSTATE is
//! Postgres vocabulary, and platform words never enter the core — it stays in the shell.
//!
//! The split the RESOLVE mandates (REV-S5-9 Q2): the MACHINE says what is *possible*
//! ([`crate::assert_transition`]), the ACTOR-GATE says who is *allowed* ([`assert_owner_target_allowed`]).

use crate::{ErrorCode, OrderStatus};

// ─────────────────────────── Actor-gate (orderAuthz.ts) ───────────────────────────

/// Ports `assertOwnerTargetAllowed` (`apps/api/src/lib/orderAuthz.ts:19-27`) VERBATIM.
///
/// The deliver-v2 offer-sweep-cancel addendum widened the MACHINE to permit
/// CONFIRMED/PREPARING/READY→CANCELLED, but those edges are SYSTEM-only (the dispatch-grace path).
/// An OWNER driving them via a request-supplied `newStatus` is refused **403 CANCEL_NOT_PERMITTED**.
/// The owner keeps PENDING→CANCELLED (pre-confirm) and IN_DELIVERY→CANCELLED (no-show). This is a
/// distinct AUTHORIZATION layer OVER the machine — call it AFTER [`crate::assert_transition`] says the
/// edge is machine-legal, BEFORE handing the transition to the mutator.
pub fn assert_owner_target_allowed(from: OrderStatus, to: OrderStatus) -> Result<(), ErrorCode> {
    let owner_forbidden_cancel_from = matches!(
        from,
        OrderStatus::Confirmed | OrderStatus::Preparing | OrderStatus::Ready
    );
    if to == OrderStatus::Cancelled && owner_forbidden_cancel_from {
        return Err(ErrorCode::CancelNotPermitted);
    }
    Ok(())
}

// ─────────────────────── updateOrderStatus fold effects (orderStatusService.ts) ───────────────────────

/// The per-transition `*_at` column an `updateOrderStatus` UPDATE stamps — the fixed
/// `STATUS_AT_COLUMN` allowlist (`orderStatusService.ts:11-18`). NEVER user input (an enum→column
/// match, safe to interpolate). REJECTED/CANCELLED have no stamp column → `None`.
pub fn status_at_column(status: OrderStatus) -> Option<&'static str> {
    match status {
        OrderStatus::Confirmed => Some("confirmed_at"),
        OrderStatus::Preparing => Some("preparing_at"),
        OrderStatus::Ready => Some("ready_at"),
        OrderStatus::InDelivery => Some("in_delivery_at"),
        OrderStatus::Delivered => Some("delivered_at"),
        OrderStatus::PickedUp => Some("picked_up_at"),
        OrderStatus::Pending
        | OrderStatus::Rejected
        | OrderStatus::Cancelled
        | OrderStatus::Scheduled => None,
    }
}

/// The lifecycle bus event `updateOrderStatus` publishes for notification fan-out
/// (`orderStatusService.ts:286-290`) — REV-S5-9 L1 (ORDER_CONFIRMED/REJECTED folds).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleEvent {
    OrderConfirmed,
    OrderRejected,
}

/// The side effects an `updateOrderStatus(current → new)` call performs, as a pure decision the
/// repo replays as SQL inside the tenant-seated tx. Every field ports a specific fold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransitionEffects {
    /// The `*_at` column to stamp (`STATUS_AT_COLUMN`), or `None`.
    pub stamp_column: Option<&'static str>,
    /// R2-3 assignment-terminalize fold (`orderStatusService.ts:139`): terminalize the active
    /// courier binding + free its shift in the SAME tx. Fires on ANY `→CANCELLED`/`→REJECTED`, and
    /// on the `IN_DELIVERY→READY` revert — so no order leaves to a terminal/downgrade with a
    /// stranded binding. Idempotent + cash-safe (writes no ledger hold).
    pub terminalize_assignment: bool,
    /// L-A `refund_due` fold (`orderStatusService.ts:165`): record a `refund_due` obligation for
    /// every `paid` payment, SAVEPOINT-wrapped fail-closed-per-order + fail-loud. Fires on
    /// `→CANCELLED`/`→REJECTED`. INERT until crypto flips (zero `paid` rows today) — ported whole so
    /// it is correct the moment crypto goes live; the mig-086 L-C trigger is the DB-level backstop.
    pub record_refund_due: bool,
    /// The lifecycle bus event to publish post-apply (REV-S5-9 L1), or `None`.
    pub lifecycle_event: Option<LifecycleEvent>,
}

/// Computes [`TransitionEffects`] for `current → new` — the single source of the fold decisions, so
/// the mutator's SQL and the tests agree. Ports the branch conditions in `updateOrderStatus`.
pub fn transition_effects(current: OrderStatus, new: OrderStatus) -> TransitionEffects {
    use OrderStatus::{Cancelled, InDelivery, Ready, Rejected};

    let terminal_cancel = new == Cancelled || new == Rejected;
    // R2-3: any →CANCELLED/→REJECTED, plus the IN_DELIVERY→READY revert (orderStatusService.ts:139).
    let terminalize_assignment = terminal_cancel || (current == InDelivery && new == Ready);
    // L-A: →CANCELLED/→REJECTED only (orderStatusService.ts:165).
    let record_refund_due = terminal_cancel;
    let lifecycle_event = match new {
        OrderStatus::Confirmed => Some(LifecycleEvent::OrderConfirmed),
        Rejected => Some(LifecycleEvent::OrderRejected),
        _ => None,
    };
    TransitionEffects {
        stamp_column: status_at_column(new),
        terminalize_assignment,
        record_refund_due,
        lifecycle_event,
    }
}

// ─────────────────────── CC-1 strand guards (orders.ts:929-955) ───────────────────────

/// Inputs to the CC-1 strand guard, read from `courier_assignments` inside the tx.
#[derive(Debug, Clone, Copy)]
pub struct BindingState {
    /// Any assignment in `('offered','assigned','accepted','picked_up')`.
    pub has_active_binding: bool,
    /// A `delivered` assignment exists.
    pub has_delivered_binding: bool,
}

/// Ports the CC-1 DELIVERED/PICKED_UP strand guards (`orders.ts:929-955`, money-audit H1) VERBATIM.
/// On a PATCH → DELIVERED/PICKED_UP:
///   (a) an ACTIVE binding exists → **409 ASSIGNMENT_ACTIVE** (complete via `/deliver`);
///   (b) `current == IN_DELIVERY` with NO delivered assignment → **409 USE_DELIVER_FLOW**.
/// Never-dispatched orders (no active binding, not IN_DELIVERY) stay PATCH-able. Non-DELIVERED/
/// PICKED_UP targets pass through untouched.
pub fn cc1_strand_guard(
    target: OrderStatus,
    current: OrderStatus,
    binding: BindingState,
) -> Result<(), ErrorCode> {
    if target != OrderStatus::Delivered && target != OrderStatus::PickedUp {
        return Ok(());
    }
    if binding.has_active_binding {
        return Err(ErrorCode::AssignmentActive);
    }
    if current == OrderStatus::InDelivery && !binding.has_delivered_binding {
        return Err(ErrorCode::UseDeliverFlow);
    }
    Ok(())
}

/// Honest-dispatch ordering (`orders.ts:962`, REV-S5-9 L2 CARRY): a PATCH → IN_DELIVERY on a
/// **delivery** order must find a courier BEFORE advancing (dispatch-then-advance, never
/// advance-then-orphan). Returns `true` when the handler must route through `attemptHonestDispatch`
/// instead of the plain mutator. CARRY the type-gate verbatim: a pickup order's IN_DELIVERY target
/// is NOT dispatched here (L2 register: the actor-gate covers only CANCELLED).
pub fn needs_honest_dispatch(new: OrderStatus, order_type_is_delivery: bool) -> bool {
    new == OrderStatus::InDelivery && order_type_is_delivery
}

#[cfg(test)]
mod tests {
    use super::*;
    use OrderStatus::*;

    // ── actor-gate ──

    #[test]
    fn owner_may_not_drive_system_only_cancel_edges() {
        // The widened SYSTEM-only edges: 403 CANCEL_NOT_PERMITTED for an owner.
        for from in [Confirmed, Preparing, Ready] {
            assert_eq!(
                assert_owner_target_allowed(from, Cancelled),
                Err(ErrorCode::CancelNotPermitted),
                "owner must not drive {from:?}→CANCELLED"
            );
        }
    }

    #[test]
    fn owner_keeps_pending_and_in_delivery_cancel() {
        assert!(assert_owner_target_allowed(Pending, Cancelled).is_ok());
        assert!(assert_owner_target_allowed(InDelivery, Cancelled).is_ok());
    }

    #[test]
    fn actor_gate_ignores_non_cancel_targets() {
        // A non-CANCELLED target is never actor-gated here (the machine already vetted it).
        assert!(assert_owner_target_allowed(Confirmed, Preparing).is_ok());
        assert!(assert_owner_target_allowed(Preparing, Ready).is_ok());
    }

    // ── fold effects ──

    #[test]
    fn stamp_columns_match_status_at_column_allowlist() {
        assert_eq!(status_at_column(Confirmed), Some("confirmed_at"));
        assert_eq!(status_at_column(Preparing), Some("preparing_at"));
        assert_eq!(status_at_column(Ready), Some("ready_at"));
        assert_eq!(status_at_column(InDelivery), Some("in_delivery_at"));
        assert_eq!(status_at_column(Delivered), Some("delivered_at"));
        assert_eq!(status_at_column(PickedUp), Some("picked_up_at"));
        assert_eq!(status_at_column(Cancelled), None);
        assert_eq!(status_at_column(Rejected), None);
        assert_eq!(status_at_column(Pending), None);
    }

    #[test]
    fn cancel_and_reject_fire_both_folds() {
        // R2-3 terminalize + L-A refund_due on →CANCELLED and →REJECTED.
        for (cur, new) in [
            (InDelivery, Cancelled),
            (Pending, Rejected),
            (Ready, Cancelled),
        ] {
            let fx = transition_effects(cur, new);
            assert!(fx.terminalize_assignment, "{cur:?}→{new:?} terminalize");
            assert!(fx.record_refund_due, "{cur:?}→{new:?} refund_due");
        }
    }

    #[test]
    fn in_delivery_to_ready_revert_terminalizes_but_records_no_refund() {
        // R2-3 fires (no stranded binding on a downgrade); L-A does NOT (not a terminal-cancel).
        let fx = transition_effects(InDelivery, Ready);
        assert!(fx.terminalize_assignment);
        assert!(!fx.record_refund_due);
        assert_eq!(fx.stamp_column, Some("ready_at"));
    }

    #[test]
    fn confirmed_and_rejected_publish_lifecycle_events() {
        assert_eq!(
            transition_effects(Pending, Confirmed).lifecycle_event,
            Some(LifecycleEvent::OrderConfirmed)
        );
        assert_eq!(
            transition_effects(Pending, Rejected).lifecycle_event,
            Some(LifecycleEvent::OrderRejected)
        );
        assert_eq!(
            transition_effects(Confirmed, Preparing).lifecycle_event,
            None
        );
    }

    #[test]
    fn ordinary_forward_transition_fires_no_terminal_folds() {
        let fx = transition_effects(Confirmed, Preparing);
        assert!(!fx.terminalize_assignment);
        assert!(!fx.record_refund_due);
        assert_eq!(fx.stamp_column, Some("preparing_at"));
    }

    // ── CC-1 strand guard ──

    #[test]
    fn cc1_active_binding_blocks_delivered_and_picked_up() {
        let binding = BindingState {
            has_active_binding: true,
            has_delivered_binding: false,
        };
        for target in [Delivered, PickedUp] {
            assert_eq!(
                cc1_strand_guard(target, InDelivery, binding),
                Err(ErrorCode::AssignmentActive)
            );
        }
    }

    #[test]
    fn cc1_in_delivery_without_delivered_binding_is_use_deliver_flow() {
        let binding = BindingState {
            has_active_binding: false,
            has_delivered_binding: false,
        };
        assert_eq!(
            cc1_strand_guard(Delivered, InDelivery, binding),
            Err(ErrorCode::UseDeliverFlow)
        );
    }

    #[test]
    fn cc1_never_dispatched_order_stays_patchable() {
        // No binding, not IN_DELIVERY → PATCH→PICKED_UP allowed (phone/manual pickup flow).
        let binding = BindingState {
            has_active_binding: false,
            has_delivered_binding: false,
        };
        assert!(cc1_strand_guard(PickedUp, Ready, binding).is_ok());
    }

    #[test]
    fn cc1_delivered_binding_allows_delivered() {
        // completeDelivery path: a delivered assignment exists → the PATCH is allowed.
        let binding = BindingState {
            has_active_binding: false,
            has_delivered_binding: true,
        };
        assert!(cc1_strand_guard(Delivered, InDelivery, binding).is_ok());
    }

    #[test]
    fn cc1_ignores_non_delivered_targets() {
        let binding = BindingState {
            has_active_binding: true,
            has_delivered_binding: false,
        };
        // A →PREPARING is never CC-1-gated even with an active binding.
        assert!(cc1_strand_guard(Preparing, Confirmed, binding).is_ok());
    }

    // ── honest-dispatch gate ──

    #[test]
    fn honest_dispatch_only_for_in_delivery_delivery_orders() {
        assert!(needs_honest_dispatch(InDelivery, true));
        assert!(!needs_honest_dispatch(InDelivery, false)); // pickup type — not dispatched (L2)
        assert!(!needs_honest_dispatch(Ready, true));
    }
}
