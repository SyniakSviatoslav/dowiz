import { useCallback, useState } from 'react';
import { useNavigate } from 'react-router-dom';
import type { CartItem } from '@deliveryos/ui';
import { useSharedCart } from '../lib/CartProvider.js';
import { reconcileCart, type ReconcileProduct } from '../lib/cartReconcile.js';

// A past-order line as returned by GET /customer/orders/:id/status (snapshot fields)
// or persisted device-locally. Only productId + quantity are load-bearing; name/price
// are snapshots we re-validate against the live menu before they ever hit the cart.
export interface ReorderOrderItem {
  productId: string;
  nameSnapshot?: string;
  name?: string;
  priceSnapshot?: number;
  price?: number;
  quantity?: number;
}

export interface ReorderResult {
  /** Lines that survived re-validation and were added to the cart (live prices). */
  items: CartItem[];
  /** Names of lines dropped because they are no longer on the menu / sold out. */
  skipped: string[];
  /** Lines whose price drifted since the order — repriced to the current menu. */
  repriced: { name: string; from: number; to: number }[];
}

/**
 * Device-local reorder id. Reordered lines carry no modifiers (the order snapshot
 * has none), so an options-free deterministic id is enough — and merges cleanly
 * with an identical modifier-free line already added via the menu's add-path
 * (CartProvider.addItem keys on `productId + JSON(options)`).
 */
function reorderCartId(productId: string): string {
  return `reorder_${productId}`;
}

/**
 * Pure rehydrate + reconcile: map a past order's snapshot lines onto the freshly
 * loaded menu and reuse the canonical `reconcileCart` so pricing/availability is
 * NEVER hand-rolled. Unavailable/deleted lines are skipped (with a note), drifted
 * prices are re-validated to the live menu, and `menu_version` is stamped by the
 * cart on add. Starting `pricedVersion` at null forces full re-validation of every
 * modifier-free line (which reordered lines all are).
 */
export function rehydrateOrderItems(
  orderItems: ReorderOrderItem[],
  menuVersion: number,
  products: ReconcileProduct[],
): ReorderResult {
  const candidates: CartItem[] = orderItems
    .filter((it) => it.productId && (it.quantity ?? 1) > 0)
    .map((it) => ({
      id: reorderCartId(it.productId),
      productId: it.productId,
      name: it.nameSnapshot ?? it.name ?? '',
      quantity: it.quantity ?? 1,
      price: it.priceSnapshot ?? it.price ?? 0,
      options: {},
    }));

  const result = reconcileCart(candidates, null, menuVersion, products);
  return {
    items: result.items,
    skipped: result.summary?.removed ?? [],
    repriced: result.summary?.repriced ?? [],
  };
}

interface MenuLike {
  menu_version: number;
  categories?: Array<{ products?: Array<{ id: string; price: number; available?: boolean }> }>;
}

/**
 * Reorder hook: re-fetches the current public menu for `slug`, rehydrates the cart
 * from a past order's lines (re-validating availability + price via the shared cart
 * add-path), and — unless told otherwise — navigates to the storefront so the
 * rehydrated cart is visible. Read-only fetch, device-local: no auth/identity.
 */
export function useReorder(slug: string | undefined) {
  const { addItem, clearCart } = useSharedCart();
  const navigate = useNavigate();
  const [busy, setBusy] = useState(false);

  const reorder = useCallback(
    async (
      orderItems: ReorderOrderItem[],
      opts?: { navigateToMenu?: boolean; replace?: boolean },
    ): Promise<ReorderResult | null> => {
      if (!slug || busy) return null;
      setBusy(true);
      try {
        const res = await fetch(`/public/locations/${slug}/menu`);
        if (!res.ok) throw new Error(`menu ${res.status}`);
        const menu = (await res.json()) as MenuLike;
        const products: ReconcileProduct[] = (menu.categories || []).flatMap((c) =>
          (c.products || []).map((p) => ({ id: p.id, price: p.price, available: p.available })),
        );

        const result = rehydrateOrderItems(orderItems, menu.menu_version, products);

        if (opts?.replace) clearCart();
        for (const it of result.items) addItem(it);

        if (opts?.navigateToMenu !== false && result.items.length > 0) {
          navigate(`/s/${slug}`);
        }
        return result;
      } catch {
        return null;
      } finally {
        setBusy(false);
      }
    },
    [slug, busy, addItem, clearCart, navigate],
  );

  return { reorder, busy };
}
