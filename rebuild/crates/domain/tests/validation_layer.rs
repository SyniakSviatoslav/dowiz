//! Validation Layer suite (VALIDATION-LAYER-SPEC §"Proof plan") — the properties that tie the
//! boundary gate [`validate`] to the kernel it guards.
//!
//! Where the inline unit tests in `kernel/validate.rs` pin the concrete RED cases (an illegal edge,
//! a negative fee), these prove the LAWS over arbitrary inputs:
//!   - **Totality** — `validate` never panics, for any state × command (Law: total, like `decide`).
//!   - **Transition soundness** — for a transition command over any observed binding,
//!     `validate(..).is_ok()` iff the machine accepts the edge AND (for an owner) the actor-gate
//!     authorizes it AND the cc1 strand guard permits it. The load-bearing property: the gate now
//!     accepts EXACTLY `decide`'s transition preconditions. Companion properties isolate the actor
//!     and cc1 dimensions.
//!   - **Money soundness** — for a `PlaceOrder`, `validate` errs iff some observed fee field is
//!     negative, and every such error is a `NonPositiveMoney` (never a spurious other invariant).
//!
//! Scope note: transition-command soundness is now CLOSED (machine + actor-gate + cc1). The one
//! remaining `decide` precondition is the `PlaceOrder` pricing corridor — `PlaceOrder` routes around
//! the machine and its pricing preconditions are not yet lifted, so full `validate.ok ⟺ decide.ok`
//! over ALL commands lands with the pricing invariants.

use domain::{
    ALL_STATUSES, Actor, BindingState, Command, Context, FeeLocation, Invariant, Lek, OrderState,
    OrderStatus, PriceInputs, PricingSnapshot, Ts, assert_transition, validate,
};
// The actor-gate predicate `decide` composes — reused verbatim so the soundness property is tied to
// the SAME source of truth the kernel uses (not a re-transcription of the rule).
use domain::kernel::policy::{assert_owner_target_allowed, cc1_strand_guard};
use proptest::prelude::*;
use std::collections::HashMap;

const NO_BINDING: BindingState = BindingState {
    has_active_binding: false,
    has_delivered_binding: false,
};

/// A transition context — no pricing authority — carrying the given courier-binding facts.
fn ctx_with(binding: BindingState) -> Context<'static> {
    Context {
        binding,
        refundable_paid: Lek::ZERO,
        pricing: None,
    }
}

/// Any courier-binding facts — the 2×2 of (active, delivered) — so cc1 is exercised across every
/// strand state a transition can observe.
fn any_binding() -> impl Strategy<Value = BindingState> {
    (any::<bool>(), any::<bool>()).prop_map(|(has_active_binding, has_delivered_binding)| {
        BindingState {
            has_active_binding,
            has_delivered_binding,
        }
    })
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
        binding in any_binding(),
    ) {
        let state = OrderState { status, ..OrderState::genesis() };
        // Totality witness: the call RETURNS a value (Ok or Err) rather than panicking — either arm
        // is acceptable here; the point is that the function is total over the whole input space
        // (every state × command × observed binding).
        let outcome = validate(&cmd, &state, &ctx_with(binding));
        prop_assert!(outcome.is_ok() || outcome.is_err());
    }

    /// Transition soundness — the load-bearing property, now spanning ALL THREE dimensions the gate
    /// covers. For any transition command over any observed binding, `validate(..).is_ok()` iff the
    /// machine accepts the edge AND (for an owner) the actor-gate authorizes it AND the cc1 strand
    /// guard permits it — i.e. the gate accepts EXACTLY what `decide`'s transition preconditions do.
    ///
    /// Falsifiable per dimension: dropping the same-status check breaks the machine half; dropping the
    /// actor-gate accepts an owner-driven SYSTEM-only cancel; dropping cc1 accepts an IN_DELIVERY→
    /// DELIVERED over a live/undelivered binding. The generated `binding` exercises the cc1 half.
    #[test]
    fn transition_gate_accepts_exactly_the_machine_actor_and_cc1_edges(
        status in any_status(),
        cmd in any_transition_command(),
        binding in any_binding(),
    ) {
        let ctx = ctx_with(binding);
        let state = OrderState { status, ..OrderState::genesis() };
        let gate_ok = validate(&cmd, &state, &ctx).is_ok();
        // The exact conjunction of `decide`'s transition preconditions, each from its own predicate.
        let machine_ok = assert_transition(status, cmd.target()).is_ok();
        let actor_ok = cmd.actor() != Actor::Owner
            || assert_owner_target_allowed(status, cmd.target()).is_ok();
        // cc1 only applies over a machine-legal edge (as decide composes it); on a machine-illegal
        // edge the machine half already forces the conjunction false, so gating cc1 on machine_ok
        // keeps the reference model identical to validate's control flow.
        let cc1_ok = !machine_ok || cc1_strand_guard(cmd.target(), status, binding).is_ok();
        prop_assert_eq!(
            gate_ok, machine_ok && actor_ok && cc1_ok,
            "{:?} @ {:?} binding {:?}: gate {} vs machine&&actor&&cc1 {}",
            cmd, status, binding, gate_ok, machine_ok && actor_ok && cc1_ok
        );
        // And when the gate refuses, it is one of the THREE transition invariants — never a foreign one.
        if let Err(violations) = validate(&cmd, &state, &ctx) {
            prop_assert!(
                violations.iter().all(|v| matches!(v,
                    Invariant::IllegalTransition { .. }
                    | Invariant::ActorNotAuthorized { .. }
                    | Invariant::CourierStrandGuard { .. })),
                "a transition refusal must be IllegalTransition|ActorNotAuthorized|CourierStrandGuard, got {:?}", violations
            );
        }
    }

    /// Actor soundness (the dimension, isolated) — over the machine-LEGAL edges, with a cc1-PERMISSIVE
    /// binding (a `delivered` assignment exists, so cc1 never fires), the gate refuses EXACTLY the
    /// owner-driven edges `assert_owner_target_allowed` forbids, and every such refusal is an
    /// `ActorNotAuthorized`. Conditioned INSIDE the body (a `prop_assume` on the sparse-legal edges
    /// would exhaust proptest's rejection limit). Falsifiable: dropping the actor-gate flips every
    /// owner-forbidden case from `Err`→`Ok`.
    #[test]
    fn owner_gate_matches_the_actor_gate_on_machine_legal_edges(
        status in any_status(),
        cmd in any_transition_command(),
    ) {
        if assert_transition(status, cmd.target()).is_ok() {
            let state = OrderState { status, ..OrderState::genesis() };
            // A delivered binding ⇒ cc1 permits every →DELIVERED/→PICKED_UP, so only the actor
            // dimension can refuse here — isolating this property to the actor-gate.
            let ctx = ctx_with(BindingState { has_active_binding: false, has_delivered_binding: true });
            let result = validate(&cmd, &state, &ctx);
            let actor_forbids = cmd.actor() == Actor::Owner
                && assert_owner_target_allowed(status, cmd.target()).is_err();
            prop_assert_eq!(result.is_err(), actor_forbids);
            if let Err(violations) = result {
                prop_assert!(
                    violations.iter().all(|v| matches!(v, Invariant::ActorNotAuthorized { .. })),
                    "a machine-legal refusal must be ActorNotAuthorized, got {:?}", violations
                );
            }
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
