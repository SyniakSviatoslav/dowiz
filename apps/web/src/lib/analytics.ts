/**
 * Product-funnel analytics taxonomy — single source of truth.
 *
 * Explicit, named events only (NOT PostHog autocapture: autocapture re-breaks on
 * every UI change, and the order funnel is the heart of the product). Lay the
 * taxonomy before the convergence-loop frontend refactor — names are painful to
 * change retroactively once data accrues.
 *
 * This module is the *contract*. Transport (PostHog capture) is wired separately
 * once a project key exists — keeping the taxonomy testable with no dependency or
 * account, and keeping ingest behind one typed seam.
 *
 * Funnel: menu_view → item_add → cart_open → checkout_start → order_placed →
 *         courier_assigned → delivered
 */

/** The 7 funnel events, in funnel order. */
export const FUNNEL_EVENTS = [
  'menu_view',
  'item_add',
  'cart_open',
  'checkout_start',
  'order_placed',
  'courier_assigned',
  'delivered',
] as const;

export type FunnelEvent = (typeof FUNNEL_EVENTS)[number];

/**
 * Properties per event. `slug` (restaurant) is the common dimension on every
 * event so the funnel can be grouped per tenant.
 */
export interface FunnelPayloads {
  menu_view: { slug: string };
  item_add: { slug: string; productId: string; quantity: number; price: number; hasModifiers: boolean };
  cart_open: { slug: string; itemCount: number };
  checkout_start: { slug: string; itemCount: number; subtotal: number };
  order_placed: { slug: string; orderId: string; locationId: string; total: number; itemCount: number };
  courier_assigned: { slug: string; orderId: string };
  delivered: { slug: string; orderId: string };
}

/** Discriminated union of every well-formed analytics event. */
export type AnalyticsEvent = {
  [E in FunnelEvent]: { event: E; properties: FunnelPayloads[E] };
}[FunnelEvent];

/**
 * Where each event fires, kept in code so the taxonomy and the eventual wiring
 * can't drift silently. (Maps reconciled against the client surfaces, 2026-06-17.)
 * Note: `courier_assigned` fires on the app's `IN_DELIVERY` status transition
 * (when a courier is assigned and delivery begins); `delivered` on `DELIVERED`.
 */
export const FUNNEL_FIRING_SURFACE: Record<FunnelEvent, string> = {
  menu_view: 'MenuPage.tsx — loadMenu effect (route /s/:slug)',
  item_add: 'MenuPage.tsx — onAdd (quick) / handleAddDetail (modifier modal)',
  cart_open: 'ClientLayout.tsx — cart-open button',
  checkout_start: 'CheckoutPage.tsx — component mount (/s/:slug/checkout)',
  order_placed: 'CheckoutPage.tsx — handlePlaceOrder, POST /orders success',
  courier_assigned: 'OrderStatusPage.tsx — ws order.* → status IN_DELIVERY / courierName',
  delivered: 'OrderStatusPage.tsx — ws order.* → status DELIVERED',
};

/**
 * Pure builder: normalize a typed event into its `{ event, properties }`
 * envelope. Transport-agnostic — the taxonomy is exercised and tested without a
 * PostHog dependency. Swap the call site's consumer for `posthog.capture` once a
 * key is configured.
 */
export function buildEvent<E extends FunnelEvent>(
  event: E,
  properties: FunnelPayloads[E],
): { event: E; properties: FunnelPayloads[E] } {
  return { event, properties };
}
