// SEMI-FINISHED PRODUCTS, THE RULES, PURE (lane W-PF, 2026-09-29): K, the
// batch cost and the cost per kg as the editor's fields change, the typed
// lines checked, the request body, the where-used sentence, a leaf's three
// decimals. No DOM, no clock, no network: node runs every rule here
// (`prep-logic.test.mjs`). The hub's answer (`recipe/prep.rs`) wins after a
// save; this mirrors it so the person sees the number before tapping Save.
//
// INTEGERS: quantities are whole base units, money is minor units. The
// live cost uses the same per-basis prices the hub uses on read
// (`ingredients-calc.js` priceOf / costOf).
// ASCII QUOTES ONLY in this file (DOWIZ-COMMON-RULES rule 11).

import { basisOf, costOf, priceOf, PM } from './ingredients-calc.js';
import { parseMilli, wireQty, toMilli, MILLI } from './qty-adapter.js';

/// The hub's own bounds (`dowiz_hub::prep`).
export const LINES_MAX = 40, QTY_MAX = 100000, DEPTH_MAX = 6;
/// The kind a semi-finished supply record carries.
export const KIND = 'prep';
export const isPrep = s => s?.kind === KIND;

/// Grams of `qty` of `sup`: g and ml are grams, a piece weighs its
/// `weightPerUnit`, or null when it has none.
export function gramsOf(sup, qty){
  if (!sup) return null;
  const milli = toMilli(qty); if (milli == null) return null;
  if (sup.unit === 'unit') return sup.weightPerUnit != null ? Math.round(sup.weightPerUnit * milli / MILLI) : null;
  return milli / MILLI;
}

/// K per mille: yield in grams over the sum of the lines' gross in grams.
/// `lines`: [{ sup, qty }]. Null when any piece has no weight (unknown, not
/// refused) or nothing is typed yet.
export function kPm(lines, yieldQty, unit, weightPerUnit = null){
  if (!lines?.length || !(yieldQty > 0)) return null;
  let gross = 0;
  for (const l of lines) {
    const g = gramsOf(l.sup, l.qty);
    if (g == null) return null;
    gross += g;
  }
  const out = gramsOf({ unit, weightPerUnit }, yieldQty);
  if (out == null || gross <= 0) return null;
  return Math.round(out * PM / gross);
}

/// One line's cost at the price the hub reads (average, else list), or null.
export const lineCost = (sup, qty) => { const m = sup ? toMilli(qty) : null; return m == null ? null : costOf(priceOf(sup).perBasis, m, basisOf(sup.unit) * MILLI); };

/// The whole batch's cost, or null when any line has no price.
export function batchCost(lines){
  if (!lines?.length) return null;
  let sum = 0;
  for (const l of lines) {
    if (l.sup?.untracked) continue;
    const c = lineCost(l.sup, l.qty);
    if (c == null) return null;
    sum += c;
  }
  return sum;
}

/// Cost per kg / l (mass) or per piece, half up; null without a batch cost.
export function costPer(batch, yieldQty, unit){
  const y = batch == null ? null : toMilli(yieldQty);
  if (y == null || !(y > 0)) return null;
  // In milli-units, integers: 1000 for 2.5 pieces is 400 a piece, half up.
  const per = (unit === 'unit' ? 1 : 1000) * MILLI;
  return Math.floor((batch * per * 2 + y) / (2 * y));
}
/// The word for `costPer`'s basis.
export const perWord = unit => (unit === 'unit' ? 'pf_costPerPiece' : unit === 'ml' ? 'pf_costPerL' : 'pf_costPerKg');

/// The typed lines as the hub takes them, or { error: key }. `rows`:
/// [{ item, qty: "1,5 kg", unit }] -- the unit is the ITEM's. An error of `pf_qtyBad`
/// names the items of the bad lines in `bad`.
export function readLines(rows){
  const out = [], bad = [];
  for (const r of rows || []) {
    if (!r.item) continue;
    // TZ 1 (CONTRACT-QTY 1): a decimal is parsed as text into milli and sent as
    // text; a whole quantity goes as the integer it always was. The hub counts a
    // semi-finished card whole for now and says so by name (H2); never rounded here.
    const milli = parseMilli(r.qty, r.unit);
    // TZ 2026-10-16 "no ingredient lost": every bad line is named (`bad`), not just the card.
    if (milli == null || milli < 1 || milli > QTY_MAX * MILLI) { bad.push(r.item); continue; }
    if (out.some(l => l.item === r.item)) return { error: 'nom_packTwice' };
    out.push({ item: r.item, qty: wireQty(milli) });
  }
  if (bad.length) return { error: 'pf_qtyBad', bad };
  if (!out.length) return { error: 'pf_noLines' };
  if (out.length > LINES_MAX) return { error: 'nom_tooMany' };
  return { lines: out };
}

/// The yield as typed, in range, or null: a whole number of base units, or
/// the decimal TEXT for a fraction ("999.5") -- the hub stores it exactly
/// (`yieldUq`, W-QTYHUB H2). At most 3 decimals.
export function readYield(raw, unit){
  const m = parseMilli(raw, unit);
  return m != null && m >= 1 && m <= QTY_MAX * MILLI ? wireQty(m) : null;
}

/// The request body of `POST /api/owner/preps`.
export function body(state){
  const b = { id: state.id, name: state.name || state.id, unit: state.unit, category: state.category || '', lines: state.lines, yield: state.yield };
  if (state.unit === 'unit' && state.weightPerUnit != null) b.weightPerUnit = state.weightPerUnit;
  return b;
}

/// A short id from a name, as the supply form makes one.
export const idOf = name => String(name || '').trim().toLowerCase().normalize('NFD').replace(/\p{Diacritic}/gu, '').replace(/[^a-z0-9а-яіїєґ]+/gi, '-').replace(/^-|-$/g, '');

/// Where an item is used, as one line for a confirmation. `uses` is the
/// hub's `{ preps: [{id,name}], dishes: [{id,name}] }` (or a map of them, by id).
export function usesLine(uses, t){
  const all = uses?.preps ? [uses] : Object.values(uses || {});
  const preps = [...new Set(all.flatMap(u => (u.preps || []).map(p => p.name || p.id)))];
  const dishes = [...new Set(all.flatMap(u => (u.dishes || []).map(d => d.name || d.id)))];
  const parts = [];
  if (preps.length) parts.push(`${preps.length} ${t('pf_preps')}: ${preps.slice(0, 6).join(', ')}${preps.length > 6 ? ' …' : ''}`);
  if (dishes.length) parts.push(`${dishes.length} ${t('pf_dishes')}: ${dishes.slice(0, 6).join(', ')}${dishes.length > 6 ? ' …' : ''}`);
  return parts.join(' · ');
}
export const isUsed = uses => usesLine(uses, () => '').length > 0;

/// Millionths of a base unit as three decimals: 773810 -> "0.774".
export function qty3(uq){
  const milli = Math.floor((Number(uq) + 500) / 1000);
  return `${Math.floor(milli / 1000)}.${String(milli % 1000).padStart(3, '0')}`;
}

/// The picker's list: raw items and OTHER semi-finished products matching
/// `q`, not already on the card, not the card itself. Sorted by name.
export function pickable(all, q, exclude = new Set(), self = null){
  const needle = String(q || '').toLowerCase().trim();
  return (all || [])
    .filter(s => s.id !== self && !exclude.has(s.id) && (s.active == null || s.active) && (!needle || `${s.name} ${s.category || ''} ${s.id}`.toLowerCase().includes(needle)))
    .sort((a, b) => Number(isPrep(b)) - Number(isPrep(a)) || String(a.name).localeCompare(String(b.name)));
}
