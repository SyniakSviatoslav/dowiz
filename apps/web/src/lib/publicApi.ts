// Public (unauthenticated) storefront endpoints that deliberately BYPASS apiClient:
// they are served at the root (no /api prefix) and must never carry auth headers,
// token refresh, or the owner-session 401 redirect. Deduplicates the venue-info
// fetch that was hand-rolled in ClientLayout / CheckoutPage / MenuPage /
// OrderStatusPage / MenuManagerPage (review finding H1).

// Response shape of GET /public/locations/:slug/info — fields typed from how the
// call sites actually consume them (the server may send more; extras are ignored).
export interface VenueInfo {
  id?: string;
  name?: string | null;
  address?: string | null;
  phone?: string | null;
  currency_code?: string | null;
  deliveryFeeFlat?: number | null;
  freeDeliveryThreshold?: number | null;
  minOrderValue?: number | null;
  taxRate?: number | null;
  priceIncludesTax?: boolean | null;
  hasDistanceTiers?: boolean | null;
  lat?: number | null;
  lng?: number | null;
  isOpen?: boolean;
  status?: 'open' | 'closed' | 'busy';
  closesAt?: string | null;
  weeklyHours?: Array<{ day: string; isOpen: boolean; open: string | null; close: string | null }> | null;
  googlePlaceId?: string | null;
  googleRating?: number | null;
  googleReviewCount?: number | null;
  googleMapsUrl?: string | null;
  socialInstagram?: string | null;
  socialFacebook?: string | null;
}

// Returns the parsed venue info, or throws (`info <status>` on a non-2xx, or the
// network/parse error). Callers keep their own catch semantics: CheckoutPage
// surfaces the failure (locationLoadFailed); the other call sites swallow it
// best-effort — exactly as their hand-rolled versions did (their old
// null-on-!ok paths were no-ops, so throwing into `.catch(() => {})` is
// observably identical).
export async function fetchVenueInfo(slug: string, opts?: { signal?: AbortSignal }): Promise<VenueInfo> {
  const res = await fetch(`/public/locations/${slug}/info`, { signal: opts?.signal });
  if (!res.ok) throw new Error(`info ${res.status}`);
  return res.json() as Promise<VenueInfo>;
}
