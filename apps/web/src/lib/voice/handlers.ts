import type { VoiceHandlers } from '@deliveryos/voice';
import type { MacroLensValue, SortByValue, VoiceStorefrontDeps } from './types.js';

// ── §2.2 arg-mismatch #1: sort ──────────────────────────────────────────────────────────────────
// The matcher emits SortKey 'price' | 'popularity' | 'name' (matcher.ts:17, detectSortKey :143-158).
// MenuPage.sortBy is 'default' | 'price-asc' | 'price-desc' | 'name' — there is no 'popularity' sort
// and 'price' must resolve to a direction. Map price → 'price-asc' (matches the "cheapest
// first"/"me te lira" trigger wording); name → 'name'; popularity has NO real-setter equivalent, so
// it maps to `undefined` and the handler no-ops + reports onNoMatch instead of guessing a sort.
const SORT_MAP: Readonly<Record<string, SortByValue | undefined>> = {
  price: 'price-asc',
  name: 'name',
  popularity: undefined,
};

// ── §2.2 arg-mismatch #2 (plus a THIRD gap the plan's premise missed) — macro lens ─────────────────
// The matcher emits MacroLens 'protein' | 'calories' | 'carbs' | 'fat' (matcher.ts:18,
// detectMacroLens :160-166). The plan document assumed "MenuPage's MacroLens union covers all
// four" — it does not: MenuPage's REAL macro-lens union is packages/ui/src/lib/characteristics.ts's
// direction-qualified `'kcal-asc' | 'kcal-desc' | 'protein-asc' | 'protein-desc'`, and the UI only
// ever renders TWO of those four values (MenuPage.tsx macro-lens toolbar: 'protein-desc' "Most
// protein" and 'kcal-asc' "Calories: low to high" — there is no button for 'kcal-desc' or
// 'protein-asc' either). So: protein → 'protein-desc' (the only protein lens exposed), calories →
// 'kcal-asc' (the only calories lens exposed), carbs/fat → no MenuPage lens exists at all → no-op.
const MACRO_LENS_MAP: Readonly<Record<string, MacroLensValue | undefined>> = {
  protein: 'protein-desc',
  calories: 'kcal-asc',
  carbs: undefined,
  fat: undefined,
};

function str(args: Readonly<Record<string, unknown>>, key: string): string | undefined {
  const v = args[key];
  return typeof v === 'string' && v.length > 0 ? v : undefined;
}

function num(args: Readonly<Record<string, unknown>>, key: string): number | undefined {
  const v = args[key];
  return typeof v === 'number' && Number.isFinite(v) ? v : undefined;
}

/**
 * Build the VoiceHandlers port the ConfirmationGate dispatches into (confirmation-gate.ts:12-21,
 * :91-119). A pure function of `deps` — imports no MenuPage/ClientLayout module, so it is
 * unit-testable with plain stubs; the future MicFab (PR-3) is the only thing that supplies real
 * setters. Resolves the two documented arg-mismatches (§2.2) plus the macro-lens gap found above;
 * every case that cannot be mapped to a real setter calls `deps.onNoMatch` and returns WITHOUT
 * calling a setter — never a guessed apply (§2.2 "drop rather than guess").
 *
 * No voice→money binding beyond confirm-gated add-to-cart lives here: `addToCart` is the only
 * handler that touches the cart, and the ConfirmationGate is the only caller — there is no
 * place-order/pay/checkout-write path in this file or anywhere the adapter reaches.
 */
export function createVoiceHandlers(deps: VoiceStorefrontDeps): VoiceHandlers {
  return {
    // STATEFUL — the ConfirmationGate calls this ONLY from #confirm() (an explicit human tap on the
    // confirm chip), never from #submit(). See gate.ts / confirmation-gate.ts:44-73 vs :76-84.
    // Returns the apply-outcome (council R-a): true ONLY after a real deps.addItem call; false on
    // every drop path, so the gate never reports `applied` ("Done ✓") over an unchanged cart.
    addToCart: (args) => {
      const productId = str(args, 'productId');
      const qty = num(args, 'qty') ?? 1;
      if (!productId || qty < 1) {
        deps.onNoMatch?.({ kind: 'ADD_TO_CART', reason: 'missing-product-or-qty' });
        return false;
      }
      const product = deps.getProduct(productId);
      // Unresolved, sold out, or requires a modifier choice the matcher can't supply → drop rather
      // than add at a possibly-wrong price (§2.2). No handler call to deps.addItem = no cart mutation.
      if (!product) {
        deps.onNoMatch?.({ kind: 'ADD_TO_CART', reason: 'unresolved-product' });
        return false;
      }
      if (!product.available) {
        deps.onNoMatch?.({ kind: 'ADD_TO_CART', reason: 'unavailable' });
        return false;
      }
      if (product.hasRequiredModifiers) {
        deps.onNoMatch?.({ kind: 'ADD_TO_CART', reason: 'requires-modifiers' });
        return false;
      }
      deps.addItem({
        id: `voice_${product.id}`,
        productId: product.id,
        name: product.name,
        quantity: Math.max(1, Math.floor(qty)),
        price: product.price,
        options: {},
      });
      return true;
    },

    setSort: (args) => {
      const by = str(args, 'by');
      const mapped = by ? SORT_MAP[by] : undefined;
      if (!mapped) {
        deps.onNoMatch?.({ kind: 'SET_SORT', reason: `unmapped-sort-key:${by ?? 'unknown'}` });
        return;
      }
      deps.setSortBy(mapped);
    },

    setMacroLens: (args) => {
      // §2.2 point 2: the lens surface itself is behind FILTER_LENSES_ENABLED — if it's off, degrade
      // to no-match rather than a "dead apply" that changes state nothing on screen reflects.
      if (!deps.filterLensesEnabled) {
        deps.onNoMatch?.({ kind: 'SET_MACRO_LENS', reason: 'filter-lenses-disabled' });
        return;
      }
      const lens = str(args, 'lens');
      const mapped = lens ? MACRO_LENS_MAP[lens] : undefined;
      if (!mapped) {
        deps.onNoMatch?.({ kind: 'SET_MACRO_LENS', reason: `unmapped-lens:${lens ?? 'unknown'}` });
        return;
      }
      deps.setMacroLens(mapped);
    },

    // Dietary/allergen-named categories never reach here — the ConfirmationGate downgrades them to
    // REJECT before #apply() ever dispatches (confirmation-gate.ts:49-61), so no re-check is needed
    // at this layer (defence-in-depth already lives in the gate + the matcher-side denylist).
    selectCategory: (args) => {
      const categoryId = str(args, 'categoryId');
      if (!categoryId) {
        deps.onNoMatch?.({ kind: 'SELECT_CATEGORY', reason: 'missing-category-id' });
        return;
      }
      deps.setSelectedCategory(categoryId);
    },

    setSearch: (args) => {
      const query = str(args, 'query');
      if (!query) {
        deps.onNoMatch?.({ kind: 'SET_SEARCH', reason: 'missing-query' });
        return;
      }
      deps.setSearchQuery(query);
    },

    toggleCompare: (args) => {
      const productId = str(args, 'productId');
      if (!productId) {
        deps.onNoMatch?.({ kind: 'TOGGLE_COMPARE', reason: 'missing-product-id' });
        return;
      }
      deps.toggleCompare(productId);
    },

    readOrder: () => deps.onReadOrder(),

    navigateCheckout: () => deps.onNavigateCheckout(),
  };
}


