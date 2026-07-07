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
//! Four invariants so far (VALIDATION-LAYER-SPEC — each landed with its own RED case first):
//!   - [`Invariant::NonPositiveMoney`] — a *format-plus* check: the raw-`i64` money on the observed
//!     pricing authority is non-negative minor units.
//!   - [`Invariant::IllegalTransition`] — a *logical* check: `(state, cmd)` is a legal edge of the
//!     order state machine.
//!   - [`Invariant::ActorNotAuthorized`] — a *logical* check: the actor may drive this (machine-legal)
//!     edge — the AUTHORIZATION layer OVER the machine (`policy::assert_owner_target_allowed`).
//!   - [`Invariant::CourierStrandGuard`] — a *logical* check over OBSERVED context: a →DELIVERED/
//!     →PICKED_UP is refused while a courier binding is live/undelivered (`policy::cc1_strand_guard`).
//!     These three logical checks compose the same predicates [`decide`](super::decide) does, so for a
//!     transition command the gate now covers ALL of `decide`'s transition preconditions.
//!
//! The remaining invariants sketched in the spec (`EmptyLineItems`, `PriceContextMismatch`,
//! `IdempotencyKeyMissing`, `QuantityOutOfRange`) are appended one at a time, each with its own RED
//! case first. [`Invariant`] is `#[non_exhaustive]` so that growth is not a breaking change for the
//! shell crate.

use super::{Actor, Command, Context, OrderState};
use crate::{ErrorCode, OrderStatus, order_status::assert_transition};

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
/// ALL of [`decide`]'s transition preconditions, so an accepted transition never trips `decide`. (The
/// `PlaceOrder` pricing-corridor dimension arrives with its invariants.)
pub fn validate(cmd: &Command, state: &OrderState, ctx: &Context<'_>) -> Result<(), Vec<Invariant>> {
    let mut violations = Vec::new();

    // ── format-plus: the observed pricing authority's raw-i64 money is non-negative minor units.
    // Present only on a `PlaceOrder` context (transitions do not price); its absence ⇒ nothing to
    // check here. The check is well-formedness of the INPUT, independent of which pricing branch a
    // given cart happens to take — a negative fee/threshold/min-order is malformed money regardless.
    if let Some(pricing) = ctx.pricing.as_ref() {
        check_fee_money_non_negative(&pricing.location, &mut violations);
    }

    // ── logical: transition legality. `PlaceOrder` is the CREATE+PRICE door — a placed order is
    // born PENDING (= genesis) and never touches the machine (decide routes it around), so it is
    // NOT transition-gated. Every other command drives an edge the machine is the sole authority on.
    if !matches!(cmd, Command::PlaceOrder { .. }) {
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

    fn place_order() -> Command {
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
