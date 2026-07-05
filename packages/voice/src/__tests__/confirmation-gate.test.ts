import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { ConfirmationGate } from '../confirmation-gate.js';
import type { VoiceHandlers } from '../confirmation-gate.js';
import { classify } from '../capability-table.js';
import type { IntentProposal } from '../types.js';

// Guardrail (council voice-control / ADR-0015 §6) — the confirm-then-execute invariant.
// RED if: a STATEFUL intent ever mutates without an explicit human confirm; classify() fails OPEN
// for an unknown/excluded kind; a money/checkout/place-order intent reaches a handler; or a
// dietary-named category auto-applies by voice. This must stay GREEN before the flag may ever flip.

function makeSpies(): { calls: Record<string, number>; handlers: VoiceHandlers } {
  const calls: Record<string, number> = {};
  const bump = (k: string) => () => {
    calls[k] = (calls[k] ?? 0) + 1;
  };
  const handlers: VoiceHandlers = {
    // addToCart reports apply-outcome (council R-a); the spy counts AND reports a real mutation.
    addToCart: () => {
      calls['addToCart'] = (calls['addToCart'] ?? 0) + 1;
      return true;
    },
    setSort: bump('setSort'),
    setMacroLens: bump('setMacroLens'),
    selectCategory: bump('selectCategory'),
    setSearch: bump('setSearch'),
    toggleCompare: bump('toggleCompare'),
    readOrder: bump('readOrder'),
    navigateCheckout: bump('navigateCheckout'),
  };
  return { calls, handlers };
}

function proposal(kind: string, args: Record<string, unknown> = {}): IntentProposal {
  return { kind, args, transcript: `mock: ${kind}`, confidence: 0.9 };
}

describe('voice ConfirmationGate — confirm-then-execute invariant', () => {
  it('READ_ONLY intent auto-applies on submit', () => {
    const { calls, handlers } = makeSpies();
    const gate = new ConfirmationGate(handlers);
    const r = gate.submit(proposal('SET_SORT', { by: 'price' }));
    assert.equal(r.status, 'applied');
    assert.equal(r.capability, 'READ_ONLY');
    assert.equal(calls.setSort, 1);
    assert.equal(gate.pending, null);
  });

  // THE core safety invariant: a STATEFUL intent NEVER mutates without an explicit human confirm.
  it('STATEFUL ADD_TO_CART does NOT apply on submit — only after confirm()', () => {
    const { calls, handlers } = makeSpies();
    const gate = new ConfirmationGate(handlers);
    const r = gate.submit(proposal('ADD_TO_CART', { productId: 'p1', qty: 2 }));
    assert.equal(r.status, 'pending-confirm');
    assert.equal(calls.addToCart, undefined, 'must NOT mutate on submit');
    assert.notEqual(gate.pending, null);

    const c = gate.confirm();
    assert.equal(c.status, 'applied');
    assert.equal(calls.addToCart, 1, 'applied exactly once, after the human confirm');
    assert.equal(gate.pending, null);
  });

  it('cancel() discards a pending STATEFUL proposal without applying it', () => {
    const { calls, handlers } = makeSpies();
    const gate = new ConfirmationGate(handlers);
    gate.submit(proposal('ADD_TO_CART', { productId: 'p1' }));
    gate.cancel();
    assert.equal(gate.pending, null);
    assert.equal(gate.confirm().status, 'rejected', 'nothing left to confirm');
    assert.equal(calls.addToCart, undefined, 'never applied');
  });

  // Money / checkout-write / order-finalization have NO IntentKind → REJECT, even if a matcher bug
  // emits them, and even if confirm() is then called.
  it('excluded money/checkout intents are REJECTed and never reach a handler', () => {
    for (const kind of ['PLACE_ORDER', 'PAY', 'SET_DELIVERY_ADDRESS', 'CHECKOUT_SUBMIT', 'SET_TIP']) {
      const { calls, handlers } = makeSpies();
      const gate = new ConfirmationGate(handlers);
      const r = gate.submit(proposal(kind, { amount: 999 }));
      assert.equal(r.status, 'rejected', `${kind} must be rejected`);
      assert.equal(gate.pending, null, `${kind} must not become pending`);
      gate.confirm(); // confirm after a reject has nothing pending → still no mutation
      assert.equal(Object.keys(calls).length, 0, `${kind} must call zero handlers`);
    }
  });

  it('unknown intent kind is REJECTed fail-closed (no handler called)', () => {
    const { calls, handlers } = makeSpies();
    const gate = new ConfirmationGate(handlers);
    const r = gate.submit(proposal('FLARGLE_BARGLE', {}));
    assert.equal(r.status, 'rejected');
    assert.equal(Object.keys(calls).length, 0);
  });

  // Dietary/allergen-named category selection is touch-only (breaker R2-B) — closes the CLASS.
  it('dietary-named SELECT_CATEGORY is rejected; a normal category applies', () => {
    const { calls, handlers } = makeSpies();
    const gate = new ConfirmationGate(handlers);
    for (const name of ['Pa gluten', 'Vegan', 'Gluten-Free', 'без лактози', 'Free From']) {
      const r = gate.submit(proposal('SELECT_CATEGORY', { categoryName: name }));
      assert.equal(r.status, 'rejected', `dietary category "${name}" must be rejected`);
    }
    assert.equal(calls.selectCategory, undefined, 'no dietary category ever applied');

    const ok = gate.submit(proposal('SELECT_CATEGORY', { categoryName: 'Pizza' }));
    assert.equal(ok.status, 'applied');
    assert.equal(calls.selectCategory, 1, 'a normal category applies (not vacuous)');
  });

  // Anti-vacuity for classify(): the table is live and money/dietary/admin/courier are NOT in it.
  it('classify() maps known kinds and fail-closes everything else', () => {
    assert.equal(classify('ADD_TO_CART'), 'STATEFUL');
    assert.equal(classify('SET_SORT'), 'READ_ONLY');
    assert.equal(classify('READ_ORDER'), 'READ_ONLY');
    assert.equal(classify('NAVIGATE_CHECKOUT'), 'READ_ONLY');
    for (const excluded of [
      'PLACE_ORDER',
      'PAY',
      'SET_FILTER_ALLERGEN',
      'COURIER_ARRIVED',
      'SET_ORDER_STATUS',
      '',
    ]) {
      assert.equal(classify(excluded), 'REJECT', `${excluded} must be REJECT`);
    }
  });
});
