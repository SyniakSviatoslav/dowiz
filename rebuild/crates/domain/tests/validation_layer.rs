//! Validation Layer suite (VALIDATION-LAYER-SPEC §"Proof plan") — the properties that tie the
//! boundary gate [`validate`] to the kernel it guards.
//!
//! Where the inline unit tests in `kernel/validate.rs` pin the concrete RED cases (an illegal edge,
//! a negative fee), these prove the LAWS over arbitrary inputs:
//!   - **Totality** — `validate` never panics, for any state × command (Law: total, like `decide`).
//!   - **Machine soundness** — for a transition command, `validate(..).is_ok()` iff the machine
//!     would accept the edge. This is the load-bearing property: an accepted command never trips
//!     `decide`'s `assert_transition` precondition (the spec's proof-plan item 1, scoped to the
//!     machine dimension — the only precondition this first step lifts).
//!   - **Money soundness** — for a `PlaceOrder`, `validate` errs iff some observed fee field is
//!     negative, and every such error is a `NonPositiveMoney` (never a spurious other invariant).
//!
//! Scope note: full cross-`decide` soundness including the pricing/actor preconditions arrives as
//! those invariants land (one at a time). Here the machine soundness is stated over TRANSITION
//! commands only — `PlaceOrder` routes around the machine and its pricing preconditions are not yet
//! lifted, so it is excluded from the machine property by construction.

use domain::{
    ALL_STATUSES, Actor, BindingState, Command, Context, FeeLocation, Invariant, Lek, OrderState,
    OrderStatus, PriceInputs, PricingSnapshot, Ts, assert_transition, validate,
};
use proptest::prelude::*;
use std::collections::HashMap;

const NO_BINDING: BindingState = BindingState {
    has_active_binding: false,
    has_delivered_binding: false,
};

/// A transition context — no pricing authority, so only the machine invariant can fire.
fn plain_ctx() -> Context<'static> {
    Context {
        binding: NO_BINDING,
        refundable_paid: Lek::ZERO,
        pricing: None,
    }
}

fn any_status() -> impl Strategy<Value = OrderStatus> {
    prop::sample::select(ALL_STATUSES.to_vec())
}

/// Any TRANSITION command (kinds 0..9 — `PlaceOrder` is excluded; it is a create/price command with
/// its own context shape, exercised by the money property below).
fn any_transition_command() -> impl Strategy<Value = Command> {
    (0u8..9, any::<i64>(), any::<bool>()).prop_map(|(kind, t, is_owner)| {
        let at = Ts(t);
        let actor = if is_owner { Actor::Owner } else { Actor::System };
        match kind {
            0 => Command::Confirm { at, actor },
            1 => Command::Reject { at, actor },
            2 => Command::StartPreparing { at, actor },
            3 => Command::MarkReady { at, actor },
            4 => Command::Dispatch { at, actor },
            5 => Command::MarkDelivered { at, actor },
            6 => Command::MarkPickedUp { at, actor },
            7 => Command::RevertToReady { at, actor },
            _ => Command::Cancel { at, actor },
        }
    })
}

/// An `Option<i64>` fee field straddling zero — so a proptest run mixes `None`, negatives, `0`, and
/// positives in roughly equal measure (a balanced falsifier for the money property).
fn any_fee_field() -> impl Strategy<Value = Option<i64>> {
    prop_oneof![Just(None), (-1_000i64..1_000).prop_map(Some)]
}

proptest! {
    /// Totality — `validate` completes (never panics) for every state × transition command. The test
    /// completing IS the witness (Law: total, side-effect-free, like `decide`).
    #[test]
    fn validate_is_total_over_state_x_transition_command(
        status in any_status(),
        cmd in any_transition_command(),
    ) {
        let state = OrderState { status, ..OrderState::genesis() };
        // Totality witness: the call RETURNS a value (Ok or Err) rather than panicking — either arm
        // is acceptable here; the point is that the function is total over the whole input space.
        let outcome = validate(&cmd, &state, &plain_ctx());
        prop_assert!(outcome.is_ok() || outcome.is_err());
    }

    /// Machine soundness — the load-bearing property. For any transition command, the gate accepts
    /// exactly the edges the machine accepts: `validate(..).is_ok()` iff `assert_transition` is ok.
    ///
    /// Falsifiable: a `validate` that checked only `can_transition` (and so missed the same-status
    /// reject) would accept a `Confirm`-on-`CONFIRMED` that `assert_transition` rejects, breaking the
    /// biconditional; dropping the machine check entirely would accept every illegal edge.
    #[test]
    fn transition_gate_accepts_exactly_the_machine_edges(
        status in any_status(),
        cmd in any_transition_command(),
    ) {
        let state = OrderState { status, ..OrderState::genesis() };
        let gate_ok = validate(&cmd, &state, &plain_ctx()).is_ok();
        let machine_ok = assert_transition(status, cmd.target()).is_ok();
        prop_assert_eq!(gate_ok, machine_ok, "{:?} @ {:?}: gate {} vs machine {}", cmd, status, gate_ok, machine_ok);
        // And when the gate refuses, it is BECAUSE of the transition — the sole invariant a plain
        // (unpriced) transition context can trip.
        if let Err(violations) = validate(&cmd, &state, &plain_ctx()) {
            prop_assert!(
                violations.iter().all(|v| matches!(v, Invariant::IllegalTransition { .. })),
                "a transition-only refusal must be IllegalTransition, got {:?}", violations
            );
        }
    }

    /// Money soundness — for a `PlaceOrder`, the gate errs iff some observed fee field is negative,
    /// and every such error is a `NonPositiveMoney` (never a spurious machine invariant — `PlaceOrder`
    /// is born PENDING and routes around the machine). Falsifiable: a `validate` that skipped the
    /// money check would return `Ok` on a negative fee and break the `iff`.
    #[test]
    fn place_order_gate_errs_iff_a_fee_is_negative(
        delivery_fee_flat in any_fee_field(),
        free_delivery_threshold in any_fee_field(),
        min_order_value in any_fee_field(),
    ) {
        let product_map = HashMap::new();
        let mod_map = HashMap::new();
        let groups = HashMap::new();
        let ctx = Context {
            binding: NO_BINDING,
            refundable_paid: Lek::ZERO,
            pricing: Some(PriceInputs {
                snapshot: PricingSnapshot {
                    product_map: &product_map,
                    mod_map: &mod_map,
                    groups_by_product: &groups,
                },
                is_pickup: false,
                location: FeeLocation { delivery_fee_flat, free_delivery_threshold, min_order_value },
                distance_m: None,
                tiers: &[],
                rate_micro: 0,
                price_includes_tax: false,
            }),
        };
        let cmd = Command::PlaceOrder { at: Ts(1), actor: Actor::Owner, cart: vec![] };
        let result = validate(&cmd, &OrderState::genesis(), &ctx);

        let any_negative = [delivery_fee_flat, free_delivery_threshold, min_order_value]
            .into_iter()
            .flatten()
            .any(|v| v < 0);
        prop_assert_eq!(result.is_err(), any_negative);
        if let Err(violations) = result {
            prop_assert!(
                violations.iter().all(|v| matches!(v, Invariant::NonPositiveMoney { .. })),
                "PlaceOrder fee refusals must all be NonPositiveMoney, got {:?}", violations
            );
        }
    }
}
