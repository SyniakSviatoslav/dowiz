// PURE. The guest's taste and menu behaviour, scored ON THE PHONE (W-MR0 row MR7, 2026-10-04).
//
// OPERATOR, verbatim: "смак і поведінку гостя треба оцінювати ... на пристрої і сервері"; DECISIONS.md
// D0 amendment of 2026-10-04. This file is the device half: what the guest ordered and how they
// browsed, folded into a taste vector and a guess at what they are doing now, used to put a short
// "For you" strip above the menu. Its rules, each one a test in taste.test.mjs:
//   * ON BY DEFAULT (operator ruling 2026-10-04, legitimate interest; store/taste-device.js keeps
//     the switch): the guest turns it off in one tap, nothing is recorded while off, and Forget
//     wipes everything at once.
//   * DAY GRANULARITY: every stored time is a day number; the hour of a visit is kept only as a
//     24-slot count, never with its day.
//   * IT ADDS, IT NEVER HIDES: the strip is drawn above a menu left exactly as it was. An inferred
//     allergy may only move a dish down (FIC 1169/2011: information before purchase); only the
//     guest's own allergen choice hides anything (store/avoid.js).
//   * IT NEVER TOUCHES MONEY: no field here is a cost or a reduction, and tools/gates/no-scoring.sh
//     refuses this file the day one is named.
//   * THE SERVER SEES AT MOST `syncVector` -- aggregated weights, no events -- and never once the
//     guest objected (`notObjected()`, store/taste-device.js; tools/gates/no-tracking.sh rule 4).
//   * THE VENUE'S OWN PROFILE of this guest (recognised by the order link they hold, never by the
//     device) may be passed back in as `prior`: it adds to the weights, like the phone's own.

import { senseOf, vectorOf, cosine, topScaled, monthOf, MOODS, because } from './sense.js';

export const VERSION = 1;
/// Days for a signal to count half (the spec's 60-day half-life).
export const HALF_LIFE_DAYS = 60;
/// A day older than this is dropped on the next write: a year of taste is plenty.
export const KEEP_DAYS = 365;
/// The strip: at most this many dishes, of which at most `AGAIN_MAX` are "Again?".
export const STRIP_MAX = 6;
export const AGAIN_MAX = 2;
/// Weights of one event of each kind (an order of one portion is 1).
export const W = { order: 1, open: 0.2, dwellPerMin: 0.3, add: 0.5, drop: -0.4, seen: 0.05 };
/// A dish sheet held open longer than this counts as this long (a phone left on a table).
export const DWELL_CAP_S = 120;
/// What the server may receive: this many tags and categories, integer weights 0..SYNC_SCALE.
export const SYNC_TOP = 12;
export const SYNC_SCALE = 1000;
const DAY_MS = 86_400_000;

/// The local day number of an instant (`tzOffsetMin` = Date#getTimezoneOffset of that instant).
export const dayOf = (ms, tzOffsetMin = 0) => Math.floor((ms - tzOffsetMin * 60_000) / DAY_MS);

/// A phone, a tablet or a desktop: the width the page has and whether the pointer is a finger.
export function deviceClass(width, coarse){
  if (coarse && width < 600) return 'phone';
  if (coarse || width < 1024) return 'tablet';
  return 'desktop';
}

/// Where the guest came from, once, at the first visit: a referrer's HOST and the three utm_ words.
export function firstVisit(referrer, search){
  let ref = null;
  try { ref = referrer ? new URL(referrer).host || null : null; } catch { ref = null; }
  const q = new URLSearchParams(search || '');
  const utm = {};
  for (const k of ['source', 'medium', 'campaign']) { const v = q.get('utm_' + k); if (v) utm[k] = v.slice(0, 64); }
  return { ref, utm: Object.keys(utm).length ? utm : null };
}

export function empty({ day, device = null, first = null } = {}){
  return { v: VERSION, since: day, device, first, dishes: {}, cats: {}, hours: Array(24).fill(0) };
}

/// Add `n` to the [day, n] list `list`, merging today's entry.
function bump(list, day, n){
  const out = (list || []).filter(([d]) => d > day - KEEP_DAYS);
  const last = out[out.length - 1];
  if (last && last[0] === day) last[1] += n; else out.push([day, n]);
  return out;
}

/// One event folded in. Unknown kinds and malformed events change nothing.
///   { kind: 'order', items: [{ id, qty }] } | { kind: 'open'|'add'|'drop', id }
///   { kind: 'dwell', id, sec } | { kind: 'seen', cat } | { kind: 'visit', hour }
export function record(profile, ev, day){
  if (!profile || profile.v !== VERSION || !ev || !Number.isInteger(day)) return profile;
  const p = structuredClone(profile);
  const dish = id => (p.dishes[id] ||= {});
  switch (ev.kind) {
    case 'order':
      for (const it of ev.items || []) if (it?.id && it.qty > 0) dish(it.id).order = bump(dish(it.id).order, day, it.qty | 0);
      // W-SENSE: the moment AT THE VENUE it was ordered in (`band:evening`, `wx:rain`), per dish.
      for (const c of Array.isArray(ev.ctx) ? ev.ctx.filter(c => typeof c === 'string' && c.length <= 24).slice(0, 4) : []) {
        p.ctx ||= {};
        for (const it of ev.items || []) if (it?.id && it.qty > 0) (p.ctx[c] ||= {})[it.id] = bump(p.ctx[c][it.id], day, it.qty | 0);
      }
      break;
    case 'open': case 'add': case 'drop':
      if (ev.id) dish(ev.id)[ev.kind] = bump(dish(ev.id)[ev.kind], day, 1);
      break;
    case 'dwell':
      if (ev.id && ev.sec > 0) dish(ev.id).dwell = bump(dish(ev.id).dwell, day, Math.min(DWELL_CAP_S, Math.round(ev.sec)));
      break;
    case 'seen':
      if (ev.cat) (p.cats[ev.cat] ||= {}).seen = bump(p.cats[ev.cat]?.seen, day, 1);
      break;
    case 'visit':
      if (Number.isInteger(ev.hour) && ev.hour >= 0 && ev.hour < 24) p.hours[ev.hour] += 1;
      break;
    default: return profile;
  }
  return p;
}

/// 1 today, 1/2 after HALF_LIFE_DAYS, never more than 1.
export const decay = (from, day) => Math.pow(0.5, Math.max(0, day - from) / HALF_LIFE_DAYS);
const sum = (list, day) => (list || []).reduce((a, [d, n]) => a + n * decay(d, day), 0);

/// Weights per dish, per tag and per category, from the profile and today's menu.
export function weights(profile, products, day){
  const dish = new Map(), tag = new Map(), cat = new Map();
  if (!profile || profile.v !== VERSION) return { dish, tag, cat };
  for (const [id, e] of Object.entries(profile.dishes || {})) {
    const w = W.order * sum(e.order, day) + W.open * sum(e.open, day) + W.dwellPerMin * sum(e.dwell, day) / 60
            + W.add * sum(e.add, day) + W.drop * sum(e.drop, day);
    if (w) dish.set(id, w);
  }
  for (const p of products || []) {
    const w = dish.get(p.id) || 0;
    if (!w) continue;
    for (const t of Array.isArray(p.tags) ? p.tags : []) tag.set(t, (tag.get(t) || 0) + w);
    if (p.categoryId) cat.set(p.categoryId, (cat.get(p.categoryId) || 0) + w);
  }
  for (const [c, e] of Object.entries(profile.cats || {})) cat.set(c, (cat.get(c) || 0) + W.seen * sum(e.seen, day));
  return { dish, tag, cat };
}

/// A dish's score: its own weight, its tags', half its category's.
export function score(p, w){
  let s = w.dish.get(p.id) || 0;
  for (const t of Array.isArray(p.tags) ? p.tags : []) s += w.tag.get(t) || 0;
  return s + 0.5 * (w.cat.get(p.categoryId) || 0);
}

/// What the guest seems to be doing NOW, from this visit only, with the rule that decided it.
export function intent({ cartLines = 0, cartQty = 0, adds = 0, opens = 0 } = {}){
  if (cartQty >= 4 || cartLines >= 3) return { kind: 'group', why: 'cart' };
  if (cartQty > 0 || adds > 0) return { kind: 'ready', why: 'cart' };
  return { kind: 'browsing', why: opens > 0 ? 'opens' : 'none' };
}

/// The "For you" strip: up to AGAIN_MAX dishes ordered before ("again"), then the best-scored rest
/// ("taste"), only dishes on sale, never more than STRIP_MAX. `avoidGuess` (an INFERRED allergy)
/// only moves a dish to the end of the strip -- it never removes one.
/// The venue's view of this guest (`GET /api/order/:id/taste` -> `taste`), as weights added to the
/// phone's: the server keeps `PRIOR_UNIT` per portion, the phone 1. Unknown shapes add nothing.
export const PRIOR_UNIT = 1000;
export function withPrior(w, prior){
  for (const [rows, m] of [[prior?.tags, w.tag], [prior?.cats, w.cat]]) {
    for (const r of Array.isArray(rows) ? rows : []) {
      const k = r && typeof r.key === 'string' ? r.key : null, x = Number(r?.w);
      if (k && Number.isFinite(x) && x > 0) m.set(k, (m.get(k) || 0) + x / PRIOR_UNIT);
    }
  }
  return w;
}
export function strip(products, profile, day, { avoidGuess = [], prior = null, ctx = null, mood = null } = {}){
  const on = (products || []).filter(p => p && p.available !== false);
  const w = withPrior(weights(profile, on, day), prior);
  const guest = senseVec(profile, on, day);
  for (const r of Array.isArray(prior?.sense) ? prior.sense : []) { const x = Number(r?.w); if (typeof r?.key === 'string' && x > 0) guest[r.key] = (guest[r.key] || 0) + x / PRIOR_UNIT; }
  const moment = ctx ? senseVec(profile, on, day, { ctx }) : null;
  const total = p => score(p, w) + senseScore(p, { guest, moment, mood });
  const ordered = id => sum(profile?.dishes?.[id]?.order, day);
  const guessed = p => Array.isArray(p.allergens) && p.allergens.some(c => avoidGuess.includes(c));
  const again = on.filter(p => ordered(p.id) > 0).sort((a, b) => ordered(b.id) - ordered(a.id) || String(a.id).localeCompare(String(b.id)))
    .slice(0, AGAIN_MAX).map(p => ({ id: p.id, why: 'again' }));
  const taken = new Set(again.map(x => x.id));
  const rest = on.filter(p => !taken.has(p.id)).map(p => ({ p, s: total(p) })).filter(x => x.s > 0)
    .sort((a, b) => b.s - a.s || String(a.p.id).localeCompare(String(b.p.id)))
    .map(x => ({ id: x.p.id, why: 'taste', guessed: guessed(x.p) }));
  const ranked = [...rest.filter(x => !x.guessed), ...rest.filter(x => x.guessed)];
  return [...again, ...ranked].slice(0, STRIP_MAX);
}

// ── W-SENSE: the guest on the dish's taste, texture and aroma ──────────────────
/// How much the dish's own axes count beside the tags (a never-ordered dish is predicted from them),
/// the current context's, and the session's mood.
export const SENSE_W = 2, CTX_W = 1, MOOD_W = 1.5;
export const MONTHS_KEEP = 13, SNAP_TOP = 12;

/// The guest's vector over the menu's declared axes: each dish's weight x its vector.
export function senseVec(profile, products, day, { ctx = null } = {}){
  const w = weights(profile, products, day).dish;
  if (ctx) {
    w.clear();
    for (const c of ctx) for (const [id, list] of Object.entries(profile?.ctx?.[c] || {})) w.set(id, (w.get(id) || 0) + sum(list, day));
  }
  const vec = {};
  for (const p of products || []) {
    const x = w.get(p.id); if (!(x > 0)) continue;
    for (const [k, v] of Object.entries(vectorOf(senseOf(p)))) vec[k] = (vec[k] || 0) + x * v / 1000;
  }
  return vec;
}

/// What the axes add to a dish's score: its match with the guest, with the moment, with the mood.
export function senseScore(p, { guest = null, moment = null, mood = null } = {}){
  const v = vectorOf(senseOf(p));
  if (!Object.keys(v).length) return 0;
  return SENSE_W * Math.max(0, cosine(guest, v)) + CTX_W * Math.max(0, cosine(moment, v)) + MOOD_W * cosine(mood ? MOODS[mood] : null, v);
}

/// The month's snapshot of the vector, kept beside the profile (the newest MONTHS_KEEP).
export function snapshot(profile, products, day){
  if (!profile || profile.v !== VERSION) return profile;
  const vec = senseVec(profile, products, day);
  if (!Object.keys(vec).length) return profile;
  const p = structuredClone(profile);
  p.months = { ...(p.months || {}), [monthOf(day)]: topScaled(vec, SNAP_TOP) };
  for (const m of Object.keys(p.months).sort().slice(0, -MONTHS_KEEP)) delete p.months[m];
  return p;
}
export { because };

/// "What this phone remembers", as plain counts the guest can read.
export function remembered(profile){
  if (!profile || profile.v !== VERSION) return null;
  const d = Object.values(profile.dishes || {});
  const n = k => d.filter(e => (e[k] || []).length).length;
  const total = k => d.reduce((a, e) => a + (e[k] || []).reduce((x, [, v]) => x + v, 0), 0);
  return {
    since: profile.since, device: profile.device, ref: profile.first?.ref || null, utm: profile.first?.utm || null,
    ordered: n('order'), portions: total('order'), opened: n('open'), added: total('add'), removed: total('drop'),
    dwellMin: Math.round(total('dwell') / 60), categoriesSeen: Object.keys(profile.cats || {}).length,
    visits: profile.hours.reduce((a, b) => a + b, 0),
  };
}

/// The ONE thing that may leave the phone, and only with consent: the strongest tags and
/// categories as integers 0..SYNC_SCALE. No dish ids, no events, no days, no referrer, no device.
export function syncVector(profile, products, day){
  const w = weights(profile, products, day);
  const top = m => {
    const rows = [...m.entries()].filter(([, v]) => v > 0).sort((a, b) => b[1] - a[1] || a[0].localeCompare(b[0])).slice(0, SYNC_TOP);
    const max = rows.length ? rows[0][1] : 1;
    return Object.fromEntries(rows.map(([k, v]) => [String(k).slice(0, 32), Math.round(SYNC_SCALE * v / max)]));
  };
  return { v: VERSION, tags: top(w.tag), cats: top(w.cat), sense: topScaled(senseVec(profile, products, day), 27) };
}
