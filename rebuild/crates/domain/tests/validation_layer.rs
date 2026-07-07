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
    ALL_STATUSES, Actor, BindingState, Command, Context, ErrorCode, FeeLocation, Invariant, Lek,
    OrderState, OrderStatus, PriceInputs, PricingItem, PricingSnapshot, Ts, assert_transition,
    compute_order_pricing, validate,
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

/// A `product_map` resolving each id (price immaterial — `validate` checks only presence), so a
/// `PlaceOrder` proptest can be well-formed on the price-reconciliation dimension and isolate the
/// dimension under test.
fn products(ids: &[&str]) -> HashMap<String, domain::ProductInfo> {
    ids.iter()
        .map(|id| {
            (
                id.to_string(),
                domain::ProductInfo {
                    name: (*id).to_string(),
                    price: Lek::ZERO,
                },
            )
        })
        .collect()
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
        // "p1" resolves so the price-reconciliation dimension is well-formed — isolating money.
        let product_map = products(&["p1"]);
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
        // A NON-EMPTY cart so `EmptyLineItems` never fires — this property isolates the money dimension.
        let cart = vec![PricingItem { product_id: "p1".to_string(), quantity: 1, modifier_ids: vec![] }];
        let cmd = Command::PlaceOrder { at: Ts(1), actor: Actor::Owner, cart };
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

    /// EmptyLineItems soundness — a `PlaceOrder` trips `EmptyLineItems` iff its cart is empty
    /// (independent of the — here well-formed — fees). Falsifiable: dropping the check makes an empty
    /// cart pass. A boundary business rule, so this is validate's OWN definition, not a decide mirror.
    #[test]
    fn place_order_flags_empty_cart_iff_empty(line_count in 0usize..4) {
        let cart: Vec<PricingItem> = (0..line_count)
            .map(|i| PricingItem { product_id: format!("p{i}"), quantity: 1, modifier_ids: vec![] })
            .collect();
        let was_empty = cart.is_empty();
        // Every generated line ("p0".."p2") resolves, so ONLY EmptyLineItems can fire.
        let product_map = products(&["p0", "p1", "p2", "p3"]);
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
                is_pickup: true,
                // Well-formed (non-negative) fees so ONLY EmptyLineItems can fire.
                location: FeeLocation { delivery_fee_flat: Some(0), free_delivery_threshold: None, min_order_value: None },
                distance_m: None,
                tiers: &[],
                rate_micro: 0,
                price_includes_tax: false,
            }),
        };
        let cmd = Command::PlaceOrder { at: Ts(1), actor: Actor::Owner, cart };
        let result = validate(&cmd, &OrderState::genesis(), &ctx);
        prop_assert_eq!(result.is_err(), was_empty);
        if let Err(violations) = result {
            prop_assert_eq!(violations, vec![Invariant::EmptyLineItems]);
        }
    }

    /// QuantityOutOfRange soundness — a single-line `PlaceOrder` trips `QuantityOutOfRange { 1, 99 }`
    /// iff the line quantity is outside `[1, 99]` (the shell cart-line Zod contract), independent of
    /// the — here well-formed — fees. Falsifiable: dropping the range check makes an out-of-range
    /// quantity pass.
    #[test]
    fn place_order_flags_quantity_out_of_range_iff_outside_1_99(qty in -10i64..120) {
        // "p1" resolves so the price-reconciliation dimension is well-formed — isolating quantity.
        let product_map = products(&["p1"]);
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
                is_pickup: true,
                location: FeeLocation { delivery_fee_flat: Some(0), free_delivery_threshold: None, min_order_value: None },
                distance_m: None,
                tiers: &[],
                rate_micro: 0,
                price_includes_tax: false,
            }),
        };
        // One non-empty line (so EmptyLineItems never fires) with the generated quantity.
        let cart = vec![PricingItem { product_id: "p1".to_string(), quantity: qty, modifier_ids: vec![] }];
        let cmd = Command::PlaceOrder { at: Ts(1), actor: Actor::Owner, cart };
        let result = validate(&cmd, &OrderState::genesis(), &ctx);
        let out_of_range = !(1..=99).contains(&qty);
        prop_assert_eq!(result.is_err(), out_of_range);
        if let Err(violations) = result {
            prop_assert_eq!(violations, vec![Invariant::QuantityOutOfRange { min: 1, max: 99 }]);
        }
    }

    /// PriceContextMismatch soundness — a `PlaceOrder` trips `PriceContextMismatch` iff some cart
    /// line's product_id is ABSENT from the observed snapshot's `product_map`, which is EXACTLY the
    /// `ProductNotFound` precondition `decide` runs (kernel `price_cart` → `compute_order_pricing`).
    /// Tied to the SAME predicate the kernel composes (`compute_order_pricing`), NOT a re-transcription
    /// of validate's own membership test: with modifiers empty and no groups that fn's ONLY reject path
    /// is `ProductNotFound`, so `validate` errs (with exactly `[PriceContextMismatch]`) iff the real
    /// pricing corridor would reject the cart with `ProductNotFound`. Falsifiable: dropping the
    /// resolution check makes an unknown product pass `validate` while `decide` would still breach.
    #[test]
    fn place_order_flags_price_context_mismatch_iff_a_product_is_unknown(
        // Each line draws from three KNOWN ids + one UNKNOWN ("ghost"); the map registers only the
        // three known ids, so a line hitting "ghost" is the unknown case.
        ids in prop::collection::vec(prop::sample::select(vec!["p0", "p1", "p2", "ghost"]), 1..4),
    ) {
        let product_map = products(&["p0", "p1", "p2"]);
        let mod_map = HashMap::new();
        let groups = HashMap::new();
        let cart: Vec<PricingItem> = ids
            .iter()
            .map(|id| PricingItem { product_id: (*id).to_string(), quantity: 1, modifier_ids: vec![] })
            .collect();

        // The reference model: the SAME corridor `decide` runs. Modifiers empty + no groups ⇒ its ONLY
        // reject path is ProductNotFound, so this isolates the product-existence dimension.
        let corridor = compute_order_pricing(&cart, &PricingSnapshot {
            product_map: &product_map,
            mod_map: &mod_map,
            groups_by_product: &groups,
        });
        let decide_rejects_product_not_found =
            matches!(corridor, Err(ref e) if e.code == ErrorCode::ProductNotFound);

        let ctx = Context {
            binding: NO_BINDING,
            refundable_paid: Lek::ZERO,
            pricing: Some(PriceInputs {
                snapshot: PricingSnapshot {
                    product_map: &product_map,
                    mod_map: &mod_map,
                    groups_by_product: &groups,
                },
                is_pickup: true,
                // Well-formed fees + qty 1 + non-empty cart so ONLY PriceContextMismatch can fire.
                location: FeeLocation { delivery_fee_flat: Some(0), free_delivery_threshold: None, min_order_value: None },
                distance_m: None,
                tiers: &[],
                rate_micro: 0,
                price_includes_tax: false,
            }),
        };
        let cmd = Command::PlaceOrder { at: Ts(1), actor: Actor::Owner, cart };
        let result = validate(&cmd, &OrderState::genesis(), &ctx);
        prop_assert_eq!(result.is_err(), decide_rejects_product_not_found);
        if let Err(violations) = result {
            prop_assert_eq!(violations, vec![Invariant::PriceContextMismatch]);
        }
    }
}
