// PURE. What a dish tastes, feels and smells like, as the kitchen declared it (W-SENSE, 2026-10-04).
//
// The vocabulary is `dowiz_hub::sense` (crates/dowiz-hub/src/sense.rs): six taste axes 0..5, nine
// textures and twelve aromas at 1..3. Its rules here, each one a test in sense.test.mjs:
//   * NOTHING IS DRAWN THAT NOBODY DECLARED: an absent axis is absent (no fake zeros). A taste axis
//     at 0 IS a claim ("not spicy at all") and is drawn as an empty bar with the figure 0.
//   * THE OLD FIELD STILL READS: a dish with no `sense` takes its taste from the old `taste` map
//     (levels 1..3 at x5/3, `richness` has no axis here), the server's own rule.
//   * EVERY BAR HAS ITS NUMBER AS TEXT, for a screen reader and for a reader who does not read bars.
//   * MOOD IS THIS PAGE ONLY: the four mood weightings are constants; the chosen one is held by the
//     caller in memory and never stored or sent (no-tracking.sh rule 6).
// No DOM here: markup is returned as strings by store/sense-ui.js.

export const TASTE = ['sweet', 'sour', 'salty', 'bitter', 'umami', 'spicy'];
export const TEXTURE = ['crispy', 'crunchy', 'tender', 'creamy', 'chewy', 'soft', 'juicy', 'silky', 'flaky'];
export const AROMA = ['smoky', 'citrus', 'herbal', 'floral', 'nutty', 'toasty', 'marine', 'fermented', 'fruity', 'earthy', 'buttery', 'spice-warm'];
export const TASTE_MAX = 5, TAG_MAX = 3, SCALE = 1000;
const DIMS = [['taste', 't', TASTE, 0, TASTE_MAX], ['texture', 'x', TEXTURE, 1, TAG_MAX], ['aroma', 'a', AROMA, 1, TAG_MAX]];

const int = v => (typeof v === 'number' && Number.isInteger(v) ? v : null);
export const legacyLevel = l => Math.floor((Math.max(0, Math.min(3, l)) * 10 + 3) / 6);

/// The dish's declared profile `{taste, texture, aroma}`, or null when it declares nothing.
export function senseOf(p){
  const out = { taste: {}, texture: {}, aroma: {} };
  const s = p && typeof p.sense === 'object' && p.sense ? p.sense : null;
  if (s) {
    for (const [dim, , words, lo, hi] of DIMS) {
      for (const [id, v] of Object.entries(s[dim] || {})) {
        const n = int(v);
        if (words.includes(id) && n !== null && n >= lo && n <= hi) out[dim][id] = n;
      }
    }
  } else if (p && p.taste && typeof p.taste === 'object') {
    for (const [id, v] of Object.entries(p.taste)) {
      const n = int(v);
      if (TASTE.includes(id) && n !== null && n >= 1 && n <= 3) out.taste[id] = legacyLevel(n);
    }
  }
  return DIMS.some(([dim]) => Object.keys(out[dim]).length) ? out : null;
}

/// `{ 't:spicy': 800, 'x:crispy': 1000, ... }`, per mille of each scale; a taste at 0 weighs nothing.
export function vectorOf(s){
  const v = {};
  if (!s) return v;
  for (const [dim, pre, , , hi] of DIMS) {
    for (const [id, n] of Object.entries(s[dim] || {})) if (n > 0) v[`${pre}:${id}`] = Math.floor(n * SCALE / hi);
  }
  return v;
}

/// Cosine of two sparse vectors (objects of numbers); 0 when either is empty.
export function cosine(a, b){
  let dot = 0, na = 0, nb = 0;
  for (const [k, x] of Object.entries(a || {})) { na += x * x; if (b && k in b) dot += x * b[k]; }
  for (const x of Object.values(b || {})) nb += x * x;
  return na && nb ? dot / Math.sqrt(na * nb) : 0;
}

/// The card's data attribute: `t:spicy:4 x:crispy:3 a:smoky:2`, what a filter reads offline.
export function cardAttr(s){
  if (!s) return '';
  return DIMS.flatMap(([dim, pre]) => Object.entries(s[dim]).map(([id, n]) => `${pre}:${id}:${n}`)).join(' ');
}
const parseAttr = attr => Object.fromEntries(String(attr || '').split(' ').filter(Boolean).map(x => { const i = x.lastIndexOf(':'); return [x.slice(0, i), Number(x.slice(i + 1))]; }));

/// The filter chips a menu can offer, from what it actually declares. `spicy` = spicy 3 or more;
/// `not-spicy` = spicy DECLARED at 0 or 1 (an undeclared dish is not "not spicy"); a texture or
/// aroma = the tag is there. Preferred ones first, then by how many dishes carry them; at most `max`.
export const PREFERRED = ['spicy', 'not-spicy', 'x:crispy', 'x:creamy', 'a:smoky'];
export function filterChips(products, max = 10){
  const n = new Map();
  const bump = k => n.set(k, (n.get(k) || 0) + 1);
  for (const p of products || []) {
    const s = senseOf(p); if (!s) continue;
    if (s.taste.spicy >= 3) bump('spicy');
    if (s.taste.spicy !== undefined && s.taste.spicy <= 1) bump('not-spicy');
    for (const id of Object.keys(s.texture)) bump('x:' + id);
    for (const id of Object.keys(s.aroma)) bump('a:' + id);
  }
  const rank = k => { const i = PREFERRED.indexOf(k); return i < 0 ? PREFERRED.length : i; };
  return [...n.keys()].sort((a, b) => rank(a) - rank(b) || n.get(b) - n.get(a) || a.localeCompare(b)).slice(0, max);
}

/// Does a card (its `data-sense`) pass every active chip? No chip, everything passes.
export function passes(attr, active){
  if (!active || !active.size) return true;
  const m = parseAttr(attr);
  for (const f of active) {
    if (f === 'spicy' ? !(m['t:spicy'] >= 3) : f === 'not-spicy' ? !(m['t:spicy'] <= 1) : !(m[f] >= 1)) return false;
  }
  return true;
}

/// The session's mood, as axis weights (+ wanted, - avoided). Never stored, never sent.
export const MOODS = {
  quick: { 't:spicy': 600, 't:sour': 500, 'a:citrus': 800, 'x:crispy': 700, 'x:crunchy': 600, 'x:juicy': 500 },
  cosy: { 't:umami': 900, 'x:creamy': 700, 'x:tender': 700, 'a:toasty': 700, 'a:buttery': 600, 'a:spice-warm': 800, 'a:smoky': 500 },
  light: { 't:sour': 600, 'a:citrus': 800, 'a:herbal': 800, 'a:marine': 600, 'x:crunchy': 600, 'x:juicy': 600, 'x:creamy': -700, 'a:buttery': -700 },
  treat: { 't:sweet': 1000, 'x:creamy': 800, 'a:buttery': 700, 'a:nutty': 600, 'x:crispy': 500, 'a:fruity': 500 },
};
export const MOOD_IDS = Object.keys(MOODS);

/// The two keys that say most about a guest's vector: textures and aromas first ("smoky + crispy").
export function because(vec, n = 2){
  const rows = Object.entries(vec || {}).filter(([, w]) => w > 0).sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0]));
  const tags = rows.filter(([k]) => !k.startsWith('t:'));
  return (tags.length ? tags : rows).slice(0, n).map(([k]) => k);
}

/// A vector's strongest `n` keys as integers per mille of its top (what may leave the phone).
export function topScaled(vec, n){
  const rows = Object.entries(vec || {}).filter(([, w]) => w > 0).sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0])).slice(0, n);
  const top = rows.length ? rows[0][1] : 1;
  return Object.fromEntries(rows.map(([k, w]) => [k, Math.max(1, Math.round(SCALE * w / top))]));
}

/// "2026-10" for a day number (days since 1970-01-01).
export function monthOf(day){
  const d = new Date(day * 86_400_000);
  return `${d.getUTCFullYear()}-${String(d.getUTCMonth() + 1).padStart(2, '0')}`;
}
