//! `OrderStatus` — the exhaustive order-status state machine. Ports `packages/domain/src/
//! order-machine.ts` (`assertTransition`/`isTerminal`) verbatim: the 10 live `order_status` Pg
//! enum values (REBUILD-MAP inventory/12 §enums census) and the exact transition table, including
//! the deliver-v2 offer-sweep-cancel addendum (CANCELLED reachable from CONFIRMED/PREPARING/READY
//! as the SYSTEM-only dispatch-exhausted terminal edge) and the IN_DELIVERY revert-to-READY edge
//! (courier cancel/abort/owner-reassign never stranding an order in IN_DELIVERY).
//!
//! `SCHEDULED` is intentionally unreachable in both directions (scaffold status — the scheduled
//! flow isn't implemented yet); this falls out of the transition table naturally (its
//! allowed-target list is empty, and no other state lists it as a target), so no separate
//! "scaffold" gate is needed here — `assert_transition` still classifies it as
//! `ScaffoldDisabled` (not the generic `IllegalTransition`) to preserve the distinct error
//! semantics the Node errors.ts exposes.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum OrderStatus {
    Pending,
    Confirmed,
    Preparing,
    Ready,
    InDelivery,
    Delivered,
    Rejected,
    Cancelled,
    Scheduled,
    PickedUp,
}

/// The full live enum, in the order the migrations census lists it. Used by the exhaustive
/// transition test so "every disallowed pair asserted" is a real 10×10 = 100-pair sweep, not a
/// hand-picked subset.
pub const ALL_STATUSES: [OrderStatus; 10] = [
    OrderStatus::Pending,
    OrderStatus::Confirmed,
    OrderStatus::Preparing,
    OrderStatus::Ready,
    OrderStatus::InDelivery,
    OrderStatus::Delivered,
    OrderStatus::Rejected,
    OrderStatus::Cancelled,
    OrderStatus::Scheduled,
    OrderStatus::PickedUp,
];

/// Pure bool table — no error classification, just "is `from -> to` a legal edge". Same-status
/// (`from == to`) is always `false` here (the Node machine treats it as a distinct
/// `SameStatusError`, surfaced by `assert_transition`, not folded into "illegal").
pub fn can_transition(from: OrderStatus, to: OrderStatus) -> bool {
    use OrderStatus::*;
    if from == to {
        return false;
    }
    match from {
        Pending => matches!(to, Confirmed | Rejected | Cancelled),
        Confirmed => matches!(to, Preparing | InDelivery | Cancelled),
        Preparing => matches!(to, Ready | Cancelled),
        Ready => matches!(to, InDelivery | PickedUp | Cancelled),
        InDelivery => matches!(to, Delivered | Cancelled | Ready),
        Delivered | Rejected | Cancelled | Scheduled | PickedUp => false,
    }
}

/// Ports `assertTransition` exactly, including its precedence (same-status checked first, then
/// scaffold, then the general table) so the distinct error *classes* survive the port, not just
/// the aggregate pass/fail.
pub fn assert_transition(from: OrderStatus, to: OrderStatus) -> Result<(), crate::DomainError> {
    use crate::DomainError;

    if from == to {
        return Err(DomainError::SameStatus(from));
    }
    if to == OrderStatus::Scheduled || from == OrderStatus::Scheduled {
        return Err(DomainError::ScaffoldDisabled { from, to });
    }
    if can_transition(from, to) {
        Ok(())
    } else {
        Err(DomainError::IllegalTransition { from, to })
    }
}

/// Ports `isTerminal` — `PICKED_UP` is a live terminal state for pickup orders (`READY ->
/// PICKED_UP`); `SCHEDULED` is scaffold, not terminal (it is simply never entered).
pub fn is_terminal(status: OrderStatus) -> bool {
    matches!(
        status,
        OrderStatus::Delivered
            | OrderStatus::PickedUp
            | OrderStatus::Rejected
            | OrderStatus::Cancelled
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use OrderStatus::*;

    /// The exact mirror of `TRANSITIONS` in `packages/domain/src/order-machine.ts` — kept as a
    /// single source inside the test so any drift in `can_transition` fails loudly against a
    /// literal transcription of the Node table, not against `can_transition`'s own logic.
    fn expected_allowed(from: OrderStatus) -> &'static [OrderStatus] {
        match from {
            Pending => &[Confirmed, Rejected, Cancelled],
            Confirmed => &[Preparing, InDelivery, Cancelled],
            Preparing => &[Ready, Cancelled],
            Ready => &[InDelivery, PickedUp, Cancelled],
            InDelivery => &[Delivered, Cancelled, Ready],
            Delivered | Rejected | Cancelled | Scheduled | PickedUp => &[],
        }
    }

    /// Exhaustive: all 10×10 = 100 ordered pairs, allowed AND disallowed, asserted individually.
    #[test]
    fn exhaustive_transition_table() {
        let mut checked = 0;
        for &from in ALL_STATUSES.iter() {
            let allowed = expected_allowed(from);
            for &to in ALL_STATUSES.iter() {
                let expect_ok = from != to && allowed.contains(&to);
                assert_eq!(
                    can_transition(from, to),
                    expect_ok,
                    "transition {from:?} -> {to:?} expected {expect_ok}"
                );
                checked += 1;
            }
        }
        assert_eq!(
            checked, 100,
            "exhaustive sweep must cover exactly 10x10 ordered pairs"
        );
    }

    #[test]
    fn assert_transition_classifies_same_status() {
        assert_eq!(
            assert_transition(Pending, Pending),
            Err(crate::DomainError::SameStatus(Pending))
        );
    }

    #[test]
    fn assert_transition_classifies_scaffold_both_directions() {
        assert_eq!(
            assert_transition(Pending, Scheduled),
            Err(crate::DomainError::ScaffoldDisabled {
                from: Pending,
                to: Scheduled
            })
        );
        assert_eq!(
            assert_transition(Scheduled, Pending),
            Err(crate::DomainError::ScaffoldDisabled {
                from: Scheduled,
                to: Pending
            })
        );
    }

    #[test]
    fn assert_transition_classifies_illegal_vs_ok() {
        assert_eq!(
            assert_transition(Delivered, Pending),
            Err(crate::DomainError::IllegalTransition {
                from: Delivered,
                to: Pending
            })
        );
        assert!(assert_transition(Pending, Confirmed).is_ok());
    }

    #[test]
    fn deliver_v2_cancel_addendum_reachable() {
        // The offer-sweep-cancel addendum: CANCELLED is a SYSTEM-only dispatch-exhausted
        // terminal edge from CONFIRMED/PREPARING/READY, not just from PENDING.
        assert!(can_transition(Confirmed, Cancelled));
        assert!(can_transition(Preparing, Cancelled));
        assert!(can_transition(Ready, Cancelled));
    }

    #[test]
    fn in_delivery_can_revert_to_ready() {
        // Courier cancel/abort/owner-reassign reverts a force-driven IN_DELIVERY back to READY
        // rather than stranding the order.
        assert!(can_transition(InDelivery, Ready));
    }

    #[test]
    fn is_terminal_matches_node() {
        for &status in ALL_STATUSES.iter() {
            let expected = matches!(status, Delivered | PickedUp | Rejected | Cancelled);
            assert_eq!(is_terminal(status), expected, "is_terminal({status:?})");
        }
    }

    #[test]
    fn serde_round_trip_uses_screaming_snake_case() {
        let json = serde_json::to_string(&InDelivery).unwrap();
        assert_eq!(json, "\"IN_DELIVERY\"");
        let decoded: OrderStatus = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, InDelivery);

        let json = serde_json::to_string(&PickedUp).unwrap();
        assert_eq!(json, "\"PICKED_UP\"");
    }
}
