import { test } from 'node:test';
import assert from 'node:assert/strict';
import * as fc from 'fast-check';
import { assertTransition, isTerminal, ORDER_STATUSES, type OrderStatus } from '../order-machine.js';
import { IllegalTransitionError, ScaffoldDisabledError, SameStatusError } from '../errors.js';

const TERMINAL: ReadonlySet<OrderStatus> = new Set(['DELIVERED', 'REJECTED', 'CANCELLED']);
const SCAFFOLD: ReadonlySet<OrderStatus> = new Set(['SCHEDULED', 'PICKED_UP']);

const VALID_TRANSITIONS: ReadonlyMap<OrderStatus, ReadonlyArray<OrderStatus>> = new Map([
  ['PENDING',     ['CONFIRMED', 'REJECTED', 'CANCELLED']],
  ['CONFIRMED',   ['PREPARING', 'IN_DELIVERY']],
  ['PREPARING',   ['READY']],
  ['READY',       ['IN_DELIVERY']],
  ['IN_DELIVERY', ['DELIVERED']],
  ['DELIVERED',   []],
  ['REJECTED',    []],
  ['CANCELLED',   []],
  ['SCHEDULED',   []],
  ['PICKED_UP',   []],
]);

const statusArb = fc.constantFrom(...ORDER_STATUSES);

test('P1: every listed valid transition is accepted', () => {
  for (const [from, tos] of VALID_TRANSITIONS) {
    if (SCAFFOLD.has(from)) continue;
    for (const to of tos) {
      if (SCAFFOLD.has(to)) continue;
      assert.doesNotThrow(
        () => assertTransition(from, to),
        `expected ${from} → ${to} to be valid`,
      );
    }
  }
});

test('P2: same-status always throws SameStatusError', () => {
  fc.assert(
    fc.property(statusArb, (s) => {
      let threw = false;
      try { assertTransition(s, s); } catch (e: any) { threw = true; assert.equal(e.name, 'SameStatusError'); }
      assert.ok(threw, `expected SameStatusError for ${s} → ${s}`);
    }),
  );
});

test('P3: terminal statuses have no outgoing transitions', () => {
  fc.assert(
    fc.property(
      fc.constantFrom(...[...TERMINAL]),
      statusArb,
      (from, to) => {
        if (from === to) return; // covered by P2
        let threw = false;
        try { assertTransition(from as OrderStatus, to); } catch { threw = true; }
        assert.ok(threw, `terminal ${from} should not transition to ${to}`);
      },
    ),
  );
});

test('P4: scaffold statuses always throw ScaffoldDisabledError in both directions', () => {
  fc.assert(
    fc.property(
      fc.constantFrom(...[...SCAFFOLD]),
      statusArb,
      (scaffoldStatus, other) => {
        if (scaffoldStatus === other) return; // SameStatusError, not ScaffoldDisabledError

        // scaffold as source
        let threw = false;
        try { assertTransition(scaffoldStatus as OrderStatus, other); } catch (e: any) {
          threw = true; assert.equal(e.name, 'ScaffoldDisabledError');
        }
        assert.ok(threw, `scaffold source ${scaffoldStatus} → ${other} should throw ScaffoldDisabledError`);

        // scaffold as destination
        threw = false;
        try { assertTransition(other, scaffoldStatus as OrderStatus); } catch (e: any) {
          threw = true; assert.equal(e.name, 'ScaffoldDisabledError');
        }
        assert.ok(threw, `scaffold destination ${other} → ${scaffoldStatus} should throw ScaffoldDisabledError`);
      },
    ),
  );
});

test('P5: no path ever leads back from a terminal status (irreversibility)', () => {
  fc.assert(
    fc.property(
      fc.constantFrom(...[...TERMINAL]),
      fc.array(statusArb, { minLength: 1, maxLength: 10 }),
      (terminal, path) => {
        let current: OrderStatus = terminal as OrderStatus;
        for (const next of path) {
          if (current === next) continue;
          let threw = false;
          try { assertTransition(current, next); } catch { threw = true; }
          if (!threw) {
            // If somehow a transition succeeded from terminal, that's the bug
            assert.fail(`Illegal transition from terminal ${current} to ${next} was accepted`);
          }
          // Once it threw, current stays terminal — chain is permanently dead
          break;
        }
      },
    ),
  );
});

test('P6a: isTerminal returns true exactly for DELIVERED, REJECTED, CANCELLED', () => {
  const shouldBeTerminal: OrderStatus[] = ['DELIVERED', 'REJECTED', 'CANCELLED'];
  for (const s of shouldBeTerminal) {
    assert.ok(isTerminal(s), `expected isTerminal(${s}) === true`);
  }
  const notTerminal = ORDER_STATUSES.filter(s => !shouldBeTerminal.includes(s));
  for (const s of notTerminal) {
    assert.ok(!isTerminal(s), `expected isTerminal(${s}) === false`);
  }
});

test('P6b: isTerminal is consistent with assertTransition (terminal ↔ no outgoing)', () => {
  fc.assert(
    fc.property(statusArb, statusArb, (from, to) => {
      if (!isTerminal(from)) return; // only test terminal sources
      if (from === to) return;       // SameStatusError, different invariant
      let threw = false;
      try { assertTransition(from, to); } catch { threw = true; }
      assert.ok(threw, `isTerminal(${from})=true but assertTransition(${from}, ${to}) did not throw`);
    }),
  );
});

test('P6c: error objects carry correct name and mention from/to in message', () => {
  fc.assert(
    fc.property(statusArb, statusArb, (from, to) => {
      if (from === to) {
        let e: any;
        try { assertTransition(from, to); } catch (err) { e = err; }
        assert.ok(e instanceof SameStatusError);
        assert.equal(e.name, 'SameStatusError');
        assert.ok(e.message.includes(from), `SameStatusError message should mention '${from}'`);
        return;
      }
      if ([...SCAFFOLD].some(s => s === from || s === to)) {
        let e: any;
        try { assertTransition(from, to); } catch (err) { e = err; }
        if (!e) return; // some scaffold combos are same-status handled above
        assert.ok(e instanceof ScaffoldDisabledError);
        assert.equal(e.name, 'ScaffoldDisabledError');
        return;
      }
      // For illegal transitions, check the error type
      let e: any;
      try { assertTransition(from, to); } catch (err) { e = err; }
      if (!e) return; // valid transition, no error expected
      assert.ok(e instanceof IllegalTransitionError);
      assert.equal(e.name, 'IllegalTransitionError');
      assert.ok(e.message.includes(from), `IllegalTransitionError message should mention '${from}'`);
      assert.ok(e.message.includes(to), `IllegalTransitionError message should mention '${to}'`);
    }),
  );
});

test('P6: assertTransition is deterministic (same inputs always yield same result)', () => {
  fc.assert(
    fc.property(statusArb, statusArb, (from, to) => {
      let result1: 'ok' | string = 'ok';
      let result2: 'ok' | string = 'ok';
      try { assertTransition(from, to); } catch (e: any) { result1 = e.name; }
      try { assertTransition(from, to); } catch (e: any) { result2 = e.name; }
      assert.equal(result1, result2, `assertTransition(${from}, ${to}) was not deterministic`);
    }),
  );
});
