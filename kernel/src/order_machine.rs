//! Deterministic order-state machine — the `decide/fold` Law.
//!
//! 1:1 port of `packages/domain/src/order-machine.ts` + `errors.ts` (the TS oracle).
//! The TS app is the legacy oracle; this Rust kernel is the canonical core.
//! WASM/headless safe. No float, no I/O.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

impl OrderStatus {
    /// Parse from the oracle's string form. Unknown strings are rejected (never silently mapped).
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "PENDING" => Some(Self::Pending),
            "CONFIRMED" => Some(Self::Confirmed),
            "PREPARING" => Some(Self::Preparing),
            "READY" => Some(Self::Ready),
            "IN_DELIVERY" => Some(Self::InDelivery),
            "DELIVERED" => Some(Self::Delivered),
            "REJECTED" => Some(Self::Rejected),
            "CANCELLED" => Some(Self::Cancelled),
            "SCHEDULED" => Some(Self::Scheduled),
            "PICKED_UP" => Some(Self::PickedUp),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "PENDING",
            Self::Confirmed => "CONFIRMED",
            Self::Preparing => "PREPARING",
            Self::Ready => "READY",
            Self::InDelivery => "IN_DELIVERY",
            Self::Delivered => "DELIVERED",
            Self::Rejected => "REJECTED",
            Self::Cancelled => "CANCELLED",
            Self::Scheduled => "SCHEDULED",
            Self::PickedUp => "PICKED_UP",
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Delivered | Self::PickedUp | Self::Rejected | Self::Cancelled
        )
    }
}

/// Transition table — identical to the oracle's `TRANSITIONS`.
/// `SCHEDULED` is a scaffold terminal (the scheduled flow is not implemented yet).
fn allowed_next(from: OrderStatus) -> &'static [OrderStatus] {
    use OrderStatus::*;
    match from {
        Pending => &[Confirmed, Rejected, Cancelled],
        Confirmed => &[Preparing, InDelivery],
        Preparing => &[Ready],
        Ready => &[InDelivery, PickedUp],
        InDelivery => &[Delivered],
        Delivered => &[],
        Rejected => &[],
        Cancelled => &[],
        Scheduled => &[],
        PickedUp => &[],
    }
}

fn is_scaffold(s: OrderStatus) -> bool {
    matches!(s, OrderStatus::Scheduled)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionError {
    /// from === to
    SameStatus(OrderStatus),
    /// transition touches a scaffold-only status
    ScaffoldDisabled(OrderStatus, OrderStatus),
    /// not in the allowed transition table
    Illegal(OrderStatus, OrderStatus),
}

impl TransitionError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::SameStatus(_) => "SameStatusError",
            Self::ScaffoldDisabled(_, _) => "ScaffoldDisabledError",
            Self::Illegal(_, _) => "IllegalTransitionError",
        }
    }
    pub fn message(&self) -> String {
        match self {
            Self::SameStatus(s) => format!("Cannot transition to same status: {}", s.as_str()),
            Self::ScaffoldDisabled(f, t) => {
                format!(
                    "Scaffold transition disabled: {} → {}",
                    f.as_str(),
                    t.as_str()
                )
            }
            Self::Illegal(f, t) => format!("Illegal transition: {} → {}", f.as_str(), t.as_str()),
        }
    }
}

/// The `decide/fold` Law: validate a single state transition.
/// Mirrors `assertTransition(from, to)` — throws (returns Err) on invalid.
pub fn assert_transition(from: OrderStatus, to: OrderStatus) -> Result<(), TransitionError> {
    if from == to {
        return Err(TransitionError::SameStatus(from));
    }
    if is_scaffold(to) || is_scaffold(from) {
        return Err(TransitionError::ScaffoldDisabled(from, to));
    }
    let allowed = allowed_next(from);
    if !allowed.contains(&to) {
        return Err(TransitionError::Illegal(from, to));
    }
    Ok(())
}

/// Fold a sequence of transitions into a final status (the `fold` half of the Law).
/// Stops at the first invalid transition and returns the error + the status reached.
/// This is the deterministic reducer the WS event bus replays against.
pub fn fold_transitions(
    start: OrderStatus,
    steps: &[OrderStatus],
) -> Result<OrderStatus, (TransitionError, OrderStatus)> {
    let mut cur = start;
    for &next in steps {
        assert_transition(cur, next).map_err(|e| (e, cur))?;
        cur = next;
    }
    Ok(cur)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── RED: illegal transitions must be rejected ──
    #[test]
    fn red_illegal_pending_to_ready() {
        assert!(matches!(
            assert_transition(OrderStatus::Pending, OrderStatus::Ready),
            Err(TransitionError::Illegal(_, _))
        ));
    }
    #[test]
    fn red_delivered_cannot_move() {
        assert!(matches!(
            assert_transition(OrderStatus::Delivered, OrderStatus::Confirmed),
            Err(TransitionError::Illegal(_, _))
        ));
    }
    #[test]
    fn red_same_status_is_same_error() {
        assert!(matches!(
            assert_transition(OrderStatus::Confirmed, OrderStatus::Confirmed),
            Err(TransitionError::SameStatus(_))
        ));
    }
    #[test]
    fn red_scaffold_scheduled_blocked() {
        assert!(matches!(
            assert_transition(OrderStatus::Pending, OrderStatus::Scheduled),
            Err(TransitionError::ScaffoldDisabled(_, _))
        ));
        assert!(matches!(
            assert_transition(OrderStatus::Scheduled, OrderStatus::Confirmed),
            Err(TransitionError::ScaffoldDisabled(_, _))
        ));
    }

    // ── GREEN: the happy-path lifecycle matches the oracle exactly ──
    #[test]
    fn green_happy_path_pending_to_delivered() {
        let path = [
            OrderStatus::Confirmed,
            OrderStatus::Preparing,
            OrderStatus::Ready,
            OrderStatus::InDelivery,
            OrderStatus::Delivered,
        ];
        assert_eq!(
            fold_transitions(OrderStatus::Pending, &path),
            Ok(OrderStatus::Delivered)
        );
    }
    #[test]
    fn green_pickup_path_ready_to_pickedup() {
        let path = [OrderStatus::PickedUp];
        assert_eq!(
            fold_transitions(OrderStatus::Ready, &path),
            Ok(OrderStatus::PickedUp)
        );
    }
    #[test]
    fn green_reject_and_cancel_terminal() {
        assert_eq!(
            fold_transitions(OrderStatus::Pending, &[OrderStatus::Rejected]),
            Ok(OrderStatus::Rejected)
        );
        assert_eq!(
            fold_transitions(OrderStatus::Pending, &[OrderStatus::Cancelled]),
            Ok(OrderStatus::Cancelled)
        );
    }
    #[test]
    fn green_fold_stops_at_first_illegal() {
        // Confirmed → Preparing → (illegal) Delivered
        let path = [OrderStatus::Preparing, OrderStatus::Delivered];
        let res = fold_transitions(OrderStatus::Confirmed, &path);
        assert!(matches!(
            res,
            Err((TransitionError::Illegal(_, _), OrderStatus::Preparing))
        ));
    }
    #[test]
    fn green_terminal_set_matches_oracle() {
        for s in [
            OrderStatus::Delivered,
            OrderStatus::PickedUp,
            OrderStatus::Rejected,
            OrderStatus::Cancelled,
        ] {
            assert!(s.is_terminal());
        }
        for s in [
            OrderStatus::Pending,
            OrderStatus::Confirmed,
            OrderStatus::Preparing,
            OrderStatus::Ready,
            OrderStatus::InDelivery,
        ] {
            assert!(!s.is_terminal());
        }
    }
}
