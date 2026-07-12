/*
 * Cinematic reveals — the shared-element (layoutId) naming SoT + the flag gate.
 *
 * The card→detail morph works only when the OUTGOING node (ProductCard) and the INCOMING
 * node (ProductDetailSheet) carry the SAME `layoutId`. Centralise the id derivation here so
 * the two surfaces can never drift — a typo on one side silently kills the morph.
 *
 * Flag-dark: everything is gated behind `VITE_CINEMATIC_REVEALS` (default OFF). Flag off →
 * `layoutId` is `undefined` on every node → framer attaches no shared-layout projection →
 * behaviour is byte-identical to today. Reduced-motion is a SECOND, independent gate applied
 * at the call site (via `useReducedMotion()`), so accessibility always wins over the flag.
 */

/** The three shared leaf nodes that fly from card into the detail sheet. */
export type RevealNode = 'media' | 'title' | 'price';

/**
 * Stable, collision-free `layoutId` for a shared node, namespaced under `product-` and keyed
 * by node kind + product id. Distinct per kind so the image/title/price morph independently
 * (a single id across three nodes would make framer treat them as one element).
 */
export function revealLayoutId(node: RevealNode, id: string): string {
  return `product-${node}-${id}`;
}

/**
 * Is the cinematic-reveal flag on? Reads the Vite build-time env; defaults OFF everywhere the
 * var is absent (SSR, tests, unset builds). Kept tiny + pure so it is unit-testable and so the
 * call sites stay declarative.
 */
export function cinematicRevealsEnabled(): boolean {
  try {
    // `import.meta.env` exists only under Vite; guard for SSR / node:test / other bundlers.
    return (import.meta as unknown as { env?: Record<string, string | undefined> }).env
      ?.VITE_CINEMATIC_REVEALS === 'true';
  } catch {
    return false;
  }
}
