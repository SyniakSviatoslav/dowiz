import type { CartItem } from '@deliveryos/ui';

/** Current menu price/availability for a product, as seen on the freshly-loaded menu. */
export interface ReconcileProduct { id: string; price: number; available?: boolean; }

/** What changed when the cart was reconciled to a newer menu_version. */
export interface ReconcileSummary {
  repriced: { name: string; from: number; to: number }[];
  removed: string[];
}

export interface ReconcileResult {
  items: CartItem[];
  pricedVersion: number;
  /** Non-null only when something actually changed — drives a non-blocking notice. */
  summary: ReconcileSummary | null;
}

function hasModifiers(item: CartItem): boolean {
  const opts = (item as { options?: Record<string, string[]> }).options;
  return !!opts && Object.values(opts).some(v => Array.isArray(v) && v.length > 0);
}

/**
 * F9: pure cart↔menu_version reconciliation. Keeps the persisted cart in sync with a
 * freshly-loaded menu so checkout never ambushes the customer with a price/availability
 * change the server would hard-block.
 *
 * - Re-prices modifier-free lines whose stored price drifted from the current menu price.
 *   (A line with modifiers bundles deltas the base price can't verify FE-side, so it's
 *   left intact — the server still guards that rarer case.)
 * - Drops items no longer on the menu (sold-out/deleted; the public menu only lists
 *   available products).
 * - Fast-paths to a no-op when the cart is empty or already stamped to this menu_version.
 *
 * Returns the (possibly unchanged) items, the version to stamp, and a change summary.
 */
export function reconcileCart(
  items: CartItem[],
  pricedVersion: number | null,
  menuVersion: number,
  products: ReconcileProduct[],
): ReconcileResult {
  if (items.length === 0) return { items, pricedVersion: menuVersion, summary: null };
  if (pricedVersion === menuVersion) return { items, pricedVersion, summary: null };

  const lookup = new Map(products.map(p => [p.id, p]));
  const repriced: ReconcileSummary['repriced'] = [];
  const removed: string[] = [];
  const next: CartItem[] = [];
  for (const it of items) {
    const p = lookup.get(it.productId);
    if (!p || p.available === false) { removed.push(it.name); continue; }
    if (!hasModifiers(it) && typeof p.price === 'number' && p.price !== it.price) {
      repriced.push({ name: it.name, from: it.price, to: p.price });
      next.push({ ...it, price: p.price });
    } else {
      next.push(it);
    }
  }
  const summary = repriced.length || removed.length ? { repriced, removed } : null;
  return { items: next, pricedVersion: menuVersion, summary };
}
