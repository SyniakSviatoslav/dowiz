// The ingredients screens' arithmetic, PURE: no DOM, no clock, no network, so
// every rule here runs in node (`ingredients-calc.test.mjs`).
//
// INTEGERS ONLY. A shelf movement is milli-units (`milli`, thousandths of the
// base unit: 2.5 pieces is 2500); a weighed board, a ratio per mille and money
// in minor units stay whole -- the hub's own units. A typed "1,5 kg" becomes
// 1500 g here, and finer than the precision is refused rather than rounded
// behind the owner's back.
//
// ASCII QUOTES ONLY in this file.

/// Per mille: 1000 = nothing lost.
export const PM = 1000;
/// Cleaning can only lose; cooking may grow a thing up to five-fold.
export const CLEAN_MAX = PM, COOK_MAX = 5 * PM;
/// How many base units a list price is per: 100 g/ml, or 1 piece.
export const basisOf = unit => unit === 'unit' ? 1 : 100;

import { canonQty } from './qty-adapter.js';

const UNIT_SCALE = { g: 1, ml: 1, kg: 1000, l: 1000 };

/// "1500", "1 500", "1,5 kg", "0.25 l" -> whole base units, or null.
/// `unit` is the supply's base unit; kg/l scale only g/ml supplies.
export function amount(raw, unit = 'g'){
  const s = canonQty(raw);
  if (!s) return null;
  const m = /^(-?\d+(?:[.,]\d+)?)\s*(kg|g|ml|l)?$/.exec(s.replace(/(\d) (?=\d{3}\b)/g, '$1'));
  if (!m) return null;
  const scale = m[2] ? UNIT_SCALE[m[2]] : 1;
  if (m[2] && unit === 'unit') return null;
  const [whole, frac = ''] = m[1].replace(',', '.').split('.');
  // Exact decimal arithmetic on the digits: 1.5 kg is 1500, never 1499.9999.
  const digits = BigInt(whole + frac) * BigInt(scale);
  const div = 10n ** BigInt(frac.length);
  if (digits % div !== 0n) return null;
  const n = Number(digits / div);
  return Number.isSafeInteger(n) ? n : null;
}

/// MILLI-UNITS (W-QTYHUB H3, CONTRACT-QTY §1): "2,5" pieces -> 2500,
/// "0,75 kg" -> 750000, "1 500" g -> 1500000 -- what a delivery, a write-off
/// or a count keeps, exactly, as an INTEGER (thousandths of the base unit).
/// Finer than a thousandth is null (`whyNot` says 'tooFine'). Exact decimal
/// arithmetic on the digits, never a float. `amount()` above stays whole for
/// what the shelf weighs whole: a price, a prep's board, a transfer.
export function milli(raw, unit = 'g'){
  const s = canonQty(raw);
  if (!s) return null;
  const m = /^(-?\d+(?:[.,]\d+)?)\s*(kg|g|ml|l)?$/.exec(s.replace(/(\d) (?=\d{3}\b)/g, '$1'));
  if (!m) return null;
  if (m[2] && unit === 'unit') return null;
  const scale = BigInt(m[2] ? UNIT_SCALE[m[2]] : 1) * 1000n;
  const [whole, frac = ''] = m[1].replace(',', '.').split('.');
  const digits = BigInt(whole + frac) * scale;
  const div = 10n ** BigInt(frac.length);
  if (digits % div !== 0n) return null;
  const n = Number(digits / div);
  return Number.isSafeInteger(n) ? n : null;
}

/// Milli-units as the wire and the screen write them: 2500 -> "2.5",
/// 1500000 -> "1500" (CONTRACT-QTY §1: send the TEXT, the Worker reads it).
export function showMilli(m){
  const neg = m < 0, a = Math.abs(Math.trunc(m));
  const w = Math.trunc(a / 1000), f = a % 1000;
  return (neg ? '-' : '') + String(w) + (f ? '.' + String(f).padStart(3, '0').replace(/0+$/, '') : '');
}

/// A supply's exact level in milli-units: the Stock answer's `<key>Milli`
/// when it has one (`onHandMilli`, `availableMilli`), else the whole `<key>`.
export function levelMilli(sup, key){
  const m = sup?.[key + 'Milli'];
  return Number.isSafeInteger(m) ? m : (Number(sup?.[key]) || 0) * 1000;
}

/// WHY `milli(raw, unit)` answered null: finer than a thousandth of the base
/// unit is 'tooFine'; a kg/l on a piece is 'badUnit'; text is 'notANumber';
/// empty is null. A fraction of a piece is a quantity (W-QTYHUB H3).
export function whyNot(raw, unit = 'g'){
  const s = canonQty(raw);
  if (!s) return null;
  const m = /^(-?\d+(?:[.,]\d+)?)\s*(kg|g|ml|l)?$/.exec(s.replace(/(\d) (?=\d{3}\b)/g, '$1'));
  if (!m) return 'notANumber';
  if (m[2] && unit === 'unit') return 'badUnit';
  return milli(raw, unit) == null ? 'tooFine' : null;
}

/// A typed percent as per mille: "55" -> 550, "55,5" -> 555, "220" -> 2200.
/// More than one decimal, or not a number, is null.
export function pmOfPct(raw){
  const m = /^(\d+)(?:[.,](\d))?$/.exec(String(raw ?? '').trim());
  if (!m) return null;
  return Number(m[1]) * 10 + Number(m[2] || 0);
}

/// `x * pm / 1000`, half up, in integers.
export const scale = (x, pm) => Math.floor((x * pm + PM / 2) / PM);

/// A supply's per-mille default, or 1000 when absent or out of range.
export function pmOf(sup, key, max){
  const v = sup?.[key];
  return Number.isInteger(v) && v >= 1 && v <= max ? v : PM;
}

/// The gross weight of `qty` base units, grams: grams are grams, a millilitre
/// is a gram, a piece weighs `weightPerUnit`.
export function grossG(sup, qty){
  if (sup?.unit === 'unit') return sup.weightPerUnit != null ? Math.round(sup.weightPerUnit * qty) : null;
  return qty;
}

/// One recipe line's gross, net and out (grams), the hub's own rule
/// (`workers/api/src/recipe/weights.rs`): the line's typed net/out win over
/// the supply's `cleanPm` / `cookPm`.
export function weights(sup, qty, net = null, out = null){
  const cleanPm = pmOf(sup, 'cleanPm', CLEAN_MAX), cookPm = pmOf(sup, 'cookPm', COOK_MAX);
  const gross = grossG(sup, qty);
  const n = net ?? (gross == null ? null : scale(gross, cleanPm));
  const o = out ?? (n == null ? null : scale(n, cookPm));
  return { gross, net: n, out: o, cleanPm, cookPm };
}

/// Lost between the gross and the plate, per mille (negative: it grew).
export const lossPm = w => (w.gross > 0 && w.out != null ? PM - Math.trunc(w.out * PM / w.gross) : null);

/// A per mille as a percent string: 450 -> "45%", 125 -> "12.5%", -1200 -> "-120%".
export function pct(pm){
  if (pm == null || !Number.isFinite(pm)) return '';
  const t = Math.abs(pm) % 10 ? (pm / 10).toFixed(1) : String(pm / 10);
  return t + '%';
}

/// What `qty` base units cost at `perBasis` minor units per basis, half up.
export const costOf = (perBasis, qty, basis) => (perBasis == null ? null : Math.floor((perBasis * qty * 2 + basis) / (2 * basis)));

/// The cost a line is charged at: the average a priced delivery set, else the
/// owner's list price. Answers `{ perBasis, from: 'wac' | 'list' | null }`.
export function priceOf(sup){
  if (sup?.wac != null) return { perBasis: sup.wac, from: 'wac' };
  if (sup?.costPerBasis != null) return { perBasis: sup.costPerBasis, from: 'list' };
  return { perBasis: null, from: null };
}

/// A delivery's price as the hub takes it: the line's total, or a cost per
/// kg / l / piece. `per` is what the typed price is for, in base units.
export function priceBody(unit, priceTyped, mode){
  if (priceTyped == null) return {};
  if (mode === 'total') return { total: priceTyped };
  return { unitCost: priceTyped, per: unit === 'unit' ? 1 : 1000 };
}

/// Measured yield of a prep, per mille of what went onto the board.
export const yieldPm = (qty, out) => (qty > 0 && out != null ? Math.trunc(out * PM / qty) : null);

/// The tone a lot's date wears: expired, soon (within `warn` days), fine.
export function expiryTone(daysLeft, warn = 2){
  if (daysLeft == null) return null;
  if (daysLeft < 0) return 'bad';
  if (daysLeft <= warn) return 'warn';
  return 'ok';
}

/// Bar heights in per cent of the largest value (0 for none), for `.bars`.
export function bars(values){
  const max = Math.max(0, ...values.map(v => Math.abs(Number(v) || 0)));
  return values.map(v => (max ? Math.round(100 * Math.abs(Number(v) || 0) / max) : 0));
}

/// A stock level as the screen says it: an uncounted negative is "needs a
/// count" (I0), never an error; a counted one is its number.
export function levelState(sup){
  if (!sup?.counted) return (sup?.onHand ?? 0) < 0 ? 'needsCount' : 'uncounted';
  if ((sup.available ?? 0) <= 0) return 'out';
  if (sup.low) return 'low';
  return 'ok';
}

/// The dishes that take nothing off the shelf, grouped by category id, each
/// group sorted by name: `[[categoryId, [dish...]], ...]`.
export function byCategory(dishes){
  const m = new Map();
  for (const d of dishes || []) {
    const c = d.categoryId || '';
    if (!m.has(c)) m.set(c, []);
    m.get(c).push(d);
  }
  return [...m.entries()].map(([c, ds]) => [c, ds.slice().sort((a, b) => String(a.name).localeCompare(String(b.name)))]);
}
