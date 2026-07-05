// Voice PR-2 (headless web adapter) — docs/design/voice-control/PHASE1-IMPLEMENTATION-PLAN.md §6.
//
// This module maps the @deliveryos/voice ENGINE's semantic IntentProposals to the real storefront
// setters (MenuPage.tsx / CartProvider.tsx / ClientLayout.tsx). It is a pure dependency-injection
// boundary: every setter/lookup the adapter needs is a field on VoiceStorefrontDeps, injected by
// whoever mounts it. This file (and handlers.ts/gate.ts/menuContext.ts) never imports MenuPage.tsx
// or ClientLayout.tsx — only their setter SIGNATURES are mirrored here — so the adapter stays
// independently unit-testable with stubs and the future MicFab (PR-3) is the only thing that wires
// it to the real page.

import type { CartItem } from '@deliveryos/ui';

/**
 * The subset of a menu product the adapter needs to safely build a full CartItem for ADD_TO_CART
 * (§2.2: "the adapter must resolve the full product (price, modifiers, currency) from the
 * already-loaded menu data by productId"). CartItem (packages/ui/src/components/client/ClientUI.tsx)
 * carries no currency field — the storefront is single-currency per tenant — so only price + options
 * are needed here.
 */
export interface VoiceMenuProduct {
  readonly id: string;
  readonly name: string;
  readonly price: number;
  readonly available: boolean;
  /**
   * True when the product has at least one REQUIRED modifier group (min_select > 0, mirroring
   * MenuPage's ModifierGroup.required / canAdd()). The matcher carries no modifier slot
   * (matcher.ts:226-231 emits only { productId, productName, qty }), so such a product cannot be
   * safely priced from voice alone. addToCart drops it (fail-quiet, §2.2 "drop rather than guess")
   * instead of adding at a wrong (base) price — the same guard MenuPage's own quick-add already
   * applies via `!product.modifier_groups?.length` before calling addItem directly.
   */
  readonly hasRequiredModifiers: boolean;
}

/** The subset of a menu category the MenuContext builder + selectCategory handler need. */
export interface VoiceMenuCategory {
  readonly id: string;
  readonly name: string;
  readonly products: readonly VoiceMenuProduct[];
}

/** MenuPage's real `sortBy` union (MenuPage.tsx: `useState<'default' | 'price-asc' | 'price-desc' | 'name'>`). */
export type SortByValue = 'default' | 'price-asc' | 'price-desc' | 'name';

/**
 * MenuPage's real macro-lens union. NOTE: this is NOT the matcher's SortKey/MacroLens vocabulary —
 * it is packages/ui/src/lib/characteristics.ts's direction-qualified `MacroLens` type
 * ('kcal-asc' | 'kcal-desc' | 'protein-asc' | 'protein-desc') plus MenuPage's own 'none' "lens off"
 * state. See handlers.ts for why this is a THIRD arg-mismatch beyond the plan's documented §2.2.
 */
export type MacroLensValue = 'none' | 'kcal-asc' | 'kcal-desc' | 'protein-asc' | 'protein-desc';

/**
 * A voice intent that matched (the gate applied/held it) but could not map to a real setter — the
 * §2.2 fail-quiet path ("drop rather than guess"). PR-3's UI surfaces this as the
 * `voice.err.no_match` toast; it is NEVER a wrong apply.
 */
export interface VoiceNoMatch {
  readonly kind: string;
  readonly reason: string;
}

/**
 * Every real storefront setter/lookup the adapter dispatches into, injected by whoever mounts it
 * (the future MicFab, PR-3). This interface is the seam between "the ConfirmationGate decided to
 * apply" and "the real DOM changes" — nothing in this package reaches into MenuPage/ClientLayout
 * module scope directly, which is what keeps createVoiceHandlers unit-testable with plain stubs.
 */
export interface VoiceStorefrontDeps {
  /** Resolve a product by id from the currently loaded tenant menu (for ADD_TO_CART pricing). */
  readonly getProduct: (productId: string) => VoiceMenuProduct | undefined;
  /** CartProvider.addItem (apps/web/src/lib/CartProvider.tsx) — the ONLY money-adjacent call this
   *  adapter makes, and it is reachable ONLY via ConfirmationGate#confirm() (an explicit human tap
   *  on the confirm chip), never from #submit(). No place-order/pay path exists anywhere here. */
  readonly addItem: (item: CartItem) => void;
  /** MenuPage's setSortBy. */
  readonly setSortBy: (value: SortByValue) => void;
  /** MenuPage's setMacroLens. */
  readonly setMacroLens: (value: MacroLensValue) => void;
  /** Mirrors MenuPage's FILTER_LENSES_ENABLED build flag — injected (not re-read from
   *  import.meta.env) so the adapter stays a pure function of its deps and is testable without a
   *  Vite env shim. */
  readonly filterLensesEnabled: boolean;
  /** MenuPage's setSelectedCategory. Dietary-named categories never reach this handler — the
   *  ConfirmationGate drops them before #apply() dispatches (confirmation-gate.ts:49-61). */
  readonly setSelectedCategory: (categoryId: string | null) => void;
  /** MenuPage's setSearchQuery. */
  readonly setSearchQuery: (query: string) => void;
  /** MenuPage's toggleCompare(id). */
  readonly toggleCompare: (productId: string) => void;
  /** PR-3 opens the read-back panel from the customer's OWN cart (ADR-0015 §6 — no PII egress,
   *  own cart only). This adapter carries no cart data into the call; the UI already has it. */
  readonly onReadOrder: () => void;
  /** ClientLayout's setCheckoutOpen(true) — navigation only, no field write, no place-order (§2/§7). */
  readonly onNavigateCheckout: () => void;
  /** Fired for a matched-but-unmappable intent (§2.2) — e.g. `by:'popularity'`, an unresolved
   *  ADD_TO_CART product, an uncovered macro lens, or SET_MACRO_LENS while the lens surface is
   *  flagged off. Optional so tests can omit it when a case doesn't need the assertion. */
  readonly onNoMatch?: (info: VoiceNoMatch) => void;
}
