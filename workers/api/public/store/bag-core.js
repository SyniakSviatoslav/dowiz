// PURE: the bag card's landing, first-party only (W-QR). No imports, no DOM:
// the storage is passed in, which is what lets bag.test.mjs prove where the
// landing lives and where it goes.
//
// THE RULE: `?src=bag[&c=<campaign>]` is kept in sessionStorage -- this tab,
// this visit -- and leaves the device exactly once, in the order body at
// checkout. Never a cookie, never localStorage, never a fingerprint, and no
// request is made because of it before the guest places an order.
//
// ASCII QUOTES ONLY in this file.

export const KEY = 'dw_src';
const CAMPAIGN = /^[a-z0-9-]{1,24}$/;

/// `{src:'bag', c}` from a query string, or null. A campaign the hub would
/// refuse is dropped rather than carried into an order the hub then refuses.
export function parseLanding(search){
  let p;
  try { p = new URLSearchParams(search || ''); } catch { return null; }
  if (p.get('src') !== 'bag') return null;
  const c = p.get('c');
  return { src: 'bag', c: c && CAMPAIGN.test(c) ? c : null };
}

/// Remember a landing for this visit (sessionStorage only), or read the one remembered.
export function capture(search, session){
  const fromUrl = parseLanding(search);
  if (fromUrl) {
    try { session.setItem(KEY, JSON.stringify(fromUrl)); } catch {}
    return fromUrl;
  }
  try {
    const v = JSON.parse(session.getItem(KEY) || 'null');
    return v && v.src === 'bag' ? { src: 'bag', c: v.c && CAMPAIGN.test(v.c) ? v.c : null } : null;
  } catch { return null; }
}

/// The order body's member: `{src:{src:'bag', c?}}`, or nothing.
export function bodyFor(landing){
  if (!landing || landing.src !== 'bag') return {};
  return { src: landing.c ? { src: 'bag', c: landing.c } : { src: 'bag' } };
}

/// After the order is placed, the landing has done its job.
export function forget(session){
  try { session.removeItem(KEY); } catch {}
}

/// The word the hub answered with, as a key the storefront translates.
export function outcome(order){
  if (order && order.welcome) return 'bg_ok';
  const why = order && order.welcome_refused;
  return why ? 'bg_r_' + why : null;
}
