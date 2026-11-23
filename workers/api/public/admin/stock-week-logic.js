// THE WEEK'S PURCHASE LIST, THE PURE PART: what each row asks for, the rows
// grouped by supplier into the text a supplier receives (the order list's
// own `orderText`), what the average stands on, and the share of sales no
// recipe draws for. Every number is the hub's (`analytics/kitchen/report/
// week.rs`); nothing here averages. No DOM: `stock-week-logic.test.mjs`.
//
// ASCII QUOTES ONLY in this file.

import { orderText } from './order-list-logic.js';
import { showMilli } from './ingredients-calc.js';

/// A quantity as the owner's screen writes it: exact, with a point, as every
/// stock screen does (`showMilli`): 2798 g is "2.798 kg", 999 g "999 g".
/// The supplier's message keeps `qtyText` (a comma, the supplier's way).
export function screenQty(q, unit){
  if ((unit === 'g' || unit === 'ml') && Math.abs(q) >= 1000) return `${showMilli(q)} ${unit === 'g' ? 'kg' : 'l'}`;
  return unit === 'g' || unit === 'ml' ? `${q} ${unit}` : String(q);
}

/// Up to whole packs of `pack` (none: as it is).
const packUp = (q, pack) => (pack > 0 ? Math.ceil(q / pack) * pack : q);

/// What a row asks to buy: the hub's `toBuy` where the stock is known, else
/// (stock not counted, or below zero) the week's need in whole packs.
export function qtyOf(r){
  if (r?.toBuy != null) return r.toBuy;
  return packUp(Math.max(0, r?.need || 0), r?.pack);
}

/// The rows with something to buy, by supplier in the order met, rows with
/// no supplier last; each line shaped as `orderText` reads it.
export function groups(week){
  const by = new Map();
  for (const r of week?.rows || []) {
    const qty = qtyOf(r);
    if (!(qty > 0)) continue;
    const key = r.supplier || null;
    if (!by.has(key)) by.set(key, []);
    by.get(key).push({ id: r.id, name: r.name, unit: r.unit, qty, pack: r.pack > 0 ? { qty: r.pack } : null });
  }
  const out = [...by].map(([supplier, lines]) => ({ supplier, lines }));
  return [...out.filter(g => g.supplier), ...out.filter(g => !g.supplier)];
}

/// The message for one group, words `w(key)` in the supplier's language.
export const supplierText = (g, venue, w) => orderText({ venue, supplier: g?.supplier, lines: g?.lines || [] }, w);

/// "2 weeks · since 2026-11-09": whole weeks when the days are, else days.
export function dataText(week, w){
  const d = week?.dataDays || 0;
  if (!(d > 0) || !week.dataFrom) return w('wp_noData');
  const span = d % 7 === 0 ? `${d / 7} ${w(d === 7 ? 'wp_week' : 'wp_weeks')}` : `${d} ${w('wp_days')}`;
  return `${span} · ${w('wp_since')} ${week.dataFrom}`;
}

/// The share of sales with no recipe, in whole per cent (half up); null
/// when nothing was sold -- no share is claimed.
export function shareText(cov){
  const pm = cov?.noRecipe?.pm;
  if (pm == null) return null;
  return `${Math.floor((pm + 5) / 10)} %`;
}
