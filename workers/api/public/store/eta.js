// The delivery estimate, asked of the kernel and never worked out here.
//
// `POST /api/public/locations/:slug/eta` runs `dowiz_kernel::eta` over the
// basket's own cooking times, the venue's kitchen profile and the distance to
// the customer. It reads no clock: the same basket from the same place gives
// the same answer, which is what makes it testable and what makes it honest --
// a range, never a single number, never zero.
//
// The answer is cached against the basket it was asked for and thrown away
// with the basket (state.js drops `lastEta` in `saveCart`), so a stale number
// cannot outlive the order it described.

import { API, SLUG, state, cartLines } from '/store/state.js';

let inflight = null;

/// `{ min, max, text }` in minutes, or null when the kernel refuses (an empty
/// basket has no estimate) or the network is away. A null is rendered as
/// nothing, never as "0 min".
export async function quoteEta({ pickup = state.how === 'pickup' || state.how === 'table', pin = state.pin, geo = state.geo } = {}){
  const lines = cartLines();
  if (!lines.length) return null;
  const sig = JSON.stringify([lines.map(l => [l.p.id, l.q]), pickup, pin, geo]);
  if (state.lastEta && state.lastEta.sig === sig) return state.lastEta;
  if (inflight && inflight.sig === sig) return inflight.p;
  const body = {
    items: lines.map(l => ({ id: l.p.id, quantity: l.q,
      ...(Number.isInteger(l.p.cookingMin) ? { cookingMin: l.p.cookingMin } : {}) })),
    pickup,
  };
  const lat = pin?.lat ?? (geo ? geo.lat_udeg / 1e6 : null);
  const lng = pin?.lng ?? (geo ? geo.lon_udeg / 1e6 : null);
  if (Number.isFinite(lat) && Number.isFinite(lng)) {
    body.latUdeg = Math.round(lat * 1e6);
    body.lonUdeg = Math.round(lng * 1e6);
  }
  const p = (async () => {
    try {
      const r = await fetch(`${API}/public/locations/${encodeURIComponent(SLUG)}/eta`, {
        method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) });
      if (!r.ok) return null;
      const d = await r.json();
      const min = Number(d.minMin ?? d.min ?? d.lowMin ?? d.low);
      const max = Number(d.maxMin ?? d.max ?? d.highMin ?? d.high);
      const text = typeof d.range === 'string' ? d.range
        : (Number.isFinite(min) && Number.isFinite(max)) ? `${min}–${max}` : null;
      if (!text) return null;
      const out = { sig, min, max, text, pickup };
      state.lastEta = out;
      return out;
    } catch { return null; }
    finally { inflight = null; }
  })();
  inflight = { sig, p };
  return p;
}

/// The venue's published range, for the hero before there is a basket. It is
/// the venue's own claim ("30-45") and is shown as such.
export const publishedEta = () => {
  const s = state.loc?.deliveryEta;
  return s && /\d/.test(String(s)) ? String(s).replace('-', '–') : null;
};
