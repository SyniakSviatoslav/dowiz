// What every W-WIRE screen shares: the venue's slug and query, a sheet with a
// heading, the one way to say a failure. Built on /admin/core.js and
// /admin/parts.js (the /lib/ui design system); no styles of its own.
//
// ASCII QUOTES ONLY as delimiters (DOWIZ-COMMON-RULES rule 11).
import { $, S, store, sheet, toast, retranslate } from '/admin/core.js';
import '/admin/wire-i18n.js';

/// The venue's storefront slug: the record's own, else the host's first label
/// on a venue subdomain, else the location id (app.js uses the same rule).
export function slug(){
  if (S.venue && S.venue.slug) return S.venue.slug;
  const h = location.hostname.split('.');
  return h.length >= 3 ? h[0] : store.loc;
}

/// The venue's zone (an IANA name): every day these screens show or send is
/// the venue's day, never the phone's (`venue-clock` gate).
export const tz = () => (S.venue && S.venue.tz) || 'Europe/Tirane';

/// `?location_id=` for an owner route that authorises the named venue.
export const q = (sep = '?') => `${sep}location_id=${encodeURIComponent(store.loc || '')}`;

/// A public-venue path (`/public/locations/<slug>/...`) for `api()`.
export const venuePath = rest => `/public/locations/${encodeURIComponent(slug())}${rest}`;

export const fail = e => toast(String((e && e.message) || e));

/// Open the one sheet with a heading and a host; answers the host element.
export function open(titleKey, hintKey, name){
  sheet(`<p class="eyebrow" data-t="w_more"></p><h2 data-t="${titleKey}"></h2>${hintKey ? `<p class="muted small" data-t="${hintKey}"></p>` : ''}<div id="wHost"></div>`, { name });
  return $('#wHost');
}

/// Re-translate what a screen just drew.
export const paint = host => retranslate(host);
