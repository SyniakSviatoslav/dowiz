// Aggregator-presence check — is a venue already on Wolt in a given city? One city-list fetch, then robust
// LOCAL name matching. "On an aggregator" = already paying ~30% commission = the sharpest cost-pressure
// signal (directly receptive to a 0%-commission storefront). Public discovery data; no personal data.
//
// Matching is TOKEN-SET based (not substring) to avoid false hits like "ama" → "pastamania". We drop generic
// words (restaurant/bar/pizzeria/durrës/…) and require the venue's DISTINCTIVE tokens to be covered.

const GENERIC = new Set(['restaurant', 'restorant', 'ristorante', 'bar', 'cafe', 'kafe', 'pizzeria', 'pizzeri', 'piceri', 'pizza', 'the', 'and', 'of', 'by', 'durres', 'durrës', 'shqiptare', 'traditional', 'tradicionale', 'gatime', 'food', 'fast', 'grill', 'lounge', 'co', 'shpk']);

function toks(name) {
  return (name || '')
    .toLowerCase()
    .normalize('NFD').replace(/[̀-ͯ]/g, '') // strip diacritics
    .replace(/[^a-z0-9\s]/g, ' ')
    .split(/\s+/)
    .filter((t) => t.length >= 3 && !GENERIC.has(t));
}

/** Fetch all Wolt venue names for a city centre (lat,lon). Returns [] on any failure (graceful). */
export async function fetchWoltVenues(lat, lon) {
  try {
    const res = await fetch(`https://restaurant-api.wolt.com/v1/pages/restaurants?lat=${lat}&lon=${lon}`, {
      headers: { 'user-agent': 'Mozilla/5.0', 'app-language': 'en' },
    });
    if (!res.ok) return [];
    const j = await res.json();
    const names = [];
    for (const sec of j.sections || []) for (const it of sec.items || []) if (it.venue?.name) names.push(it.venue.name);
    return names;
  } catch { return []; }
}

/** True + matched name if `name`'s distinctive tokens are covered by some venue in `woltNames`.
 *  Guards against common-word false hits (e.g. "trattoria", "shije") by requiring at least one shared token
 *  that is RARE across the city list (document frequency ≤ 2 = a real brand token, not a descriptor). */
export function matchOnWolt(name, woltNames) {
  const want = toks(name);
  if (!want.length || !woltNames.length) return { on: false, match: null };
  const df = new Map();
  for (const wn of woltNames) for (const t of new Set(toks(wn))) df.set(t, (df.get(t) || 0) + 1);
  for (const wn of woltNames) {
    const have = new Set(toks(wn));
    if (!have.size) continue;
    const shared = want.filter((t) => have.has(t));
    const hasRare = shared.some((t) => (df.get(t) || 0) <= 2); // a distinctive brand token, not a descriptor
    const enough = want.length === 1 ? shared.length === 1 : (shared.length >= 2 || shared.length / want.length >= 0.6);
    if (enough && hasRare) return { on: true, match: wn };
  }
  return { on: false, match: null };
}

/** Convenience: fetch + check one venue. */
export async function checkWolt(name, lat, lon) {
  const venues = await fetchWoltVenues(lat, lon);
  if (!venues.length) return { on: null, match: null, listSize: 0 }; // list unavailable → inconclusive
  const r = matchOnWolt(name, venues);
  return { ...r, listSize: venues.length };
}
