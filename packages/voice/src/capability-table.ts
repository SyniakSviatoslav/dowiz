import type { Capability, IntentKind } from './types.js';

/**
 * The capability table — the SINGLE source of truth for what each voice intent may do.
 * Typed as Record<IntentKind, …> so a NEW IntentKind with no entry FAILS THE BUILD
 * (the exhaustiveness ratchet). Money / checkout-write / dietary / admin / courier intents
 * are absent BY DESIGN: they have no IntentKind, so classify() returns REJECT for them.
 */
const CAPABILITY_TABLE: Record<IntentKind, Exclude<Capability, 'REJECT'>> = {
  ADD_TO_CART: 'STATEFUL',
  SET_SORT: 'READ_ONLY',
  SET_MACRO_LENS: 'READ_ONLY',
  SELECT_CATEGORY: 'READ_ONLY',
  SET_SEARCH: 'READ_ONLY',
  TOGGLE_COMPARE: 'READ_ONLY',
  READ_ORDER: 'READ_ONLY',
  NAVIGATE_CHECKOUT: 'READ_ONLY',
};

/**
 * Fail-closed classification. An intent kind absent from the table — unknown, future, excluded,
 * or money/dietary — returns REJECT. There is no path by which an unclassified intent reaches a
 * mutation. (ADR-0015 §6, breaker finding M4.)
 */
export function classify(kind: string): Capability {
  return (CAPABILITY_TABLE as Record<string, Capability | undefined>)[kind] ?? 'REJECT';
}
