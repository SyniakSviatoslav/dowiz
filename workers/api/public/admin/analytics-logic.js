// The analytics screen's arithmetic, PURE (W-HIST P2c, P3): no DOM, no
// clock, no network, so node tests it (`analytics-logic.test.mjs`). Every
// number on the screen is the hub's; this file only words, shapes and checks.
//
// ASCII QUOTES ONLY in this file: a typographic quote once took down the
// whole console. No apostrophes inside strings either.

/// The windows the hub answers, in days (`fold::window`).
export const WINDOWS = [7, 30, 90, 365];
/// The longest period one read folds (`history::MAX_SPAN`).
export const MAX_SPAN = 366;
/// The kitchen window, which the menu matrix is read over (`kitchen::MAX_DAYS`).
export const KITCHEN_MAX = 62;

/// A change in per mille as a signed percent with one decimal: 1500 -> "+150.0%".
/// `null` (nothing to compare with) is an empty string, never "0%".
export function pct(pm){
  if (pm == null || !Number.isFinite(pm)) return '';
  const a = Math.abs(Math.trunc(pm));
  return (pm > 0 ? '+' : pm < 0 ? '-' : '') + Math.trunc(a / 10) + '.' + (a % 10) + '%';
}

/// The tone of a change: up is good for money and orders.
export const tone = d => (d > 0 ? 'ok' : d < 0 ? 'bad' : '');

/// `{name}` placeholders filled from `params`; an unknown one stays visible.
export function fill(template, params = {}){
  return String(template || '').replace(/\{(\w+)\}/g, (m, k) => (params[k] == null ? m : String(params[k])));
}

/// A custom period, checked as the hub checks it: two dates, in order, at most
/// MAX_SPAN days. Returns null when fine, else the reason key.
/// A `YYYY-MM-DD` calendar day as a UTC instant at hour `h` -- arithmetic on
/// the date's own numbers, never `Date.parse` (venue-clock gate rule b).
export const utcDayMs = (s, h = 0) => { const [y, m, d] = String(s).split('-').map(Number); return Date.UTC(y, m - 1, d, h); };

export function badRange(from, to){
  const re = /^\d{4}-\d{2}-\d{2}$/;
  if (!re.test(from || '') || !re.test(to || '')) return 'anBadDate';
  const a = utcDayMs(from), b = utcDayMs(to);
  if (!Number.isFinite(a) || !Number.isFinite(b)) return 'anBadDate';
  if (a > b) return 'anFromAfterTo';
  if ((b - a) / 86400000 + 1 > MAX_SPAN) return 'anTooLong';
  return null;
}

/// The query for a period: `{ days }` or `{ from, to }`, always for the v2 shape.
export function query(p){
  return 'v=2&' + (p.from ? `from=${encodeURIComponent(p.from)}&to=${encodeURIComponent(p.to)}` : `days=${Number(p.days) || 7}`);
}

/// The kitchen read for the matrix: the same period, clipped to its last
/// KITCHEN_MAX days (the kitchen window), as `{ query, clipped }`.
export function kitchenQuery(p, a){
  const days = Number(a && a.days) || Number(p.days) || 7;
  const to = a && a.to;
  if (!to) return { query: `v=2&days=${Math.min(days, KITCHEN_MAX)}`, clipped: days > KITCHEN_MAX };
  return { query: `v=2&to=${encodeURIComponent(to)}&days=${Math.min(days, KITCHEN_MAX)}`, clipped: days > KITCHEN_MAX };
}

/// An SVG polyline's points for a series, `w` x `h`, the max at the top.
export function spark(series, w = 80, h = 20){
  const s = (series || []).map(n => Number(n) || 0);
  if (!s.length) return '';
  const max = Math.max(1, ...s), step = s.length > 1 ? w / (s.length - 1) : 0;
  return s.map((n, i) => `${Math.round(i * step * 10) / 10},${Math.round((h - (n * h) / max) * 10) / 10}`).join(' ');
}

/// A heat cell's level, 0 (none) to 4 (the busiest cell of the grid).
export function heat(n, max){
  if (!n || !max) return 0;
  return Math.min(4, Math.max(1, Math.ceil((4 * n) / max)));
}

/// The matrix's dishes grouped by quadrant, unknown cost last.
export const QUADRANTS = ['star', 'plowhorse', 'puzzle', 'dog'];
export function byQuadrant(menu){
  const out = { star: [], plowhorse: [], puzzle: [], dog: [], unknown: [] };
  for (const d of (menu && menu.dishes) || []) (out[d.quadrant] || out.unknown).push(d);
  return out;
}

/// The one sentence a dish gets: the template key and its parameters. The
/// words are `mm<Action>Do` in four languages; nothing is generated.
export function advice(d, money = n => String(n)){
  const key = { star: 'mmStarDo', plowhorse: 'mmPlowhorseDo', puzzle: 'mmPuzzleDo', dog: 'mmDogDo' }[d.quadrant] || 'mmUnknownDo';
  const c = d.costliest || {};
  return { key, params: { raise: money(d.raiseBy || 0), line: c.name || '-', cost: money(c.cost || 0) } };
}

/// Which records of a trace a filter keeps: a dish, an hour, or all.
export function keep(records, f = {}){
  return (records || []).filter(r => (f.dish == null || (r.items || []).some(i => i.id === f.dish)) && (f.hour == null || r.hour === Number(f.hour)));
}

/// The days of the pane as CSV (day, orders, revenue, from the live log, from
/// the archive). Money stays in minor units, as the hub sends it.
export function csv(a){
  const rows = [['day', 'orders', 'revenue_minor', 'hot_orders', 'archived_orders']];
  for (const d of (a && a.byDay) || []) rows.push([d.day, d.orders, d.revenue, d.hot, d.archived]);
  return rows.map(r => r.join(',')).join('\n');
}
