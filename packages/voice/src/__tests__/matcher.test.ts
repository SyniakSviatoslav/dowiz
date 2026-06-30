import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { matchIntent } from '../matcher.js';
import type { Locale, MenuContext } from '../matcher.js';

// sq/en/uk intent corpus (council voice-control / ADR-0015 §4.2 — the deterministic, text-only half
// of the Phase-0 gate). RED if the matcher mis-resolves an intent, fires a STATEFUL ADD on an
// ambiguous/allergen utterance (dangerous-misfire), or resolves a product/category it should not.
// This is NOT the live-audio gate (≥300 real sq utterances) — that is the separate, human-mic artifact.

const MENU: MenuContext = {
  products: [
    { id: 'p-sufllaqe', name: 'Sufllaqe' },
    { id: 'p-margherita', name: 'Margherita Pizza' },
    { id: 'p-greek', name: 'Greek Salad' },
    { id: 'p-cheeseburger', name: 'Cheeseburger' },
    { id: 'p-byrek', name: 'Spinach Byrek' },
  ],
  categories: [
    { id: 'c-pizza', name: 'Pizza' },
    { id: 'c-salads', name: 'Salads' },
    { id: 'c-drinks', name: 'Drinks' },
    { id: 'c-glutenfree', name: 'Pa gluten' }, // dietary — the gate must reject voice selection
  ],
};

type ArgsSubset = Readonly<Record<string, unknown>>;
interface Case {
  readonly locale: Locale;
  readonly transcript: string;
  readonly kind: string | null; // null = must NOT produce a confident intent
  readonly args?: ArgsSubset;
}

const CASES: readonly Case[] = [
  // ── sq (Albanian) ──
  { locale: 'sq', transcript: 'Shto dy sufllaqe', kind: 'ADD_TO_CART', args: { productName: 'Sufllaqe', qty: 2 } },
  { locale: 'sq', transcript: 'shto 3 cheeseburger', kind: 'ADD_TO_CART', args: { productName: 'Cheeseburger', qty: 3 } },
  { locale: 'sq', transcript: 'rendit sipas çmimit', kind: 'SET_SORT', args: { by: 'price' } },
  { locale: 'sq', transcript: 'rendit sipas popullaritetit', kind: 'SET_SORT', args: { by: 'popularity' } },
  { locale: 'sq', transcript: 'trego makro sipas proteinave', kind: 'SET_MACRO_LENS', args: { lens: 'protein' } },
  { locale: 'sq', transcript: 'sipas kalorive', kind: 'SET_MACRO_LENS', args: { lens: 'calories' } },
  { locale: 'sq', transcript: 'trego kategorinë pizza', kind: 'SELECT_CATEGORY', args: { categoryName: 'Pizza' } },
  { locale: 'sq', transcript: 'kërko byrek', kind: 'SET_SEARCH', args: { query: 'byrek' } },
  { locale: 'sq', transcript: 'krahaso greek salad', kind: 'TOGGLE_COMPARE', args: { productName: 'Greek Salad' } },
  { locale: 'sq', transcript: 'lexo porosinë time', kind: 'READ_ORDER' },
  { locale: 'sq', transcript: 'shko te arka', kind: 'NAVIGATE_CHECKOUT' },
  // sq dangerous-misfire guards — questions / bare verbs must do NOTHING
  { locale: 'sq', transcript: 'a ka gluten kjo pjatë', kind: null },
  { locale: 'sq', transcript: 'shto', kind: null },

  // ── en ──
  { locale: 'en', transcript: 'add two cheeseburger', kind: 'ADD_TO_CART', args: { productName: 'Cheeseburger', qty: 2 } },
  { locale: 'en', transcript: 'add cheeseburger', kind: 'ADD_TO_CART', args: { productName: 'Cheeseburger', qty: 1 } },
  { locale: 'en', transcript: 'sort by price', kind: 'SET_SORT', args: { by: 'price' } },
  { locale: 'en', transcript: 'show macros by protein', kind: 'SET_MACRO_LENS', args: { lens: 'protein' } },
  { locale: 'en', transcript: 'search greek salad', kind: 'SET_SEARCH', args: { query: 'greek salad' } },
  { locale: 'en', transcript: 'compare margherita', kind: 'TOGGLE_COMPARE', args: { productName: 'Margherita Pizza' } },
  { locale: 'en', transcript: 'show category pizza', kind: 'SELECT_CATEGORY', args: { categoryName: 'Pizza' } },
  { locale: 'en', transcript: 'read my order', kind: 'READ_ORDER' },
  { locale: 'en', transcript: 'go to checkout', kind: 'NAVIGATE_CHECKOUT' },
  // en dangerous-misfire guards
  { locale: 'en', transcript: 'does it have gluten', kind: null },
  { locale: 'en', transcript: 'do these contain nuts', kind: null },

  // ── uk (Ukrainian) ──
  { locale: 'uk', transcript: 'до кошика', kind: 'NAVIGATE_CHECKOUT' },
  { locale: 'uk', transcript: 'оформити замовлення', kind: 'NAVIGATE_CHECKOUT' },
  { locale: 'uk', transcript: 'що в кошику', kind: 'READ_ORDER' },
  { locale: 'uk', transcript: 'сортувати за ціною', kind: 'SET_SORT', args: { by: 'price' } },
  { locale: 'uk', transcript: 'за популярністю', kind: 'SET_SORT', args: { by: 'popularity' } },
  { locale: 'uk', transcript: 'покажи макрос за білком', kind: 'SET_MACRO_LENS', args: { lens: 'protein' } },
  { locale: 'uk', transcript: 'знайти піца', kind: 'SET_SEARCH', args: { query: 'піца' } },
  { locale: 'uk', transcript: 'покажи категорію pizza', kind: 'SELECT_CATEGORY', args: { categoryName: 'Pizza' } },
  // uk product-slot gap (latin menu names ≠ cyrillic speech) — must fail SAFE (null), never a wrong add
  { locale: 'uk', transcript: 'додай чізбургер', kind: null },
];

describe('voice matcher — sq/en/uk intent corpus', () => {
  for (const c of CASES) {
    const label = `[${c.locale}] "${c.transcript}" → ${c.kind ?? 'null'}`;
    it(label, () => {
      const got = matchIntent(c.transcript, c.locale, MENU);
      if (c.kind === null) {
        assert.equal(got, null, `expected no confident intent, got ${got?.kind}`);
        return;
      }
      assert.notEqual(got, null, 'expected an intent, got null');
      assert.equal(got!.kind, c.kind);
      assert.ok(got!.confidence >= 0.6, `confidence ${got!.confidence} below threshold`);
      if (c.args) {
        for (const [k, v] of Object.entries(c.args)) {
          assert.deepEqual(got!.args[k], v, `arg "${k}" mismatch (got ${JSON.stringify(got!.args[k])})`);
        }
      }
    });
  }

  // Coverage assertion (anti-vacuity): the corpus must exercise every active-scope intent kind.
  it('corpus covers every active-scope intent kind', () => {
    const covered = new Set(CASES.filter((c) => c.kind).map((c) => c.kind));
    for (const kind of [
      'ADD_TO_CART',
      'SET_SORT',
      'SET_MACRO_LENS',
      'SELECT_CATEGORY',
      'SET_SEARCH',
      'TOGGLE_COMPARE',
      'READ_ORDER',
      'NAVIGATE_CHECKOUT',
    ]) {
      assert.ok(covered.has(kind), `corpus is missing coverage for ${kind}`);
    }
  });
});
