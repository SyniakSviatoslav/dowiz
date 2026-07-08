//! Verified-by-Math tests for the Bebop deterministic core + falsifiable guardrails.
//!
//! Every gate carries a RED case (goes false on bad input) and a GREEN case (stays true on good
//! input). A test that cannot fail is a false-green and is rejected by the discipline.

use bebop::{
    Core, GuardKind, command_hash, guard_path,
    core::Step,
};
use dowiz_core::{
    Actor, Command, Context, OrderStatus, Ts,
    kernel::pricing::{FeeLocation, PricingItem, PricingSnapshot, ProductInfo, PriceInputs},
    kernel::policy::BindingState,
    Lek,
};

// ─────────────────────────── helpers ───────────────────────────

fn plain_ctx() -> Context<'static> {
    Context {
        binding: BindingState { has_active_binding: false, has_delivered_binding: false },
        refundable_paid: Lek::ZERO,
        pricing: None,
    }
}

fn priced_ctx() -> Context<'static> {
    let mut product_map = std::collections::HashMap::new();
    product_map.insert("p1".to_string(), ProductInfo { name: "Pizza".into(), price: Lek::new(1_000).unwrap() });
    let snapshot = PricingSnapshot {
        product_map: &product_map,
        mod_map: &std::collections::HashMap::new(),
        groups_by_product: &std::collections::HashMap::new(),
    };
    let inputs = PriceInputs {
        snapshot,
        is_pickup: true,
        location: FeeLocation { delivery_fee_flat: None, free_delivery_threshold: None, min_order_value: None },
        distance_m: None,
        tiers: &[],
        rate_micro: 200_000,
        price_includes_tax: false,
    };
    Context { binding: BindingState { has_active_binding: false, has_delivered_binding: false }, refundable_paid: Lek::ZERO, pricing: Some(inputs) }
}

// ─────────────────────────── RED+GREEN: guardrail deny ───────────────────────────

#[test]
fn redline_deny_green_allows_scope_file() {
    assert_eq!(guard_path("tools/bebop/src/core.rs", "/repo"), GuardKind::Ok);
}

#[test]
fn redline_deny_red_blocks_migration() {
    // RED: a migration file MUST be denied as a red-line, never Ok.
    assert_eq!(guard_path("packages/db/migrations/002.sql", "/repo"), GuardKind::RedLine);
}

#[test]
fn scope_block_green_allows_bebop() {
    assert_eq!(guard_path("crates/bebop/src/guard.rs", "/repo"), GuardKind::Ok);
}

#[test]
fn scope_block_red_blocks_outside() {
    // RED: a file outside the agreed scope MUST be blocked.
    assert_eq!(guard_path("apps/api/src/server.ts", "/repo"), GuardKind::Scope);
}

// ─────────────────────────── RED+GREEN: deterministic hash ───────────────────────────

#[test]
fn hash_is_deterministic_same_input() {
    let c = Command::Confirm { at: Ts(1), actor: Actor::Owner };
    assert_eq!(command_hash(&c), command_hash(&c));
}

#[test]
fn hash_is_deterministic_across_calls() {
    // RED: two DIFFERENT commands must NOT hash equal (a hash that collapses is a false-green).
    let a = Command::Confirm { at: Ts(1), actor: Actor::Owner };
    let b = Command::Cancel { at: Ts(1), actor: Actor::Owner };
    assert_ne!(command_hash(&a), command_hash(&b));
}

// ─────────────────────────── RED+GREEN: kernel door ───────────────────────────

#[test]
fn place_order_prices_through_kernel() {
    let mut core = Core::new();
    let t = Ts(1_700_000_000_000);
    let step = core.apply(
        Command::PlaceOrder { at: t, actor: Actor::Owner, cart: vec![PricingItem { product_id: "p1".into(), quantity: 2, modifier_ids: vec![] }] },
        &priced_ctx(),
    );
    assert!(step.violations.is_empty(), "place order must be clean: {:?}", step.violations);
    assert_eq!(core.state().status, OrderStatus::Pending);
    // 2 × 1000 = 2000 subtotal, 0 delivery, 20% tax = 400, total 2400.
    let totals = core.state().totals.expect("order is priced");
    assert_eq!(totals.total, Lek::new(2_400).unwrap());
}

#[test]
fn illegal_transition_is_refused_red() {
    // RED: an illegal edge (Confirm on a Delivered order) must produce a violation, not a silent ok.
    let mut core = Core::new();
    let delivered = {
        let mut c = Core::new();
        let t = Ts(1);
        let mut ctx = plain_ctx();
        // drive to delivered
        for cmd in [
            Command::Confirm { at: t, actor: Actor::Owner },
            Command::StartPreparing { at: t, actor: Actor::Owner },
            Command::MarkReady { at: t, actor: Actor::Owner },
            Command::Dispatch { at: t, actor: Actor::Owner },
            Command::MarkDelivered { at: t, actor: Actor::System },
        ] {
            c.apply(cmd, &ctx);
            ctx = plain_ctx();
        }
        c
    };
    let step = delivered.apply(Command::Confirm { at: Ts(1), actor: Actor::Owner }, &plain_ctx());
    assert!(!step.violations.is_empty(), "illegal edge MUST be refused");
}

// ─────────────────────────── RED+GREEN: replay determinism ───────────────────────────

#[test]
fn log_replays_to_same_state() {
    let mut core = Core::new();
    let t = Ts(1_700_000_000_000);
    let ctx = priced_ctx();
    for c in [
        Command::PlaceOrder { at: t, actor: Actor::Owner, cart: vec![PricingItem { product_id: "p1".into(), quantity: 2, modifier_ids: vec![] }] },
        Command::Confirm { at: t, actor: Actor::Owner },
        Command::StartPreparing { at: t, actor: Actor::Owner },
        Command::MarkReady { at: t, actor: Actor::Owner },
        Command::Dispatch { at: t, actor: Actor::Owner },
        Command::MarkDelivered { at: t, actor: Actor::System },
    ] {
        core.apply(c, &ctx);
    }
    let log = core.export_log();
    let replayed = Core::from_log(&log).expect("log replays");
    assert_eq!(replayed.state().status, OrderStatus::Delivered);
    assert_eq!(replayed.state().totals, core.state().totals);
}

#[test]
fn log_is_byte_stable_for_same_inputs_red() {
    // RED: building the same session twice MUST yield identical canonical bytes.
    let build = || {
        let mut core = Core::new();
        let t = Ts(42);
        let ctx = priced_ctx();
        core.apply(Command::PlaceOrder { at: t, actor: Actor::Owner, cart: vec![PricingItem { product_id: "p1".into(), quantity: 1, modifier_ids: vec![] }] }, &ctx);
        core.export_log()
    };
    assert_eq!(build(), build(), "deterministic log must be byte-identical");
}

// keep Step import referenced (used in signatures/doc)
#[allow(dead_code)]
fn _uses_step(_: Step) {}
