import { describe, it, mock } from 'node:test';
import assert from 'node:assert/strict';
import { MockProvider } from '@deliveryos/voice';
import { buildMenuContext } from './menuContext.js';
import { createVoiceGate } from './gate.js';
import type { VoiceMenuCategory, VoiceNoMatch, VoiceStorefrontDeps } from './types.js';

// PR-2 proof (docs/design/voice-control/PHASE1-IMPLEMENTATION-PLAN.md §6): sq/en/uk transcripts →
// the SAME MockProvider-driven matcher the engine uses → the real ConfirmationGate → THIS adapter's
// handlers, with STUB storefront setters (never MenuPage/ClientLayout — the adapter is unit-testable
// in isolation, per the PR-2 design constraint). Expected values below are written independently of
// handlers.ts's internal SORT_MAP/MACRO_LENS_MAP tables (literal strings, not re-imports), so this
// cannot pass by tautology if the mapping tables are wrong.

const MENU: readonly VoiceMenuCategory[] = [
  {
    id: 'c-grill',
    name: 'Grill',
    products: [
      { id: 'p-sufllaqe', name: 'Sufllaqe', price: 450, available: true, hasRequiredModifiers: false },
      { id: 'p-cheeseburger', name: 'Cheeseburger', price: 700, available: true, hasRequiredModifiers: true },
    ],
  },
  {
    id: 'c-salads',
    name: 'Salads',
    products: [
      { id: 'p-greek', name: 'Greek Salad', price: 500, available: true, hasRequiredModifiers: false },
    ],
  },
  {
    id: 'c-pizza',
    name: 'Pizza',
    products: [
      { id: 'p-margherita', name: 'Margherita Pizza', price: 600, available: true, hasRequiredModifiers: false },
    ],
  },
  // dietary-named — the ConfirmationGate must drop this before the adapter ever sees it
  // (confirmation-gate.ts:49-61 / dietary-denylist.ts).
  { id: 'c-glutenfree', name: 'Pa gluten', products: [] },
];

const MENU_CONTEXT = buildMenuContext(MENU);

function getProduct(productId: string) {
  for (const c of MENU) {
    const p = c.products.find((x) => x.id === productId);
    if (p) return p;
  }
  return undefined;
}

interface Spies {
  readonly deps: VoiceStorefrontDeps;
  readonly addItem: ReturnType<typeof mock.fn>;
  readonly setSortBy: ReturnType<typeof mock.fn>;
  readonly setMacroLens: ReturnType<typeof mock.fn>;
  readonly setSelectedCategory: ReturnType<typeof mock.fn>;
  readonly setSearchQuery: ReturnType<typeof mock.fn>;
  readonly toggleCompare: ReturnType<typeof mock.fn>;
  readonly onReadOrder: ReturnType<typeof mock.fn>;
  readonly onNavigateCheckout: ReturnType<typeof mock.fn>;
  readonly onNoMatch: ReturnType<typeof mock.fn>;
  readonly noMatches: VoiceNoMatch[];
}

function makeSpies(filterLensesEnabled: boolean): Spies {
  const noMatches: VoiceNoMatch[] = [];
  const addItem = mock.fn();
  const setSortBy = mock.fn();
  const setMacroLens = mock.fn();
  const setSelectedCategory = mock.fn();
  const setSearchQuery = mock.fn();
  const toggleCompare = mock.fn();
  const onReadOrder = mock.fn();
  const onNavigateCheckout = mock.fn();
  const onNoMatch = mock.fn((info: VoiceNoMatch) => { noMatches.push(info); });
  const deps: VoiceStorefrontDeps = {
    getProduct,
    addItem: addItem as unknown as VoiceStorefrontDeps['addItem'],
    setSortBy: setSortBy as unknown as VoiceStorefrontDeps['setSortBy'],
    setMacroLens: setMacroLens as unknown as VoiceStorefrontDeps['setMacroLens'],
    filterLensesEnabled,
    setSelectedCategory: setSelectedCategory as unknown as VoiceStorefrontDeps['setSelectedCategory'],
    setSearchQuery: setSearchQuery as unknown as VoiceStorefrontDeps['setSearchQuery'],
    toggleCompare: toggleCompare as unknown as VoiceStorefrontDeps['toggleCompare'],
    onReadOrder: onReadOrder as unknown as VoiceStorefrontDeps['onReadOrder'],
    onNavigateCheckout: onNavigateCheckout as unknown as VoiceStorefrontDeps['onNavigateCheckout'],
    onNoMatch: onNoMatch as unknown as VoiceStorefrontDeps['onNoMatch'],
  };
  return { deps, addItem, setSortBy, setMacroLens, setSelectedCategory, setSearchQuery, toggleCompare, onReadOrder, onNavigateCheckout, onNoMatch, noMatches };
}

async function submitAll(gate: ReturnType<typeof createVoiceGate>, transcripts: readonly string[], locale: 'sq' | 'en' | 'uk') {
  const provider = new MockProvider(transcripts, locale, MENU_CONTEXT);
  const statuses: string[] = [];
  for await (const proposal of provider.intents()) {
    statuses.push(gate.submit(proposal).status);
  }
  return statuses;
}

describe('voice adapter (PR-2) — READ_ONLY intents call their real setters', () => {
  it('en: sort by price → setSortBy("price-asc") — the §2.2 direction mapping, an independent literal', async () => {
    const s = makeSpies(true);
    const gate = createVoiceGate(s.deps);
    await submitAll(gate, ['sort by price'], 'en');
    assert.equal(s.setSortBy.mock.calls.length, 1);
    assert.equal(s.setSortBy.mock.calls[0]?.arguments[0], 'price-asc');
  });

  it('en: sort by name → setSortBy("name")', async () => {
    const s = makeSpies(true);
    const gate = createVoiceGate(s.deps);
    await submitAll(gate, ['sort by name'], 'en');
    assert.equal(s.setSortBy.mock.calls.length, 1);
    assert.equal(s.setSortBy.mock.calls[0]?.arguments[0], 'name');
  });

  it('en: search greek salad → setSearchQuery("greek salad")', async () => {
    const s = makeSpies(true);
    const gate = createVoiceGate(s.deps);
    await submitAll(gate, ['search greek salad'], 'en');
    assert.equal(s.setSearchQuery.mock.calls.length, 1);
    assert.equal(s.setSearchQuery.mock.calls[0]?.arguments[0], 'greek salad');
  });

  it('en: compare margherita → toggleCompare("p-margherita")', async () => {
    const s = makeSpies(true);
    const gate = createVoiceGate(s.deps);
    await submitAll(gate, ['compare margherita'], 'en');
    assert.equal(s.toggleCompare.mock.calls.length, 1);
    assert.equal(s.toggleCompare.mock.calls[0]?.arguments[0], 'p-margherita');
  });

  it('en: show category pizza → setSelectedCategory("c-pizza")', async () => {
    const s = makeSpies(true);
    const gate = createVoiceGate(s.deps);
    await submitAll(gate, ['show category pizza'], 'en');
    assert.equal(s.setSelectedCategory.mock.calls.length, 1);
    assert.equal(s.setSelectedCategory.mock.calls[0]?.arguments[0], 'c-pizza');
  });

  it('en: read my order / go to checkout → onReadOrder / onNavigateCheckout (navigation + read-back ONLY)', async () => {
    const s = makeSpies(true);
    const gate = createVoiceGate(s.deps);
    await submitAll(gate, ['read my order', 'go to checkout'], 'en');
    assert.equal(s.onReadOrder.mock.calls.length, 1);
    assert.equal(s.onNavigateCheckout.mock.calls.length, 1);
  });

  it('sq: trego makro sipas proteinave → setMacroLens("protein-desc") when lenses are enabled', async () => {
    const s = makeSpies(true);
    const gate = createVoiceGate(s.deps);
    await submitAll(gate, ['trego makro sipas proteinave'], 'sq');
    assert.equal(s.setMacroLens.mock.calls.length, 1);
    assert.equal(s.setMacroLens.mock.calls[0]?.arguments[0], 'protein-desc');
  });

  it('sq: sipas kalorive → setMacroLens("kcal-asc") when lenses are enabled', async () => {
    const s = makeSpies(true);
    const gate = createVoiceGate(s.deps);
    await submitAll(gate, ['sipas kalorive'], 'sq');
    assert.equal(s.setMacroLens.mock.calls.length, 1);
    assert.equal(s.setMacroLens.mock.calls[0]?.arguments[0], 'kcal-asc');
  });

  it('uk: сортувати за ціною → setSortBy("price-asc")', async () => {
    const s = makeSpies(true);
    const gate = createVoiceGate(s.deps);
    await submitAll(gate, ['сортувати за ціною'], 'uk');
    assert.equal(s.setSortBy.mock.calls.length, 1);
    assert.equal(s.setSortBy.mock.calls[0]?.arguments[0], 'price-asc');
  });

  it('uk: покажи категорію pizza → setSelectedCategory("c-pizza") (latin menu name resolves regardless of speech locale)', async () => {
    const s = makeSpies(true);
    const gate = createVoiceGate(s.deps);
    await submitAll(gate, ['покажи категорію pizza'], 'uk');
    assert.equal(s.setSelectedCategory.mock.calls.length, 1);
    assert.equal(s.setSelectedCategory.mock.calls[0]?.arguments[0], 'c-pizza');
  });
});

describe('voice adapter (PR-2) — ADD_TO_CART is confirm-gated (STATEFUL held until a human tap)', () => {
  it('sq: shto dy sufllaqe → held pending, addItem NOT called; confirm() builds the FULL CartItem from menu data', async () => {
    const s = makeSpies(true);
    const gate = createVoiceGate(s.deps);
    const statuses = await submitAll(gate, ['shto dy sufllaqe'], 'sq');

    assert.deepEqual(statuses, ['pending-confirm']);
    assert.equal(s.addItem.mock.calls.length, 0, 'NOT applied on submit — the gate held it');
    assert.notEqual(gate.pending, null);

    gate.confirm(); // the human taps the confirm chip

    assert.equal(s.addItem.mock.calls.length, 1, 'applied exactly once, after confirm');
    // Independent expected literal — Sufllaqe is priced 450 in the test menu above, qty parsed
    // from "dy" (sq for two, matcher.ts NUMBER_WORDS) — NOT re-derived from handlers.ts.
    assert.deepEqual(s.addItem.mock.calls[0]?.arguments[0], {
      id: 'voice_p-sufllaqe',
      productId: 'p-sufllaqe',
      name: 'Sufllaqe',
      quantity: 2,
      price: 450,
      options: {},
    });
  });

  it('en: add cheeseburger → held pending, but confirm() does NOT add — the product requires modifiers the matcher cannot supply', async () => {
    const s = makeSpies(true);
    const gate = createVoiceGate(s.deps);
    const statuses = await submitAll(gate, ['add cheeseburger'], 'en');

    assert.deepEqual(statuses, ['pending-confirm'], 'the GATE still holds it — capability classification does not know about modifiers');
    gate.confirm();

    assert.equal(s.addItem.mock.calls.length, 0, 'never added at a possibly-wrong base price');
    assert.equal(s.onNoMatch.mock.calls.length, 1);
    assert.equal((s.onNoMatch.mock.calls[0]?.arguments[0] as VoiceNoMatch).reason, 'requires-modifiers');
  });

  it('cancel() discards the pending proposal — a re-tap barge-in never lets a stale ADD apply', async () => {
    const s = makeSpies(true);
    const gate = createVoiceGate(s.deps);
    await submitAll(gate, ['add cheeseburger'], 'en');
    gate.cancel();
    gate.confirm(); // no pending → no-op reject, addItem must still never be called
    assert.equal(s.addItem.mock.calls.length, 0);
  });
});

describe('voice adapter (PR-2) — a dietary-named category is dropped, never reaches the adapter', () => {
  it('sq: trego kategorinë pa gluten → gate REJECTs it; setSelectedCategory is never called', async () => {
    const s = makeSpies(true);
    const gate = createVoiceGate(s.deps);
    const statuses = await submitAll(gate, ['trego kategorinë pa gluten'], 'sq');
    assert.ok(statuses.includes('rejected'), 'the dietary category produced a rejection');
    assert.equal(s.setSelectedCategory.mock.calls.length, 0);
  });
});

describe('voice adapter (PR-2) — §2.2 arg-mismatch: "popularity" sort has no real setter', () => {
  it('sq: rendit sipas popullaritetit → setSortBy is NEVER called (no wrong sort applied); onNoMatch fires instead', async () => {
    const s = makeSpies(true);
    const gate = createVoiceGate(s.deps);
    const statuses = await submitAll(gate, ['rendit sipas popullaritetit'], 'sq');

    // The GATE applies it (SET_SORT is READ_ONLY capability-wise — the mismatch lives in the
    // adapter, not the capability table), but the HANDLER must not guess a sort.
    assert.deepEqual(statuses, ['applied']);
    assert.equal(s.setSortBy.mock.calls.length, 0, 'no setSortBy call for an unmapped sort key — never a wrong sort');
    assert.equal(s.onNoMatch.mock.calls.length, 1);
    assert.equal((s.onNoMatch.mock.calls[0]?.arguments[0] as VoiceNoMatch).kind, 'SET_SORT');
  });

  it('uk: за популярністю → same no-op, independent of locale', async () => {
    const s = makeSpies(true);
    const gate = createVoiceGate(s.deps);
    await submitAll(gate, ['за популярністю'], 'uk');
    assert.equal(s.setSortBy.mock.calls.length, 0);
    assert.equal(s.onNoMatch.mock.calls.length, 1);
  });
});

describe('voice adapter (PR-2) — macro-lens gaps beyond the plan\'s documented §2.2 (found while wiring)', () => {
  it('setMacroLens degrades to no-match when FILTER_LENSES_ENABLED is off, even for a mapped lens', async () => {
    const s = makeSpies(false); // flag OFF
    const gate = createVoiceGate(s.deps);
    await submitAll(gate, ['trego makro sipas proteinave'], 'sq');
    assert.equal(s.setMacroLens.mock.calls.length, 0, 'no dead apply while the lens surface itself is flagged off');
    assert.equal(s.onNoMatch.mock.calls.length, 1);
    assert.equal((s.onNoMatch.mock.calls[0]?.arguments[0] as VoiceNoMatch).reason, 'filter-lenses-disabled');
  });

  it('"carbs"/"fat" lenses have no MenuPage equivalent — no-op even when the flag is on', async () => {
    const s = makeSpies(true);
    const gate = createVoiceGate(s.deps);
    // "show macros" is a real SET_MACRO_LENS trigger; "carbs" drives detectMacroLens to 'carbs'
    // (matcher.ts:163-164), a lens MenuPage never exposes a button for.
    await submitAll(gate, ['show macros by carbs'], 'en');
    assert.equal(s.setMacroLens.mock.calls.length, 0);
    assert.equal(s.onNoMatch.mock.calls.length, 1);
    assert.equal((s.onNoMatch.mock.calls[0]?.arguments[0] as VoiceNoMatch).kind, 'SET_MACRO_LENS');
  });
});

describe('voice adapter (PR-2) — no voice→money binding beyond confirm-gated add-to-cart', () => {
  it('VoiceStorefrontDeps carries no place-order/pay/checkout-write field — only navigation + read-back', async () => {
    const s = makeSpies(true);
    const keys = Object.keys(s.deps);
    for (const forbidden of ['placeOrder', 'pay', 'submitOrder', 'checkoutWrite', 'setAddress', 'setPhone']) {
      assert.ok(!keys.includes(forbidden), `VoiceStorefrontDeps must never carry a ${forbidden} field`);
    }
  });
});
