//! Phase 2.2 Adversarial Test Suite — Money Invariants (RED Proofs)
//!
//! The three critical money invariants for the direct checkout flow
//! (`docs/design/sovereign-core-mvp/PHASE-2-2-CART-TOKEN-SPEC.md`):
//!   1. the client cannot inject a price — the SERVER prices the cart;
//!   2. a duplicate request hash yields exactly ONE order (idempotent retry);
//!   3. the conservation invariant `total = subtotal + tax_charged + delivery_fee − discount` holds
//!      for every order (with `tax_charged ≤ tax_total`, LC1, and every term ≥ 0).
//!
//! These are FALSIFIABLE proofs (Verified-by-Math): each ships with a stated RED case that turns it
//! red, and each exercises the REAL sovereign core — `domain::decide` / `Command::PlaceOrder` (the
//! kernel checkout door), `domain::kernel::idempotency` (the dedup decision), and the integer money
//! composition — NOT test-local re-implementations.
//!
//! `crates/api` is a binary crate with no library target, so an integration test cannot reach the
//! `api` HTTP handler directly; the HTTP-boundary enforcement of guard #1 (a `x-dowiz-cutover`
//! request with a client price field → `400 VALIDATION_FAILED`) is proven in the crate's own
//! `routes::orders::checkout` unit tests + `routes::orders::handler_tests`. THIS suite pins the
//! deeper money invariants those guards exist to protect, at the core that owns them. The live-DB
//! COUNT/SUM gate (real `orders` rows on staging) runs via the `#[ignore]` Postgres probes in
//! `routes/orders/pg.rs` and `scripts/replay-parity-check`.

use std::collections::HashMap;

use domain::kernel::idempotency::{ExistingKey, IdempotencyDecision, idempotency_decision};
use domain::{
    Actor, BindingState, Command, Context, DeliveryTier, Event, FeeLocation, GroupInfo, Lek,
    ModifierInfo, OrderState, PriceInputs, PricingItem, PricingSnapshot, ProductInfo, Ts, decide,
};
use uuid::Uuid;

/// A fixed caller-supplied event time — the core reads no clock (Law 1), so this is arbitrary.
const AT: Ts = Ts(1_700_000_000_000);

/// Price a cart through the REAL sovereign-core checkout door: `decide(genesis, PlaceOrder, ctx)` →
/// the `Event::Priced` money snapshot. The price authority (product snapshot, fee config, tax rate)
/// is OBSERVED context; the cart is the only intent — there is no channel for a client price.
#[allow(clippy::too_many_arguments)]
fn price_cart_via_kernel(
    products: &HashMap<String, ProductInfo>,
    mods: &HashMap<String, ModifierInfo>,
    groups: &HashMap<String, Vec<GroupInfo>>,
    cart: Vec<PricingItem>,
    is_pickup: bool,
    location: FeeLocation,
    distance_m: Option<i64>,
    tiers: &[DeliveryTier],
    rate_micro: i64,
    price_includes_tax: bool,
) -> Event {
    let inputs = PriceInputs {
        snapshot: PricingSnapshot {
            product_map: products,
            mod_map: mods,
            groups_by_product: groups,
        },
        is_pickup,
        location,
        distance_m,
        tiers,
        rate_micro,
        price_includes_tax,
    };
    let ctx = Context {
        binding: BindingState {
            has_active_binding: false,
            has_delivered_binding: false,
        },
        refundable_paid: Lek::ZERO,
        pricing: Some(inputs),
    };
    let events = decide(
        &OrderState::genesis(),
        Command::PlaceOrder {
            at: AT,
            actor: Actor::Owner,
            cart,
        },
        &ctx,
    )
    .expect("PlaceOrder against a valid snapshot must price the cart");
    events
        .into_iter()
        .find(|e| matches!(e, Event::Priced { .. }))
        .expect("PlaceOrder emits a Priced event")
}

fn single_product(price: i64) -> HashMap<String, ProductInfo> {
    let mut m = HashMap::new();
    m.insert(
        "p1".to_string(),
        ProductInfo {
            name: "item".to_string(),
            price: Lek::new(price).unwrap(),
        },
    );
    m
}

fn one_item_cart(qty: i64) -> Vec<PricingItem> {
    vec![PricingItem {
        product_id: "p1".to_string(),
        quantity: qty,
        modifier_ids: vec![],
    }]
}

fn no_fee_location() -> FeeLocation {
    FeeLocation {
        delivery_fee_flat: None,
        free_delivery_threshold: None,
        min_order_value: None,
    }
}

fn flat_fee_location(flat: i64) -> FeeLocation {
    FeeLocation {
        delivery_fee_flat: Some(flat),
        free_delivery_threshold: None,
        min_order_value: None,
    }
}

/// RED PROOF 1 — a client-injected price is refused; the SERVER prices the cart.
///
/// The Phase 2.2 spec forbids client price fields (`subtotal`/`tax_total`/`delivery_fee`/`total`/
/// `discount_total`) — a `x-dowiz-cutover` request carrying one is `400 VALIDATION_FAILED` (proven at
/// the HTTP boundary in `routes::orders::checkout` / `handler_tests`). THIS proof pins the invariant
/// that guard protects at the core: the create door `Command::PlaceOrder` has NO channel for a client
/// price — it prices the cart from the DB snapshot alone. A malicious client "wants" to pay 1; the
/// true cart is 2 × 1000 = 2000.
///
/// RED: a regression that trusted a client-supplied price would make `total` track the injected
/// value → the final `assert_ne!` (and the exact-total `assert_eq!`) fails.
#[test]
fn red_proof_1_client_injected_price_is_refused_server_prices_the_cart() {
    let products = single_product(1000);
    let mods = HashMap::new();
    let groups = HashMap::new();
    const MALICIOUS_CLIENT_TOTAL: i64 = 1;

    let priced = price_cart_via_kernel(
        &products,
        &mods,
        &groups,
        one_item_cart(2),
        true, // pickup
        no_fee_location(),
        None,
        &[],
        200_000, // 20% exclusive
        false,
    );
    let Event::Priced {
        subtotal,
        tax_total,
        delivery_fee,
        total,
    } = priced
    else {
        panic!("expected a Priced event");
    };

    // Server price authority: subtotal = 1000·2 = 2000; tax = 2000·0.2 = 400 (exclusive); pickup fee 0.
    assert_eq!(
        subtotal.minor_units(),
        2000,
        "subtotal is the cart × menu price, never the client's"
    );
    assert_eq!(tax_total.minor_units(), 400);
    assert_eq!(delivery_fee.minor_units(), 0);
    assert_eq!(total.minor_units(), 2400);
    // The charge is the SERVER total, categorically NOT the injected client value.
    assert_ne!(
        total.minor_units(),
        MALICIOUS_CLIENT_TOTAL,
        "the server must ignore any client-injected price"
    );
}

/// RED PROOF 2 — a duplicate request hash yields exactly ONE order.
///
/// A network retry re-submits the SAME cart, which re-hashes to the SAME request_hash. The
/// `(location_id, request_hash)` UNIQUE key + the kernel idempotency decision guarantee the second
/// submit REPLAYS the committed order rather than inserting a duplicate. The falsifiable core is the
/// REAL `domain::kernel::idempotency::idempotency_decision`; the UNIQUE(location_id, request_hash)
/// table is modelled in memory to COUNT the resulting rows.
///
/// RED: a regression that returned `Proceed` for a matching hash+present order (i.e. removed the
/// dedup) inserts a second row → `count == 2` (and the UNIQUE-key `assert!` fires first).
#[test]
fn red_proof_2_duplicate_request_hash_yields_exactly_one_order() {
    let location = Uuid::new_v4();
    let request_hash = "the-stable-canonical-hash-of-this-cart";

    // The kernel decision — the actual dedup logic (orders.ts §5, REV-S5-5):
    assert_eq!(
        idempotency_decision(None, request_hash),
        IdempotencyDecision::Proceed,
        "first submit (no key row) → create"
    );
    let key = ExistingKey {
        request_hash: request_hash.to_string(),
        order_present: true,
    };
    assert_eq!(
        idempotency_decision(Some(&key), request_hash),
        IdempotencyDecision::Replay,
        "retry (key hit, hash matches, order present) → replay, NOT a new insert"
    );
    assert_eq!(
        idempotency_decision(Some(&key), "a-different-cart-hash"),
        IdempotencyDecision::Reuse422,
        "a MUTATED cart reusing the key → 422, never a silent duplicate"
    );

    // Enact the decisions against the UNIQUE(location_id, request_hash) store, over 3 identical retries.
    let mut orders: Vec<(Uuid, String)> = Vec::new();
    let mut existing: Option<ExistingKey> = None;
    for _retry in 0..3 {
        match idempotency_decision(existing.as_ref(), request_hash) {
            IdempotencyDecision::Proceed | IdempotencyDecision::DeleteAndRecreate => {
                // UNIQUE constraint: an insert with a duplicate (location, hash) must be impossible.
                let already = orders.iter().any(|(l, h)| *l == location && h == request_hash);
                assert!(
                    !already,
                    "UNIQUE(location_id, request_hash) must block a 2nd insert"
                );
                orders.push((location, request_hash.to_string()));
                existing = Some(ExistingKey {
                    request_hash: request_hash.to_string(),
                    order_present: true,
                });
            }
            IdempotencyDecision::Replay => { /* idempotent retry — no new row */ }
            IdempotencyDecision::Reuse422 => panic!("the same cart must never 422"),
        }
    }

    let count = orders
        .iter()
        .filter(|(l, h)| *l == location && h == request_hash)
        .count();
    assert_eq!(
        count, 1,
        "a duplicate request hash must yield exactly ONE order"
    );
}

/// RED PROOF 3 — the conservation invariant holds over a matrix of orders.
///
/// spec §Conservation: for EVERY order `total = subtotal + tax_charged + delivery_fee − discount`,
/// with `tax_charged ≤ tax_total` (LC1) and every term ≥ 0. Price a matrix of carts through the REAL
/// kernel and INDEPENDENTLY recompute the composition in raw `i64` — never calling `compose_total`.
///
/// RED: a composition bug (e.g. adding `tax_total` instead of the LC1 `tax_charged` on an inclusive
/// venue — a double charge) diverges the kernel's `total` from the recomputation → `assert_eq!` fails.
#[test]
fn red_proof_3_conservation_invariant_holds_over_all_test_orders() {
    const DISCOUNT: i64 = 0; // REV-S5-6 CARRY (no redemption runtime; always 0)
    let mut orders_checked = 0usize;
    for &unit_price in &[500i64, 1000, 1075, 1999, 12_345] {
        for &qty in &[1i64, 2, 3, 7] {
            for &rate_micro in &[0i64, 75_000, 100_000, 200_000] {
                for &price_includes_tax in &[false, true] {
                    for &(is_pickup, flat) in &[(true, None), (false, Some(350i64))] {
                        let products = single_product(unit_price);
                        let mods = HashMap::new();
                        let groups = HashMap::new();
                        let location = match flat {
                            Some(f) => flat_fee_location(f),
                            None => no_fee_location(),
                        };
                        let priced = price_cart_via_kernel(
                            &products,
                            &mods,
                            &groups,
                            one_item_cart(qty),
                            is_pickup,
                            location,
                            None,
                            &[],
                            rate_micro,
                            price_includes_tax,
                        );
                        let Event::Priced {
                            subtotal,
                            tax_total,
                            delivery_fee,
                            total,
                        } = priced
                        else {
                            panic!("expected a Priced event");
                        };

                        // Independent i64 recomputation (NOT via compose_total).
                        let sub = subtotal.minor_units();
                        let gross_tax = tax_total.minor_units();
                        let fee = delivery_fee.minor_units();
                        let charged_tax = if price_includes_tax { 0 } else { gross_tax }; // LC1
                        let expected_total = sub + charged_tax + fee - DISCOUNT;

                        assert_eq!(sub, unit_price * qty, "subtotal = cart × menu price");
                        assert_eq!(
                            total.minor_units(),
                            expected_total,
                            "CONSERVATION broke: unit={unit_price} qty={qty} rate={rate_micro} \
                             incl={price_includes_tax} pickup={is_pickup}"
                        );
                        // LC1: the charged tax never exceeds the gross tax (no double-taxation).
                        assert!(charged_tax <= gross_tax, "LC1: tax_charged ≤ tax_total");
                        // Every term ≥ 0.
                        assert!(
                            sub >= 0 && gross_tax >= 0 && fee >= 0 && total.minor_units() >= 0,
                            "all money terms ≥ 0"
                        );
                        orders_checked += 1;
                    }
                }
            }
        }
    }
    // 5 × 4 × 4 × 2 × 2 = 320 orders.
    assert!(
        orders_checked >= 100,
        "the matrix must exercise many orders, got {orders_checked}"
    );
}

/// RED PROOF 3 reinforcement — absolute (subtotal, tax_total, total) against hand-derived vectors.
///
/// Zero-import literals transcribed from the Node reference (`order-total-vectors.ts`), NEVER computed
/// from the implementation — an INDEPENDENT oracle that catches a tax-arithmetic bug, not merely a
/// composition-consistency bug. RED: break `apply_tax` or the LC1 branch and a literal diverges.
#[test]
fn red_proof_3b_absolute_totals_match_hand_derived_vectors() {
    struct V {
        unit: i64,
        qty: i64,
        rate_micro: i64,
        incl: bool,
        pickup: bool,
        flat: Option<i64>,
        subtotal: i64,
        tax_total: i64,
        total: i64,
    }
    let vectors = [
        // exclusive 20%, delivery flat 200: 1000 + tax 200 + fee 200 = 1400
        V { unit: 1000, qty: 1, rate_micro: 200_000, incl: false, pickup: false, flat: Some(200), subtotal: 1000, tax_total: 200, total: 1400 },
        // inclusive 20%, delivery flat 250: charged 0 (tax 200 extracted) → 1200 + 250 = 1450
        V { unit: 600, qty: 2, rate_micro: 200_000, incl: true, pickup: false, flat: Some(250), subtotal: 1200, tax_total: 200, total: 1450 },
        // inclusive 7.5% pickup: embedded tax 75, no fee → total = subtotal 1075
        V { unit: 1075, qty: 1, rate_micro: 75_000, incl: true, pickup: true, flat: None, subtotal: 1075, tax_total: 75, total: 1075 },
        // zero-rate pickup: no tax, no fee → total = subtotal
        V { unit: 1000, qty: 1, rate_micro: 0, incl: false, pickup: true, flat: None, subtotal: 1000, tax_total: 0, total: 1000 },
    ];
    for v in vectors {
        let products = single_product(v.unit);
        let mods = HashMap::new();
        let groups = HashMap::new();
        let location = match v.flat {
            Some(f) => flat_fee_location(f),
            None => no_fee_location(),
        };
        let priced = price_cart_via_kernel(
            &products,
            &mods,
            &groups,
            one_item_cart(v.qty),
            v.pickup,
            location,
            None,
            &[],
            v.rate_micro,
            v.incl,
        );
        let Event::Priced {
            subtotal,
            tax_total,
            total,
            ..
        } = priced
        else {
            panic!("expected a Priced event");
        };
        assert_eq!(subtotal.minor_units(), v.subtotal, "subtotal @ unit={} qty={}", v.unit, v.qty);
        assert_eq!(tax_total.minor_units(), v.tax_total, "tax_total @ unit={} rate={}", v.unit, v.rate_micro);
        assert_eq!(total.minor_units(), v.total, "total @ unit={} rate={} incl={}", v.unit, v.rate_micro, v.incl);
    }
}
