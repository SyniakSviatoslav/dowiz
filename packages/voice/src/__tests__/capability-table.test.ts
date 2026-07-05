import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import type { IntentKind, Capability } from '../types.js';
import { classify } from '../capability-table.js';
import type { VoiceHandlers } from '../confirmation-gate.js';

// Guardrails G3 (explicit `never`-exhaustiveness) + G4 (capability-table table test) — ADR-0015 §6/§10,
// proposal §6 / PHASE1-IMPLEMENTATION-PLAN.md §5. capability-table.ts's `Record<IntentKind, …>` already
// fails the BUILD if a new IntentKind has no row (the STRUCTURAL ratchet). This file makes that
// invariant EXPLICIT — a dedicated, by-name `never` assertion — and adds the money/dietary/settling
// EXCLUSION table test the ADR requires (fail-closed default; no voice-reachable place-order/pay/
// checkout/payment/finalization/settling kind; setFilterAllergen not a VoiceHandlers key).
//
// IMPORTANT — two enforcement layers, both must stay green:
//  1. TYPE-LEVEL (the `never`/`IsDisjoint` checks below) — enforced by `tsc`, i.e.
//     `pnpm --filter @deliveryos/voice typecheck`. `tsx --test` strips types WITHOUT checking them, so
//     these do not fail at `test` time — exactly like the pre-existing structural Record<IntentKind,…>
//     ratchet. Breaking one is a compile error, not a thrown assertion.
//  2. RUNTIME (the `classify()` calls below) — enforced by `node --test` / `tsx --test`, independent of
//     the type layer, so a `classify()` fail-open regression is caught even without a typecheck step.

// Type-level helper: true iff A and B share no member. Tuple-wrapped ([X] extends [never]) so a UNION
// passed as A does not distribute the conditional over its members (the naive `A extends never` form
// would silently no-op on a union input).
type IsDisjoint<A extends string, B extends string> = [Extract<A, B>] extends [never] ? true : false;

function assertNeverKind(k: never): never {
  throw new Error(`IntentKind not exhaustively handled by this table test: ${String(k)}`);
}

/**
 * G3 — mirrors CAPABILITY_TABLE via an EXHAUSTIVE switch. Adding a member to `IntentKind` without
 * extending this switch fails `tsc`: the new member falls to `default`, `kind` there is not narrowed to
 * `never`, and `assertNeverKind(kind)` no longer type-checks (TS2345). This IS the explicit
 * never-exhaustiveness assertion (distinct from — and in addition to — the structural Record ratchet).
 */
function expectedCapability(kind: IntentKind): Capability {
  switch (kind) {
    case 'ADD_TO_CART':
      return 'STATEFUL';
    case 'SET_SORT':
    case 'SET_MACRO_LENS':
    case 'SELECT_CATEGORY':
    case 'SET_SEARCH':
    case 'TOGGLE_COMPARE':
    case 'READ_ORDER':
    case 'NAVIGATE_CHECKOUT':
      return 'READ_ONLY';
    default:
      return assertNeverKind(kind);
  }
}

// The full active-scope IntentKind union, mirrored here (same convention as matcher.test.ts's coverage
// assertion at src/__tests__/matcher.test.ts:100-114) so the runtime loop below independently
// cross-checks the real `classify()` against this table — a drift between capability-table.ts and this
// list fails at `it()`, not silently.
const ALL_KINDS: readonly IntentKind[] = [
  'ADD_TO_CART',
  'SET_SORT',
  'SET_MACRO_LENS',
  'SELECT_CATEGORY',
  'SET_SEARCH',
  'TOGGLE_COMPARE',
  'READ_ORDER',
  'NAVIGATE_CHECKOUT',
];

describe('voice capability-table — exhaustiveness (G3) + table test (G4)', () => {
  it('G3: every IntentKind classifies via the exhaustive switch and matches the real classify()', () => {
    for (const kind of ALL_KINDS) {
      assert.equal(classify(kind), expectedCapability(kind), `${kind} capability diverged vs. the exhaustive table`);
    }
  });

  it('G4: ADD_TO_CART is the ONE STATEFUL kind in active scope; all others are READ_ONLY', () => {
    assert.equal(classify('ADD_TO_CART'), 'STATEFUL');
    for (const kind of ALL_KINDS.filter((k) => k !== 'ADD_TO_CART')) {
      assert.equal(classify(kind), 'READ_ONLY', `${kind} must be READ_ONLY`);
    }
  });

  it('G4: money/checkout/place-order/payment/order-finalization strings fail-closed to REJECT (no such kind exists)', () => {
    const forbidden = [
      'PLACE_ORDER', 'place_order', 'PlaceOrder',
      'PAY', 'pay',
      'CHECKOUT', 'checkout', 'CHECKOUT_SUBMIT', 'checkout_submit',
      'PAYMENT', 'payment',
      'ORDER_FINALIZATION', 'order-finalization', 'FINALIZE_ORDER', 'finalizeOrder',
    ];
    for (const kind of forbidden) {
      assert.equal(classify(kind), 'REJECT', `${kind} must fail-closed to REJECT — no money/checkout-write kind may exist`);
    }
  });

  it('G4 (compile-time): IntentKind never collides with a money/checkout literal — tsc-enforced', () => {
    type ForbiddenMoneyKinds =
      | 'PLACE_ORDER' | 'place_order'
      | 'PAY' | 'pay'
      | 'CHECKOUT' | 'checkout' | 'CHECKOUT_SUBMIT'
      | 'PAYMENT' | 'payment'
      | 'ORDER_FINALIZATION' | 'order-finalization' | 'FINALIZE_ORDER';
    type Disjoint = IsDisjoint<IntentKind, ForbiddenMoneyKinds>;
    // If a future IntentKind literal collides with the forbidden set above, `Disjoint` becomes `false`
    // and this assignment fails `tsc` (RED). Currently `true` (GREEN).
    const proof: Disjoint = true;
    assert.equal(proof, true);
  });

  it('G4: no arrived/completeDelivery/settling kind exists — courier/admin voice is out of scope (H4)', () => {
    const forbidden = [
      'ARRIVED', 'arrived', 'COURIER_ARRIVED',
      'COMPLETE_DELIVERY', 'completeDelivery',
      'SETTLE', 'settle', 'SETTLE_DELIVERY', 'MARK_DELIVERED', 'DELIVERED',
    ];
    for (const kind of forbidden) {
      assert.equal(classify(kind), 'REJECT', `${kind} must fail-closed to REJECT — no settling kind is voice-reachable`);
    }
  });

  it('G4 (compile-time): IntentKind never collides with an arrived/completeDelivery/settling literal — tsc-enforced', () => {
    type ForbiddenSettlingKinds =
      | 'ARRIVED' | 'arrived' | 'COURIER_ARRIVED'
      | 'COMPLETE_DELIVERY' | 'completeDelivery'
      | 'SETTLE' | 'settle' | 'SETTLE_DELIVERY' | 'MARK_DELIVERED' | 'DELIVERED';
    type Disjoint = IsDisjoint<IntentKind, ForbiddenSettlingKinds>;
    const proof: Disjoint = true;
    assert.equal(proof, true);
  });

  it('G4: setFilterAllergen is NOT a VoiceHandlers key — the allergen filter is not voice-reachable (C1)', () => {
    // A fully-typed handlers literal: if VoiceHandlers ever gains a new REQUIRED key, this object
    // literal fails `tsc` (missing property) until updated — a live drift signal independent of the
    // Exclude-based check below.
    const handlers: VoiceHandlers = {
      addToCart: () => true,
      setSort: () => {},
      setMacroLens: () => {},
      selectCategory: () => {},
      setSearch: () => {},
      toggleCompare: () => {},
      readOrder: () => {},
      navigateCheckout: () => {},
    };
    const EXPECTED_KEYS = [
      'addToCart', 'setSort', 'setMacroLens', 'selectCategory',
      'setSearch', 'toggleCompare', 'readOrder', 'navigateCheckout',
    ];
    assert.deepEqual(Object.keys(handlers).sort(), [...EXPECTED_KEYS].sort());
    assert.ok(!('setFilterAllergen' in handlers), 'setFilterAllergen must not be a VoiceHandlers key');
  });

  it('G4 (compile-time): VoiceHandlers never gains a setFilterAllergen key — tsc-enforced', () => {
    // Compile-time ratchet: if VoiceHandlers ever adds 'setFilterAllergen' (required OR optional),
    // `Disjoint` becomes `false` and this assignment fails `tsc` (RED) — independent of what any
    // hand-built object above happens to contain.
    type Disjoint = IsDisjoint<'setFilterAllergen', keyof VoiceHandlers>;
    const proof: Disjoint = true;
    assert.equal(proof, true);
  });
});
