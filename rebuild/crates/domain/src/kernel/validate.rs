//! The Validation Layer — the invariant gate the orchestrator/shell runs BEFORE [`super::decide`].
//!
//! Operator 2026-07-07: *"a validation layer between the orchestrator and the core — a small Rust
//! function that checks not just format (syntax) but logical invariants."* The deterministic-core
//! thesis: a rejected command is a **bug caught at the boundary**, never a bad state reaching
//! [`decide`](super::decide). Spec: `docs/design/sovereign-core-mvp/VALIDATION-LAYER-SPEC.md`.
//!
//! ## Where it slots
//! `orchestrator/shell → validate(cmd, &state, &ctx) → kernel::decide(&state, cmd, &ctx)`.
//! [`decide`](super::decide) stays pure and UNCHANGED; `validate` is the enforced contract at the
//! seam — [`decide`]'s preconditions lifted BEFORE the transition and returned as DATA (a `Vec` of
//! every violated [`Invariant`], not an internal first-fail early-return) so the orchestrator gets
//! the full picture in one round-trip.
//!
//! ## Scope (extended one invariant at a time)
//! Six invariants so far (VALIDATION-LAYER-SPEC — each landed with its own RED case first), across
//! two disjoint command families.
//!
//! TRANSITION commands mirror `decide`'s preconditions EXACTLY (soundness is a biconditional):
//!   - [`Invariant::IllegalTransition`] — `(state, cmd)` is a legal edge of the order state machine.
//!   - [`Invariant::ActorNotAuthorized`] — the actor may drive this (machine-legal) edge; the
//!     AUTHORIZATION layer OVER the machine (`policy::assert_owner_target_allowed`).
//!   - [`Invariant::CourierStrandGuard`] — over OBSERVED context: a →DELIVERED/→PICKED_UP is refused
//!     while a courier binding is live/undelivered (`policy::cc1_strand_guard`).
//!
//! `PlaceOrder` (the CREATE+PRICE door) gets BOUNDARY rules, stricter than `decide`'s permissive pricing:
//!   - [`Invariant::NonPositiveMoney`] — the raw-`i64` money on the observed pricing authority is
//!     non-negative minor units.
//!   - [`Invariant::EmptyLineItems`] — the cart carries at least one line item.
//!   - [`Invariant::QuantityOutOfRange`] — every line quantity is in `[1, 99]` (the shell cart-line
//!     Zod contract).
//!
//! The remaining invariants sketched in the spec (`PriceContextMismatch`, `IdempotencyKeyMissing`) are
//! appended one at a time, each with its own RED case first. [`Invariant`] is `#[non_exhaustive]` so
//! that growth is not a breaking change for the shell crate.

use super::{Actor, Command, Context, OrderState};
use crate::{ErrorCode, OrderStatus, order_status::assert_transition};

/// The per-line quantity bounds the orchestrator enforces, mirroring the shell cart-line Zod schema
/// `z.number().int().positive().max(99)` (`packages/shared-types/src/legacy.ts:34`). A BOUNDARY rule:
/// the pure core is looser (it accepts `0` and rejects only negatives, via `checked_mul_qty`), so
/// these bounds are the shell's contract, not a lift of a `decide` precondition.
const MIN_LINE_QUANTITY: i64 = 1;
const MAX_LINE_QUANTITY: i64 = 99;

/// A named LOGICAL invariant the boundary enforces — each is a rule, not a serde/format error.
///
/// Modelled on [`crate::DomainError`] (an internal decision type): it derives `Copy`/`Eq` and does
/// NOT derive serde. The core speaks the domain's own vocabulary; when the shell needs to hand a
/// violation to the (TypeScript) orchestrator it maps to a wire code, exactly as
/// [`DomainError::code`](crate::DomainError::code) does — the core stays serde-free on this type.
///
/// `#[non_exhaustive]`: future steps append variants (the spec's `ActorNotAuthorized`,
/// `PriceContextMismatch`, …) without breaking any downstream match.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Invariant {
    // ── format-plus (structural but semantic — beyond serde) ──
    /// A money field is not a non-negative integer in minor units. Only the raw-`i64` money that
    /// ESCAPED the [`Lek`](crate::Lek) type at the shell seam is checked here — [`Lek`] fields
    /// (product prices, tier fees) are non-negative BY CONSTRUCTION, so the type is their proof.
    /// `field` names the offending input (a stable `&'static str`, e.g. `"delivery_fee_flat"`).
    NonPositiveMoney { field: &'static str },
    /// A [`PlaceOrder`](Command::PlaceOrder) carries an EMPTY cart — an order must have at least one
    /// line item. A BOUNDARY business rule the orchestrator enforces: [`decide`](super::decide)'s
    /// pricing corridor is permissive (an empty cart prices to zero), so this is stricter than
    /// `decide`, not a lift of one of its preconditions.
    EmptyLineItems,
    /// A [`PlaceOrder`](Command::PlaceOrder) line carries a quantity outside `[min, max]`. A BOUNDARY
    /// business rule mirroring the shell cart-line Zod schema `z.number().int().positive().max(99)`
    /// (`packages/shared-types/src/legacy.ts:34`) — stricter than the pure core (which accepts `0` and
    /// rejects only negatives via `checked_mul_qty`). `min`/`max` are the enforced bounds (`1`/`99`).
    QuantityOutOfRange { min: i64, max: i64 },
    // ── logical / business invariant ──
    /// `(state, cmd)` is not a legal edge of the order state machine — the union of the machine's
    /// reject classes (illegal edge | same-status | scaffold-disabled), lifted before the
    /// transition. `from` = the current status, `cmd` = the attempted command (SCREAMING_SNAKE
    /// wire names). Mirrors [`decide`](super::decide)'s `assert_transition` precondition.
    IllegalTransition {
        from: &'static str,
        cmd: &'static str,
    },
    /// The actor is not authorized to drive this (machine-legal) edge — the AUTHORIZATION layer OVER
    /// the machine (`policy::assert_owner_target_allowed`). Today: an [`Owner`](super::Actor::Owner)
    /// may not drive the SYSTEM-only widened cancel edges (CONFIRMED/PREPARING/READY→CANCELLED, the
    /// dispatch-grace path); [`System`](super::Actor::System) keeps them. Evaluated ONLY for a
    /// machine-LEGAL edge (the gate sits over a real edge), mirroring where [`decide`](super::decide)
    /// composes it (machine first, then actor-gate). `actor`/`cmd` = SCREAMING_SNAKE wire names.
    ActorNotAuthorized {
        actor: &'static str,
        cmd: &'static str,
    },
    /// The courier-binding STRAND guard forbids this →DELIVERED/→PICKED_UP transition (CC-1) — reusing
    /// `policy::cc1_strand_guard` over the OBSERVED [`ctx.binding`](super::Context). No order is marked
    /// done while a courier is still strand-bound. `reason` = `"ACTIVE_BINDING"` (a live binding exists
    /// → complete via the deliver flow) or `"REQUIRES_DELIVER_FLOW"` (IN_DELIVERY with no delivered
    /// binding). Evaluated ONLY on a machine-LEGAL edge, mirroring where [`decide`](super::decide)
    /// composes cc1 (after the machine + actor-gate). The first invariant that reads observed context,
    /// not just the command.
    CourierStrandGuard { reason: &'static str },
}

/// Validate a command against the current state and observed context BEFORE [`decide`](super::decide).
///
/// TOTAL, side-effect-free, no panics, no I/O (Laws 1–3) — like [`decide`], it reads only its
/// arguments. Returns `Ok(())` when [`decide`] may run, or `Err` carrying EVERY violated
/// [`Invariant`] (not first-fail). Soundness (proven in `tests/validation_layer.rs`, one dimension
/// per invariant): for a transition command, `validate(..).is_ok()` iff the machine accepts the edge
/// AND the actor-gate authorizes it AND the cc1 strand guard permits it — i.e. the gate now covers
/// ALL of [`decide`]'s transition preconditions, so an accepted transition never trips `decide`. For
/// a `PlaceOrder` the gate enforces BOUNDARY business rules (non-negative fee money, non-empty cart)
/// that are stricter than `decide`'s permissive pricing corridor; the remaining pricing-resolution
/// dimension arrives with its invariants.
pub fn validate(cmd: &Command, state: &OrderState, ctx: &Context<'_>) -> Result<(), Vec<Invariant>> {
    let mut violations = Vec::new();

    // Two disjoint command families. The CREATE+PRICE door (`PlaceOrder`) gets the cart/pricing
    // format-plus checks; every other command drives a state-machine TRANSITION and gets the
    // machine → actor-gate → cc1 composition — the same predicates `decide` composes, in that order.
    if let Command::PlaceOrder { cart, .. } = cmd {
        // Business invariant: an order must carry at least one line item. `decide`'s pricing corridor
        // is PERMISSIVE (an empty cart prices to zero), so this is a boundary RULE the orchestrator
        // enforces — stricter than `decide`, not a lift of a `decide` precondition.
        if cart.is_empty() {
            violations.push(Invariant::EmptyLineItems);
        }
        // Boundary rule: every line quantity is in `[MIN, MAX]` (the shell cart-line Zod contract).
        // Reported ONCE (the variant names the bounds, not the offending line), so N bad lines don't
        // produce N identical entries; an empty cart has no lines, so this never fires alongside
        // `EmptyLineItems` for the same order.
        if cart
            .iter()
            .any(|item| item.quantity < MIN_LINE_QUANTITY || item.quantity > MAX_LINE_QUANTITY)
        {
            violations.push(Invariant::QuantityOutOfRange {
                min: MIN_LINE_QUANTITY,
                max: MAX_LINE_QUANTITY,
            });
        }
        // Format-plus: the observed pricing authority's raw-`i64` money is non-negative minor units
        // (the money that crossed the seam as a bare integer, escaping the `Lek` type), independent
        // of which pricing branch a given cart takes — a negative fee/threshold/min-order is
        // malformed money regardless.
        if let Some(pricing) = ctx.pricing.as_ref() {
            check_fee_money_non_negative(&pricing.location, &mut violations);
        }
    } else {
        // A TRANSITION command — the machine is the sole legality authority; the actor-gate and cc1
        // strand guard compose OVER a machine-legal edge (mirroring `decide`'s order).
        let from = state.status;
        let to = cmd.target();
        if assert_transition(from, to).is_err() {
            violations.push(Invariant::IllegalTransition {
                from: status_name(from),
                cmd: command_name(cmd),
            });
        } else {
            // A machine-LEGAL edge — the layers OVER the machine now apply, in the order `decide`
            // composes them (machine → actor-gate → cc1). Both are collected (not first-fail); they
            // are mutually exclusive by target in practice (owner-forbidden edges target CANCELLED,
            // cc1 targets DELIVERED/PICKED_UP), so at most one fires — but the Vec is honest about it.

            // The AUTHORIZATION layer OVER the machine: the deliver-v2 sweep made
            // CONFIRMED/PREPARING/READY→CANCELLED machine-legal but SYSTEM-only; an owner is refused
            // there while `System` (dispatch-grace) keeps it.
            if cmd.actor() == Actor::Owner
                && super::policy::assert_owner_target_allowed(from, to).is_err()
            {
                violations.push(Invariant::ActorNotAuthorized {
                    actor: actor_name(cmd.actor()),
                    cmd: command_name(cmd),
                });
            }

            // The CC-1 STRAND guard, read over the OBSERVED `ctx.binding`: a →DELIVERED/→PICKED_UP
            // with a live (or IN_DELIVERY-but-undelivered) courier binding is refused, so no order is
            // marked done while a courier is still strand-bound. A no-op for every other target.
            if let Err(code) = super::policy::cc1_strand_guard(to, from, ctx.binding) {
                violations.push(Invariant::CourierStrandGuard {
                    reason: cc1_reason(code),
                });
            }
        }
    }

    if violations.is_empty() {
        Ok(())
    } else {
        Err(violations)
    }
}

/// The three raw-`i64` money fields on [`FeeLocation`](super::pricing::FeeLocation) — the money that
/// crosses the shell seam as a bare integer rather than a [`Lek`](crate::Lek). Each, when present,
/// must be `>= 0` (a `0` fee / `0` free-delivery threshold / `0` min-order are all legitimate, so
/// the rule is non-negative, not strictly positive). A negative one is a [`Invariant::NonPositiveMoney`].
fn check_fee_money_non_negative(location: &super::pricing::FeeLocation, out: &mut Vec<Invariant>) {
    for (field, value) in [
        ("delivery_fee_flat", location.delivery_fee_flat),
        ("free_delivery_threshold", location.free_delivery_threshold),
        ("min_order_value", location.min_order_value),
    ] {
        if let Some(v) = value {
            if v < 0 {
                out.push(Invariant::NonPositiveMoney { field });
            }
        }
    }
}

/// The SCREAMING_SNAKE wire name of a status (matches the `OrderStatus` serde rename) — carried on
/// [`Invariant::IllegalTransition`] as a stable, allocation-free label.
fn status_name(status: OrderStatus) -> &'static str {
    match status {
        OrderStatus::Pending => "PENDING",
        OrderStatus::Confirmed => "CONFIRMED",
        OrderStatus::Preparing => "PREPARING",
        OrderStatus::Ready => "READY",
        OrderStatus::InDelivery => "IN_DELIVERY",
        OrderStatus::Delivered => "DELIVERED",
        OrderStatus::Rejected => "REJECTED",
        OrderStatus::Cancelled => "CANCELLED",
        OrderStatus::Scheduled => "SCHEDULED",
        OrderStatus::PickedUp => "PICKED_UP",
    }
}

/// The stable label for a CC-1 strand-guard refusal, mapped from the `ErrorCode`
/// [`cc1_strand_guard`](super::policy::cc1_strand_guard) returns. That fn yields ONLY
/// [`AssignmentActive`](ErrorCode::AssignmentActive) or [`UseDeliverFlow`](ErrorCode::UseDeliverFlow);
/// the catch-all is an unreachable-today backstop that keeps `validate` TOTAL (never panics) if the
/// upstream guard ever grows a new code.
fn cc1_reason(code: ErrorCode) -> &'static str {
    match code {
        ErrorCode::AssignmentActive => "ACTIVE_BINDING",
        ErrorCode::UseDeliverFlow => "REQUIRES_DELIVER_FLOW",
        _ => "STRAND_GUARD",
    }
}

/// The SCREAMING_SNAKE wire name of an actor (matches the `Actor` serde rename) — carried on
/// [`Invariant::ActorNotAuthorized`].
fn actor_name(actor: Actor) -> &'static str {
    match actor {
        Actor::Owner => "OWNER",
        Actor::System => "SYSTEM",
    }
}

/// The SCREAMING_SNAKE wire name of a command (matches the `Command` serde tag rename).
fn command_name(cmd: &Command) -> &'static str {
    match cmd {
        Command::Confirm { .. } => "CONFIRM",
        Command::Reject { .. } => "REJECT",
        Command::StartPreparing { .. } => "START_PREPARING",
        Command::MarkReady { .. } => "MARK_READY",
        Command::Dispatch { .. } => "DISPATCH",
        Command::MarkDelivered { .. } => "MARK_DELIVERED",
        Command::MarkPickedUp { .. } => "MARK_PICKED_UP",
        Command::RevertToReady { .. } => "REVERT_TO_READY",
        Command::Cancel { .. } => "CANCEL",
        Command::PlaceOrder { .. } => "PLACE_ORDER",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Actor, BindingState, FeeLocation, Lek, PriceInputs, PricingSnapshot, Ts,
        kernel::pricing::PricingItem,
    };
    use std::collections::HashMap;

    const T: Ts = Ts(1_700_000_000_000);
    const NO_BINDING: BindingState = BindingState {
        has_active_binding: false,
        has_delivered_binding: false,
    };

    /// A transition context — no pricing authority (transitions do not price), so only the machine
    /// invariant can fire.
    fn plain_ctx() -> Context<'static> {
        Context {
            binding: NO_BINDING,
            refundable_paid: Lek::ZERO,
            pricing: None,
        }
    }

    /// A `PlaceOrder` context whose `FeeLocation` carries the given raw-i64 money fields. The
    /// snapshot maps are empty — `validate` reads only `ctx.pricing.location`, never prices.
    fn place_order_ctx<'a>(
        product_map: &'a HashMap<String, crate::ProductInfo>,
        mod_map: &'a HashMap<String, crate::ModifierInfo>,
        groups: &'a HashMap<String, Vec<crate::GroupInfo>>,
        location: FeeLocation,
    ) -> Context<'a> {
        Context {
            binding: NO_BINDING,
            refundable_paid: Lek::ZERO,
            pricing: Some(PriceInputs {
                snapshot: PricingSnapshot {
                    product_map,
                    mod_map,
                    groups_by_product: groups,
                },
                is_pickup: false,
                location,
                distance_m: None,
                tiers: &[],
                rate_micro: 0,
                price_includes_tax: false,
            }),
        }
    }

    /// A minimal well-formed cart (one line, quantity 1) — `validate` never prices, so the
    /// product_id need not resolve against any snapshot; it only needs to be non-empty.
    fn nonempty_cart() -> Vec<PricingItem> {
        vec![PricingItem {
            product_id: "p1".to_string(),
            quantity: 1,
            modifier_ids: vec![],
        }]
    }

    fn place_order() -> Command {
        Command::PlaceOrder {
            at: T,
            actor: Actor::Owner,
            cart: nonempty_cart(),
        }
    }

    fn place_order_empty_cart() -> Command {
        Command::PlaceOrder {
            at: T,
            actor: Actor::Owner,
            cart: Vec::<PricingItem>::new(),
        }
    }

    // ─────────────────────────── RED case #1: illegal machine edge ───────────────────────────

    /// READY + `Confirm` targets CONFIRMED, and READY→CONFIRMED is not a machine edge → the boundary
    /// reports it as an `IllegalTransition` carrying the wire names. RED proof: a `validate` that
    /// skipped the `assert_transition` check would return `Ok(())` here and red this assertion.
    #[test]
    fn illegal_edge_is_reported_as_illegal_transition() {
        let ready = OrderState {
            status: OrderStatus::Ready,
            ..OrderState::genesis()
        };
        assert_eq!(
            validate(
                &Command::Confirm {
                    at: T,
                    actor: Actor::Owner
                },
                &ready,
                &plain_ctx()
            ),
            Err(vec![Invariant::IllegalTransition {
                from: "READY",
                cmd: "CONFIRM",
            }])
        );
    }

    /// The machine's other two reject classes fold into `IllegalTransition` too (validate is a clean
    /// superset of the machine's precondition): a same-status command and a scaffold edge both fail.
    #[test]
    fn same_status_and_scaffold_also_fold_into_illegal_transition() {
        // Same-status: a Confirm on an already-CONFIRMED order (target == from).
        let confirmed = OrderState {
            status: OrderStatus::Confirmed,
            ..OrderState::genesis()
        };
        assert_eq!(
            validate(
                &Command::Confirm {
                    at: T,
                    actor: Actor::Owner
                },
                &confirmed,
                &plain_ctx()
            ),
            Err(vec![Invariant::IllegalTransition {
                from: "CONFIRMED",
                cmd: "CONFIRM",
            }])
        );
    }

    // ─────────────────────────── RED case #2: negative money at the seam ───────────────────────────

    /// A negative `delivery_fee_flat` on the observed pricing authority is malformed money → the
    /// boundary reports `NonPositiveMoney { field: "delivery_fee_flat" }` BEFORE `decide` ever prices
    /// (where it would otherwise surface late as a pricing `CorridorBreach`). RED proof: a `validate`
    /// that skipped the money check would return `Ok(())` here.
    #[test]
    fn negative_delivery_fee_flat_is_non_positive_money() {
        let product_map = HashMap::new();
        let mod_map = HashMap::new();
        let groups = HashMap::new();
        let ctx = place_order_ctx(
            &product_map,
            &mod_map,
            &groups,
            FeeLocation {
                delivery_fee_flat: Some(-1),
                free_delivery_threshold: None,
                min_order_value: None,
            },
        );
        assert_eq!(
            validate(&place_order(), &OrderState::genesis(), &ctx),
            Err(vec![Invariant::NonPositiveMoney {
                field: "delivery_fee_flat"
            }])
        );
    }

    /// Every violated invariant is returned (a `Vec`, not first-fail): two negative fee fields yield
    /// BOTH `NonPositiveMoney` variants, in field order. RED proof: a first-fail `validate` (early
    /// `return Err` on the first violation) would return only the first and red this assertion.
    #[test]
    fn all_money_violations_are_returned_not_first_fail() {
        let product_map = HashMap::new();
        let mod_map = HashMap::new();
        let groups = HashMap::new();
        let ctx = place_order_ctx(
            &product_map,
            &mod_map,
            &groups,
            FeeLocation {
                delivery_fee_flat: Some(-5),
                free_delivery_threshold: Some(-3),
                min_order_value: None,
            },
        );
        assert_eq!(
            validate(&place_order(), &OrderState::genesis(), &ctx),
            Err(vec![
                Invariant::NonPositiveMoney {
                    field: "delivery_fee_flat"
                },
                Invariant::NonPositiveMoney {
                    field: "free_delivery_threshold"
                },
            ])
        );
    }

    // ─────────────────────────── GREEN: legal inputs pass the gate ───────────────────────────

    /// A legal machine edge with no pricing authority passes. RED proof: a `validate` that always
    /// returned `Err` would red this — the green half of the two-sided falsifiability.
    #[test]
    fn a_legal_transition_passes() {
        assert_eq!(
            validate(
                &Command::Confirm {
                    at: T,
                    actor: Actor::Owner
                },
                &OrderState::genesis(), // PENDING → CONFIRMED is a legal edge
                &plain_ctx()
            ),
            Ok(())
        );
    }

    /// A `PlaceOrder` with well-formed (non-negative) fee money passes — `0` fees are legitimate, and
    /// `PlaceOrder` is not machine-gated (born PENDING).
    #[test]
    fn a_place_order_with_well_formed_fees_passes() {
        let product_map = HashMap::new();
        let mod_map = HashMap::new();
        let groups = HashMap::new();
        let ctx = place_order_ctx(
            &product_map,
            &mod_map,
            &groups,
            FeeLocation {
                delivery_fee_flat: Some(0),
                free_delivery_threshold: Some(50_000),
                min_order_value: Some(0),
            },
        );
        assert_eq!(validate(&place_order(), &OrderState::genesis(), &ctx), Ok(()));
    }

    // ─────────────── RED case: PlaceOrder with an empty cart (boundary business rule) ───────────────

    /// A `PlaceOrder` with an EMPTY cart → `EmptyLineItems` (an order must carry ≥1 line item). A
    /// boundary rule: `decide` prices an empty cart to zero, so a `validate` that skipped the check
    /// would return `Ok(())` here. RED proof.
    #[test]
    fn empty_cart_is_empty_line_items() {
        let product_map = HashMap::new();
        let mod_map = HashMap::new();
        let groups = HashMap::new();
        // Well-formed fees so ONLY the empty-cart rule fires.
        let ctx = place_order_ctx(
            &product_map,
            &mod_map,
            &groups,
            FeeLocation {
                delivery_fee_flat: Some(0),
                free_delivery_threshold: None,
                min_order_value: None,
            },
        );
        assert_eq!(
            validate(&place_order_empty_cart(), &OrderState::genesis(), &ctx),
            Err(vec![Invariant::EmptyLineItems])
        );
    }

    /// Both PlaceOrder format-plus violations are returned together (a `Vec`, not first-fail): an
    /// empty cart AND a negative fee → `[EmptyLineItems, NonPositiveMoney]` (cart checked first).
    #[test]
    fn empty_cart_and_negative_fee_return_both_violations() {
        let product_map = HashMap::new();
        let mod_map = HashMap::new();
        let groups = HashMap::new();
        let ctx = place_order_ctx(
            &product_map,
            &mod_map,
            &groups,
            FeeLocation {
                delivery_fee_flat: Some(-1),
                free_delivery_threshold: None,
                min_order_value: None,
            },
        );
        assert_eq!(
            validate(&place_order_empty_cart(), &OrderState::genesis(), &ctx),
            Err(vec![
                Invariant::EmptyLineItems,
                Invariant::NonPositiveMoney {
                    field: "delivery_fee_flat",
                },
            ])
        );
    }

    // ─────────────── RED case: PlaceOrder line quantity out of [1, 99] (boundary rule) ───────────────

    fn place_order_qty(quantity: i64) -> Command {
        Command::PlaceOrder {
            at: T,
            actor: Actor::Owner,
            cart: vec![PricingItem {
                product_id: "p1".to_string(),
                quantity,
                modifier_ids: vec![],
            }],
        }
    }

    fn well_formed_place_order_ctx<'a>(
        product_map: &'a HashMap<String, crate::ProductInfo>,
        mod_map: &'a HashMap<String, crate::ModifierInfo>,
        groups: &'a HashMap<String, Vec<crate::GroupInfo>>,
    ) -> Context<'a> {
        place_order_ctx(
            product_map,
            mod_map,
            groups,
            FeeLocation {
                delivery_fee_flat: Some(0),
                free_delivery_threshold: None,
                min_order_value: None,
            },
        )
    }

    /// A line quantity outside `[1, 99]` → `QuantityOutOfRange { 1, 99 }` (the shell Zod
    /// `.positive().max(99)` contract). Covers the floor (`0`, negative) and the ceiling (`>99`). RED
    /// proof: a `validate` that skipped the range check would return `Ok(())` for these.
    #[test]
    fn out_of_range_quantities_are_flagged() {
        let (product_map, mod_map, groups) = (HashMap::new(), HashMap::new(), HashMap::new());
        let ctx = well_formed_place_order_ctx(&product_map, &mod_map, &groups);
        for bad in [0, -1, 100, 1_000] {
            assert_eq!(
                validate(&place_order_qty(bad), &OrderState::genesis(), &ctx),
                Err(vec![Invariant::QuantityOutOfRange { min: 1, max: 99 }]),
                "quantity {bad} must be out of range"
            );
        }
    }

    /// The inclusive boundary values `1` and `99` (and a mid value) are IN range → no violation.
    #[test]
    fn boundary_quantities_1_and_99_are_in_range() {
        let (product_map, mod_map, groups) = (HashMap::new(), HashMap::new(), HashMap::new());
        let ctx = well_formed_place_order_ctx(&product_map, &mod_map, &groups);
        for good in [1, 50, 99] {
            assert_eq!(
                validate(&place_order_qty(good), &OrderState::genesis(), &ctx),
                Ok(()),
                "quantity {good} must be in range"
            );
        }
    }

    /// A single out-of-range line flags ONCE (the variant names the bounds, not the line), even
    /// among valid lines — no duplicate entries.
    #[test]
    fn quantity_out_of_range_reported_once_across_lines() {
        let cmd = Command::PlaceOrder {
            at: T,
            actor: Actor::Owner,
            cart: vec![
                PricingItem {
                    product_id: "p1".to_string(),
                    quantity: 2,
                    modifier_ids: vec![],
                },
                PricingItem {
                    product_id: "p2".to_string(),
                    quantity: 0,
                    modifier_ids: vec![],
                },
                PricingItem {
                    product_id: "p3".to_string(),
                    quantity: 200,
                    modifier_ids: vec![],
                },
            ],
        };
        let (product_map, mod_map, groups) = (HashMap::new(), HashMap::new(), HashMap::new());
        let ctx = well_formed_place_order_ctx(&product_map, &mod_map, &groups);
        assert_eq!(
            validate(&cmd, &OrderState::genesis(), &ctx),
            Err(vec![Invariant::QuantityOutOfRange { min: 1, max: 99 }])
        );
    }

    // ─────────────── RED case #3: actor not authorized on a machine-legal edge ───────────────

    /// The deliver-v2 sweep widened the MACHINE to permit CONFIRMED/PREPARING/READY→CANCELLED, but
    /// those are SYSTEM-only edges: an OWNER driving one is machine-legal yet unauthorized → the
    /// boundary reports `ActorNotAuthorized` (never `IllegalTransition` — the edge IS a real edge).
    /// RED proof: a `validate` that skipped the actor-gate would return `Ok(())` here.
    #[test]
    fn owner_on_a_system_only_cancel_edge_is_actor_not_authorized() {
        for from in [
            OrderStatus::Confirmed,
            OrderStatus::Preparing,
            OrderStatus::Ready,
        ] {
            let state = OrderState {
                status: from,
                ..OrderState::genesis()
            };
            assert_eq!(
                validate(
                    &Command::Cancel {
                        at: T,
                        actor: Actor::Owner
                    },
                    &state,
                    &plain_ctx()
                ),
                Err(vec![Invariant::ActorNotAuthorized {
                    actor: "OWNER",
                    cmd: "CANCEL",
                }]),
                "owner must be actor-gated off {from:?}→CANCELLED"
            );
        }
    }

    /// The gate is PRECISE, not a blanket owner-cancel ban: `System` keeps the widened cancel edges
    /// (dispatch-grace), and an owner keeps the cancels it IS authorized for (PENDING→CANCELLED
    /// pre-confirm, IN_DELIVERY→CANCELLED no-show).
    #[test]
    fn system_keeps_and_owner_keeps_authorized_cancels() {
        let confirmed = OrderState {
            status: OrderStatus::Confirmed,
            ..OrderState::genesis()
        };
        // System drives the SYSTEM-only edge → passes the gate (actor-gate applies to owners only).
        assert_eq!(
            validate(
                &Command::Cancel {
                    at: T,
                    actor: Actor::System
                },
                &confirmed,
                &plain_ctx()
            ),
            Ok(())
        );
        // Owner keeps PENDING→CANCELLED (pre-confirm).
        assert_eq!(
            validate(
                &Command::Cancel {
                    at: T,
                    actor: Actor::Owner
                },
                &OrderState::genesis(),
                &plain_ctx()
            ),
            Ok(())
        );
        // Owner keeps IN_DELIVERY→CANCELLED (no-show).
        let in_delivery = OrderState {
            status: OrderStatus::InDelivery,
            ..OrderState::genesis()
        };
        assert_eq!(
            validate(
                &Command::Cancel {
                    at: T,
                    actor: Actor::Owner
                },
                &in_delivery,
                &plain_ctx()
            ),
            Ok(())
        );
    }

    // ─────────────── RED case #4: CC-1 strand guard over observed binding ───────────────

    fn ctx_with_binding(binding: BindingState) -> Context<'static> {
        Context {
            binding,
            refundable_paid: Lek::ZERO,
            pricing: None,
        }
    }

    /// A →DELIVERED with an ACTIVE courier binding is refused — complete via the deliver flow —
    /// `CourierStrandGuard { "ACTIVE_BINDING" }`. IN_DELIVERY→DELIVERED is the machine-legal edge; the
    /// active binding is OBSERVED on the context (the first invariant that reads `ctx.binding`). RED
    /// proof: a `validate` that skipped cc1 would return `Ok(())` here.
    #[test]
    fn active_binding_blocks_mark_delivered() {
        let in_delivery = OrderState {
            status: OrderStatus::InDelivery,
            ..OrderState::genesis()
        };
        let ctx = ctx_with_binding(BindingState {
            has_active_binding: true,
            has_delivered_binding: false,
        });
        assert_eq!(
            validate(
                &Command::MarkDelivered {
                    at: T,
                    actor: Actor::System
                },
                &in_delivery,
                &ctx
            ),
            Err(vec![Invariant::CourierStrandGuard {
                reason: "ACTIVE_BINDING",
            }])
        );
    }

    /// IN_DELIVERY→DELIVERED with NO delivered binding must route through the deliver flow —
    /// `CourierStrandGuard { "REQUIRES_DELIVER_FLOW" }` — so an order is never marked done out from
    /// under a live dispatch.
    #[test]
    fn in_delivery_without_delivered_binding_requires_deliver_flow() {
        let in_delivery = OrderState {
            status: OrderStatus::InDelivery,
            ..OrderState::genesis()
        };
        assert_eq!(
            validate(
                &Command::MarkDelivered {
                    at: T,
                    actor: Actor::System
                },
                &in_delivery,
                &plain_ctx()
            ),
            Err(vec![Invariant::CourierStrandGuard {
                reason: "REQUIRES_DELIVER_FLOW",
            }])
        );
    }

    /// cc1 PERMITS the completeDelivery path (a `delivered` assignment exists → IN_DELIVERY→DELIVERED)
    /// and a never-dispatched manual pickup (READY→PICKED_UP, no binding) — the gate is precise, not a
    /// blanket done-block.
    #[test]
    fn cc1_permits_completed_delivery_and_manual_pickup() {
        let in_delivery = OrderState {
            status: OrderStatus::InDelivery,
            ..OrderState::genesis()
        };
        let delivered = ctx_with_binding(BindingState {
            has_active_binding: false,
            has_delivered_binding: true,
        });
        assert_eq!(
            validate(
                &Command::MarkDelivered {
                    at: T,
                    actor: Actor::System
                },
                &in_delivery,
                &delivered
            ),
            Ok(())
        );
        let ready = OrderState {
            status: OrderStatus::Ready,
            ..OrderState::genesis()
        };
        assert_eq!(
            validate(
                &Command::MarkPickedUp {
                    at: T,
                    actor: Actor::System
                },
                &ready,
                &plain_ctx()
            ),
            Ok(())
        );
    }
}
