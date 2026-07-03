import './_env-stub.js';
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { assertTransition, ORDER_STATUSES, isTerminal, type OrderStatus } from '@deliveryos/domain';

// ── EXHAUSTIVE assertTransition PIN (deliver-v2 offer-sweep-cancel addendum, F6 / counsel condition) ──
// The order state machine is a broad-authority surface. This test pins the EXACT legal edge set — every
// ordered (from,to) pair over ORDER_STATUSES is asserted to pass or throw the right error. Widening the
// machine (the three new SYSTEM-only →CANCELLED edges) OR any future drift (a new state, a removed edge)
// forces a conscious update here or fails RED. Duplicated here on purpose: TRANSITIONS is module-private,
// so this is an independent authority, not a re-import of the thing under test.

// The canonical expected legal edge set. THE THREE ADDED EDGES ARE MARKED.
const EXPECTED: Record<OrderStatus, OrderStatus[]> = {
  PENDING: ['CONFIRMED', 'REJECTED', 'CANCELLED'],
  CONFIRMED: ['PREPARING', 'IN_DELIVERY', 'CANCELLED'], // + CANCELLED (addendum, system-only)
  PREPARING: ['READY', 'CANCELLED'], //                    + CANCELLED (addendum, system-only)
  READY: ['IN_DELIVERY', 'PICKED_UP', 'CANCELLED'], //     + CANCELLED (addendum, system-only)
  IN_DELIVERY: ['DELIVERED', 'CANCELLED', 'READY'],
  DELIVERED: [],
  REJECTED: [],
  CANCELLED: [],
  SCHEDULED: [], // scaffold — both directions are ScaffoldDisabledError
  PICKED_UP: [],
};

const SCAFFOLD: ReadonlySet<OrderStatus> = new Set(['SCHEDULED']);

function classify(from: OrderStatus, to: OrderStatus): 'ok' | 'SameStatusError' | 'ScaffoldDisabledError' | 'IllegalTransitionError' {
  if (from === to) return 'SameStatusError';
  if (SCAFFOLD.has(to) || SCAFFOLD.has(from)) return 'ScaffoldDisabledError';
  return EXPECTED[from].includes(to) ? 'ok' : 'IllegalTransitionError';
}

test('assertTransition — exhaustive matrix over every ordered (from,to) pair', () => {
  let ok = 0;
  let thrown = 0;
  for (const from of ORDER_STATUSES) {
    for (const to of ORDER_STATUSES) {
      const expected = classify(from, to);
      if (expected === 'ok') {
        assert.doesNotThrow(() => assertTransition(from, to), `${from}→${to} must be legal`);
        ok++;
      } else {
        try {
          assertTransition(from, to);
          assert.fail(`${from}→${to} must throw ${expected} but passed`);
        } catch (e: any) {
          assert.equal(e.name, expected, `${from}→${to} must throw ${expected}, got ${e.name}`);
          thrown++;
        }
      }
    }
  }
  // 10 states × 10 = 100 ordered pairs; sanity-pin the split so an accidental empty matrix can't pass.
  assert.equal(ok + thrown, ORDER_STATUSES.length * ORDER_STATUSES.length);
  const legalCount = Object.values(EXPECTED).reduce((n, arr) => n + arr.length, 0);
  assert.equal(ok, legalCount, 'exactly the expected number of legal edges pass');
  assert.equal(legalCount, 14, 'legal edge total (11 pre-existing + the 3 addendum →CANCELLED edges)');
});

test('assertTransition — the three ADDED system-only →CANCELLED edges are now legal', () => {
  assert.doesNotThrow(() => assertTransition('CONFIRMED', 'CANCELLED'));
  assert.doesNotThrow(() => assertTransition('PREPARING', 'CANCELLED'));
  assert.doesNotThrow(() => assertTransition('READY', 'CANCELLED'));
});

test('assertTransition — pre-existing cancel edges still legal; terminal cancels still illegal', () => {
  assert.doesNotThrow(() => assertTransition('PENDING', 'CANCELLED'));
  assert.doesNotThrow(() => assertTransition('IN_DELIVERY', 'CANCELLED'));
  // Terminal states have no outbound CANCELLED edge (CANCELLED→CANCELLED is SameStatus, handled elsewhere).
  for (const from of ['DELIVERED', 'REJECTED', 'PICKED_UP'] as OrderStatus[]) {
    try {
      assertTransition(from, 'CANCELLED');
      assert.fail(`${from}→CANCELLED must stay illegal`);
    } catch (e: any) {
      assert.equal(e.name, 'IllegalTransitionError', `${from}→CANCELLED must throw IllegalTransitionError`);
    }
  }
});

test('CANCELLED is terminal (no outbound edges)', () => {
  assert.ok(isTerminal('CANCELLED'));
  assert.deepEqual(EXPECTED.CANCELLED, []);
});
