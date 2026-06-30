import type { IntentProposal } from './types.js';
import { normalize } from './normalize.js';

// Deterministic intent matcher: a transcript string → an IntentProposal (or null below threshold).
// This is the rule-based half of the Phase-0 gate — pure text → intent, no model, fully unit-testable
// (the sq/en/uk corpus in __tests__/matcher.test.ts measures it). It emits SEMANTIC args (e.g.
// { by: 'price' }) and never imports apps/web; the web adapter maps semantic args to the real
// MenuPage setters. The matcher PRODUCES proposals only — the ConfirmationGate is the sole sink.

export type Locale = 'sq' | 'en' | 'uk';

export interface MenuContext {
  readonly products: ReadonlyArray<{ readonly id: string; readonly name: string }>;
  readonly categories: ReadonlyArray<{ readonly id: string; readonly name: string }>;
}

export type SortKey = 'price' | 'popularity' | 'name';
export type MacroLens = 'protein' | 'calories' | 'carbs' | 'fat';

// Confidence below this is treated as "no confident match" → null (the UI shows nothing, the gate
// is never reached). Keeps the dangerous-misfire surface small: an ambiguous utterance does nothing.
const MIN_CONFIDENCE = 0.6;

/** Per-locale trigger phrases. Checked in the order intents are evaluated below (specific first). */
const TRIGGERS: Record<Locale, Record<string, readonly string[]>> = {
  sq: {
    NAVIGATE_CHECKOUT: ['te arka', 'tek arka', 'shko te arka', 'vazhdo te arka', 'paguaj', 'perfundo porosine'],
    READ_ORDER: ['lexo porosine', 'porosia ime', 'cfare kam ne shporte', 'shporta ime', 'lexo shporten'],
    TOGGLE_COMPARE: ['krahaso'],
    SET_MACRO_LENS: ['trego makro', 'sipas proteinave', 'sipas kalorive', 'filtro makro', 'proteina', 'kalori'],
    SET_SORT: ['rendit', 'rendis', 'sipas cmimit', 'me te lira', 'sipas popullaritetit', 'sipas emrit'],
    SET_SEARCH: ['kerko', 'gjej', 'kerko per'],
    SELECT_CATEGORY: ['kategoria', 'kategorine', 'trego kategorine', 'shfaq kategorine'],
    ADD_TO_CART: ['shto', 'shtoje', 'me shto'],
  },
  en: {
    NAVIGATE_CHECKOUT: ['go to checkout', 'to checkout', 'proceed to checkout', 'check out', 'go to cart'],
    READ_ORDER: ['read my order', 'what is in my cart', 'what is in my order', 'read my cart', 'my order'],
    TOGGLE_COMPARE: ['compare'],
    SET_MACRO_LENS: ['show macros', 'by protein', 'by calories', 'macro filter', 'protein', 'calories'],
    SET_SORT: ['sort', 'sort by price', 'cheapest first', 'by popularity', 'by name'],
    SET_SEARCH: ['search', 'search for', 'find'],
    SELECT_CATEGORY: ['category', 'show category', 'show the category'],
    ADD_TO_CART: ['add', 'i want', 'put in cart'],
  },
  uk: {
    NAVIGATE_CHECKOUT: ['до кошика', 'оформити', 'оформити замовлення', 'до оплати', 'перейти до кошика'],
    READ_ORDER: ['прочитай замовлення', 'що в кошику', 'мій кошик', 'моє замовлення', 'прочитай кошик'],
    TOGGLE_COMPARE: ['порівняй', 'порівняти'],
    SET_MACRO_LENS: ['покажи макрос', 'за білком', 'за калоріями', 'макрофільтр', 'білок', 'калорії'],
    SET_SORT: ['сортувати', 'за ціною', 'найдешевші', 'за популярністю', 'за назвою'],
    SET_SEARCH: ['знайти', 'пошук', 'шукати'],
    SELECT_CATEGORY: ['категорія', 'покажи категорію', 'категорію'],
    ADD_TO_CART: ['додай', 'додати', 'хочу'],
  },
};

/** Number words → quantity, merged across locales (digits handled separately). */
const NUMBER_WORDS: Record<string, number> = {
  // sq
  nje: 1, dy: 2, tre: 3, tri: 3, kater: 4, pese: 5, gjashte: 6,
  // en
  one: 1, two: 2, three: 3, four: 4, five: 5, six: 6, a: 1, an: 1,
  // uk
  odyn: 1, odna: 1, dva: 2, dvi: 2, try: 3, chotyry: 4, pyat: 5,
  один: 1, одна: 1, два: 2, дві: 2, три: 3, чотири: 4, пять: 5,
};

function buildProposal(
  kind: string,
  args: Readonly<Record<string, unknown>>,
  confidence: number,
  transcript: string,
): IntentProposal {
  return { kind, args, transcript, confidence };
}

/** First trigger phrase from `phrases` that appears in the normalized text, else null. */
function firstHit(norm: string, phrases: readonly string[]): string | null {
  for (const p of phrases) {
    const np = normalize(p);
    if (norm.includes(np)) return np;
  }
  return null;
}

/** The text after the LAST occurrence of the trigger phrase (the slot region). */
function afterTrigger(norm: string, trigger: string): string {
  const idx = norm.lastIndexOf(trigger);
  if (idx < 0) return norm;
  return norm.slice(idx + trigger.length).trim();
}

/** Leading quantity in a slot region: a digit, "2x", or a number word. Defaults to 1. */
function parseQuantity(slot: string): { qty: number; rest: string } {
  const tokens = slot.split(' ').filter(Boolean);
  if (tokens.length === 0) return { qty: 1, rest: slot };
  const head = tokens[0] ?? '';
  const digit = head.match(/^(\d+)x?$/);
  if (digit && digit[1]) return { qty: Math.max(1, parseInt(digit[1], 10)), rest: tokens.slice(1).join(' ') };
  if (head in NUMBER_WORDS) {
    const qty = NUMBER_WORDS[head] ?? 1;
    return { qty, rest: tokens.slice(1).join(' ') };
  }
  return { qty: 1, rest: slot };
}

/** Best product whose normalized name overlaps the slot text. Returns null if nothing meaningful. */
function resolveProduct(slot: string, menu: MenuContext): { id: string; name: string } | null {
  return resolveByName(slot, menu.products);
}

function resolveCategory(slot: string, menu: MenuContext): { id: string; name: string } | null {
  return resolveByName(slot, menu.categories);
}

function resolveByName(
  slot: string,
  items: ReadonlyArray<{ readonly id: string; readonly name: string }>,
): { id: string; name: string } | null {
  const s = normalize(slot);
  if (!s) return null;
  let best: { id: string; name: string; score: number } | null = null;
  for (const item of items) {
    const n = normalize(item.name);
    if (!n) continue;
    let score = 0;
    if (s.includes(n) || n.includes(s)) {
      score = Math.min(n.length, s.length) / Math.max(n.length, s.length);
    } else {
      // token overlap fallback
      const sTokens = new Set(s.split(' '));
      const nTokens = n.split(' ');
      const hits = nTokens.filter((t) => sTokens.has(t)).length;
      score = nTokens.length ? hits / nTokens.length : 0;
    }
    if (score > 0 && (!best || score > best.score)) best = { id: item.id, name: item.name, score };
  }
  if (!best || best.score < 0.5) return null;
  return { id: best.id, name: best.name };
}

function detectSortKey(norm: string, locale: Locale): SortKey {
  const t = (s: string) => norm.includes(normalize(s));
  if (locale === 'sq') {
    if (t('popullaritet')) return 'popularity';
    if (t('emr')) return 'name';
    return 'price';
  }
  if (locale === 'uk') {
    if (t('популярн')) return 'popularity';
    if (t('назв')) return 'name';
    return 'price';
  }
  if (t('popularity')) return 'popularity';
  if (t('name')) return 'name';
  return 'price';
}

function detectMacroLens(norm: string, locale: Locale): MacroLens {
  const t = (s: string) => norm.includes(normalize(s));
  if (t('calor') || t('kalor') || t('калор')) return 'calories';
  if (t('carb') || t('karbo') || t('вуглев')) return 'carbs';
  if (t('fat') || t('yndyr') || t('жир')) return 'fat';
  return 'protein';
}

/**
 * Match a transcript to a voice IntentProposal, or null if no confident intent is found.
 * Specific/phrasal intents are tried before the generic ADD verb. Slot-less READ-ONLY intents
 * (checkout/read/sort/macro) resolve confidently; slot intents (compare/search/category/add) return
 * null when the slot does not resolve, so a mis-heard product never becomes a confident proposal.
 */
export function matchIntent(transcript: string, locale: Locale, menu: MenuContext): IntentProposal | null {
  const norm = normalize(transcript);
  if (!norm) return null;
  const T = TRIGGERS[locale];

  if (firstHit(norm, T.NAVIGATE_CHECKOUT ?? [])) {
    return buildProposal('NAVIGATE_CHECKOUT', {}, 0.9, transcript);
  }
  if (firstHit(norm, T.READ_ORDER ?? [])) {
    return buildProposal('READ_ORDER', {}, 0.9, transcript);
  }

  const compareHit = firstHit(norm, T.TOGGLE_COMPARE ?? []);
  if (compareHit) {
    const product = resolveProduct(afterTrigger(norm, compareHit), menu);
    if (!product) return null;
    return buildProposal('TOGGLE_COMPARE', { productId: product.id, productName: product.name }, 0.8, transcript);
  }

  if (firstHit(norm, T.SET_MACRO_LENS ?? [])) {
    return buildProposal('SET_MACRO_LENS', { lens: detectMacroLens(norm, locale) }, 0.8, transcript);
  }
  if (firstHit(norm, T.SET_SORT ?? [])) {
    return buildProposal('SET_SORT', { by: detectSortKey(norm, locale) }, 0.8, transcript);
  }

  const searchHit = firstHit(norm, T.SET_SEARCH ?? []);
  if (searchHit) {
    const query = afterTrigger(norm, searchHit).trim();
    if (!query) return null;
    return buildProposal('SET_SEARCH', { query }, 0.75, transcript);
  }

  const categoryHit = firstHit(norm, T.SELECT_CATEGORY ?? []);
  if (categoryHit) {
    const category = resolveCategory(afterTrigger(norm, categoryHit), menu);
    if (!category) return null;
    // categoryName is carried so the ConfirmationGate's dietary denylist can REJECT a safety-named
    // category (defence in depth — the gate is the backstop even if a dietary category resolves here).
    return buildProposal(
      'SELECT_CATEGORY',
      { categoryId: category.id, categoryName: category.name },
      0.8,
      transcript,
    );
  }

  const addHit = firstHit(norm, T.ADD_TO_CART ?? []);
  if (addHit) {
    const { qty, rest } = parseQuantity(afterTrigger(norm, addHit));
    const product = resolveProduct(rest, menu);
    if (!product) return null; // never propose adding an unresolved item
    return buildProposal(
      'ADD_TO_CART',
      { productId: product.id, productName: product.name, qty },
      0.8,
      transcript,
    );
  }

  return null;
}

export { MIN_CONFIDENCE };
