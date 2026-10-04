// AN INVOICE'S LINES, READ OFF A PHOTO (W-OCR, research 2026-10-03 row P10), PURE:
// no DOM, no clock, no network, so every rule here runs in node
// (`receipt-photo-logic.test.mjs`). The OCR itself (tesseract.js, `sqi+eng`,
// vendored under /lib/ocr) runs in `receipt-photo.js`; this file only reads
// the text it returns.
//
//   "Salmon file Norvegjeze 2,5 kg 1.800 4.500"
//     -> { text: 'Salmon file Norvegjeze', qty: '2,5' as {v: 25, d: 1}, unit: 'kg',
//          unit_price: 1800, total: 4500, flags: [] }
//
// NUMBERS are read as exact decimals {v, d} (value = v / 10^d) -- never a JS
// float -- in both the Albanian/continental form (1.234,50) and the English one
// (1,234.50). A single separator followed by exactly three digits is a
// THOUSANDS separator ("1.800" lek, "2.500" g), anything else a decimal point.
//
// MONEY IS WHOLE LEK. A price with nonzero qindarka is rounded half up in
// integers and the line is flagged `rounded`, so the owner sees it before
// anything is written.
//
// THE CROSS-CHECK: qty x unit_price must equal total within one lek; a line
// where it does not is flagged `mismatch` (a misread digit is far likelier
// than a supplier's arithmetic error, and the owner must look).

/// The units an invoice line may carry, as the console's base units scale them.
/// `copë` (piece) is Albanian; OCR often drops the diacritic.
export const UNITS = {
  kg: { base: 'g', mul: 1000 }, g: { base: 'g', mul: 1 }, gr: { base: 'g', mul: 1 },
  l: { base: 'ml', mul: 1000 }, lt: { base: 'ml', mul: 1000 }, litër: { base: 'ml', mul: 1000 }, liter: { base: 'ml', mul: 1000 }, ml: { base: 'ml', mul: 1 },
  // OCR reads ë as é about half the time on synthetic invoices (measure.mjs).
  'copë': { base: 'unit', mul: 1 }, 'copé': { base: 'unit', mul: 1 }, cope: { base: 'unit', mul: 1 }, 'copa': { base: 'unit', mul: 1 }, cop: { base: 'unit', mul: 1 },
  pcs: { base: 'unit', mul: 1 }, pc: { base: 'unit', mul: 1 }, pce: { base: 'unit', mul: 1 }, 'cp': { base: 'unit', mul: 1 },
};
/// The currency words a line may end with; they are dropped, never parsed.
const CURRENCY = new Set(['lek', 'leke', 'lekë', 'l', 'all']);
/// Lines that are the invoice's sums, taxes or header -- never a delivered good.
const NOT_A_LINE = /\b(totali?|total|subtotal|nëntotal|nentotal|gjithsej|shuma|tvsh|vat|nipt|nuis|fatura|invoice|pagesa|payment|data|date|blerësi|bleresi|shitësi|shitesi|buyer|seller|zbritje|discount)\b/i;
const NUM = /^\d{1,3}(?:[.,\s]\d{3})*(?:[.,]\d+)?$|^\d+(?:[.,]\d+)?$/;

/// "1.234,50" | "1,234.50" | "1 234" | "2,5" | "1.800" -> {v, d} exact, or null.
export function num(raw){
  let s = String(raw ?? '').trim().replace(/[  ]/g, ' ');
  if (!s || !NUM.test(s)) return null;
  s = s.replace(/ /g, '');
  const lastDot = s.lastIndexOf('.'), lastComma = s.lastIndexOf(',');
  let int = s, frac = '';
  if (lastDot >= 0 && lastComma >= 0) {
    // Both: the LAST one is the decimal point, the other groups thousands.
    const dec = Math.max(lastDot, lastComma);
    int = s.slice(0, dec).replace(/[.,]/g, ''); frac = s.slice(dec + 1);
  } else if (lastDot >= 0 || lastComma >= 0) {
    const sep = lastDot >= 0 ? '.' : ',';
    const parts = s.split(sep);
    if (parts.length > 2 || parts[parts.length - 1].length === 3) {
      int = parts.join('');
      // "2.500" is 2500 -- or 2,5 written with three decimals, as scales print
      // kilograms. `alt` keeps the second reading for the cross-check to try.
      if (parts.length === 2) { const alt = exact(parts[0], parts[1]); const n = exact(int, ''); return n && { ...n, alt }; }
    } else { int = parts[0]; frac = parts[1]; }
  }
  return exact(int, frac);
}

function exact(int, frac){
  if (!/^\d+$/.test(int) || !/^\d*$/.test(frac)) return null;
  // Trailing zeros of the fraction carry nothing: 450,00 is 450.
  const f = frac.replace(/0+$/, '');
  const v = Number(int + f);
  return Number.isSafeInteger(v) ? { v, d: f.length } : null;
}

const POW = d => 10 ** d;

/// {v, d} as whole lek, half up; `rounded` when qindarka were dropped.
export function lek(n){
  if (!n) return null;
  if (n.d === 0) return { lek: n.v, rounded: false };
  const p = POW(n.d);
  return { lek: Math.floor((n.v * 2 + p) / (2 * p)), rounded: n.v % p !== 0 };
}

/// An invoice quantity in the supply's base unit (g / ml / unit), whole, or null
/// when it does not fit: kg on a piece supply, or a fraction of a gram.
export function toBase(qty, unit, supplyUnit){
  if (!qty) return null;
  const u = UNITS[String(unit || '').toLowerCase()];
  const mul = u ? u.mul : 1;
  if (u && supplyUnit && u.base !== supplyUnit) return null;
  const scaled = qty.v * mul, p = POW(qty.d);
  return scaled % p === 0 ? scaled / p : null;
}

/// The words of a line as an alias key: trimmed, lower case, one space between
/// words -- the SAME rule as `aliases::key` in the Worker (aliases.rs).
export const aliasKey = text => String(text ?? '').split(/\s+/).filter(Boolean).join(' ').toLowerCase();

const unitOf = tok => {
  const k = tok.toLowerCase().replace(/[.:]$/, '');
  return UNITS[k] ? k : null;
};

/// |qty x price - total| <= 1 lek, exactly: compared in units of 10^-qty.d lek.
const agrees = (qty, price, total) => Math.abs(qty.v * price - total * POW(qty.d)) <= POW(qty.d);

/// One OCR line -> a line, or null when it is not one (no words, no total, a sum).
export function parseLine(raw){
  const line = String(raw ?? '').replace(/[|‖¦]/g, ' ').replace(/\s+/g, ' ').trim();
  if (!line || NOT_A_LINE.test(line)) return null;
  // "2,5kg" and "4.500L" are one token each on paper; split the unit off.
  const toks = line.split(' ').flatMap(t => {
    const m = /^(\d[\d.,]*)([a-zA-Zëç]+)\.?$/.exec(t);
    return m && (unitOf(m[2]) || CURRENCY.has(m[2].toLowerCase())) ? [m[1], m[2]] : [t];
  });
  // From the right: numbers, units and currency words; the rest is the text.
  const nums = []; let unit = null, i = toks.length - 1;
  for (; i >= 0; i--) {
    const t = toks[i], low = t.toLowerCase().replace(/[.:]$/, '');
    // "L" after the total or the price is lek; after both, a litre.
    if (low === 'l' ? nums.length <= 1 : CURRENCY.has(low)) continue;
    // A unit after all three numbers belongs to them only when no number sits
    // left of it ("Salmon kg 2,5 ..."); "Cola 0,33 l 24 80 1.920" keeps "0,33 l".
    const u = unitOf(t);
    if (u && !unit && (nums.length < 3 || !num(toks[i - 1] ?? ''))) { unit = u; continue; }
    const n = num(t);
    if (n && nums.length < 3) { nums.unshift(n); continue; }
    break;
  }
  // A leading row number ("1", "1.", "1)") and a code are not the product's words.
  const words = toks.slice(0, i + 1).filter((t, j) => !(j === 0 && /^\d{1,3}[.)]?$/.test(t)));
  const text = words.join(' ').replace(/^[-–:#*.\s]+|[-–:#*\s]+$/g, '');
  if (!/\p{L}{2}/u.test(text) || nums.length < 2) return null;
  const flags = [];
  let qty, price, total;
  if (nums.length === 3) [qty, price, total] = nums;
  else { [qty, total] = nums; price = null; flags.push('no_price'); }
  const t = lek(total), p = price ? lek(price) : null;
  if (!t) return null;
  if (t.rounded || p?.rounded) flags.push('rounded');
  if (p && qty) {
    if (!agrees(qty, p.lek, t.lek) && qty.alt && agrees(qty.alt, p.lek, t.lek)) qty = qty.alt;
    else if (!agrees(qty, p.lek, t.lek)) flags.push('mismatch');
  }
  if (qty) qty = { v: qty.v, d: qty.d };
  return { text, key: aliasKey(text), qty, unit: unit || '', unit_price: p ? p.lek : null, total: t.lek, flags };
}

/// The invoice's paper: its number and the supplier's NIPT, when printed.
export function header(lines){
  const all = (lines || []).join('\n');
  const nipt = /\b([A-Z]\d{8}[A-Z])\b/.exec(all.toUpperCase());
  const doc = /(?:fatur[ëea]?\s*(?:nr|n[ru]mër|numri)?|invoice\s*(?:no|nr|number|#)?)\.?\s*[:#]?\s*([A-Z0-9/\-]{0,12}\d[A-Z0-9/\-]{0,12})/i.exec(all);
  return { nipt: nipt ? nipt[1] : '', doc: doc ? doc[1] : '' };
}

/// The whole OCR text -> { lines, nipt, doc }.
export function parseText(text){
  const raw = String(text ?? '').split(/\r?\n/);
  return { ...header(raw), lines: raw.map(parseLine).filter(Boolean) };
}

const fold = s => s.normalize('NFD').replace(/\p{Diacritic}/gu, '').toLowerCase();
const wordsOf = s => fold(s).split(/[^a-z0-9а-яіїєґ]+/).filter(w => w.length > 2);

/// Which supply a line is: the supplier's remembered alias first, then the
/// supply whose name shares the most words with the line (a SUGGESTION the
/// owner confirms), else none. `known` is `supplierAliases[card].lines`.
export function match(line, known, supplies){
  const hit = known?.[line.key];
  if (hit && (supplies || []).some(s => s.id === hit)) return { item: hit, by: 'alias' };
  const mine = new Set(wordsOf(line.text));
  let best = null, bestN = 0;
  for (const s of supplies || []) {
    const n = wordsOf(`${s.name || ''} ${s.id}`).filter(w => mine.has(w)).length;
    if (n > bestN) { best = s.id; bestN = n; }
  }
  return best ? { item: best, by: 'name' } : { item: '', by: '' };
}

/// The receipt request for one confirmed line (`POST /api/owner/stock/received`),
/// or `{error}`. The invoice's TOTAL is sent, so the shelf's price is what was paid.
export function receivedBody(line, item, supplyUnit, supplier, doc){
  if (!item) return { error: 'rp_pickSupply' };
  const qty = toBase(line.qty, line.unit, supplyUnit);
  if (qty == null || qty <= 0) return { error: 'rp_badQty' };
  const body = { item, qty, total: line.total };
  if (supplier) body.supplier = supplier;
  if (doc) body.doc = doc;
  return { body };
}

/// The alias request after a confirmation (`POST /api/owner/stock/alias`):
/// only lines whose match the owner set or changed -- the Worker writes only
/// what differs from what it knows anyway.
export function aliasBody(cardId, nipt, confirmed, known){
  const lines = confirmed.filter(c => c.item && known?.[c.key] !== c.item).map(c => ({ text: c.key, item: c.item }));
  const card = { supplier: cardId, lines };
  if (nipt) card.nipt = nipt;
  return { card };
}

/// How many fields the owner had to change to reach `want` from what was read:
/// per expected line, a missing line counts its 4 fields, otherwise one per
/// wrong field (item, qty, unit price, total). The fixtures' accuracy measure.
export function edits(got, want){
  let n = 0, exact = 0;
  for (const w of want) {
    const g = got.find(x => x.key === aliasKey(w.text)) || got[want.indexOf(w)];
    if (!g) { n += 4; continue; }
    let e = 0;
    if ((g.item || '') !== (w.item || '')) e++;
    if (!g.qty || g.qty.v / POW(g.qty.d) !== Number(w.qty)) e++;
    if (g.unit_price !== w.unit_price) e++;
    if (g.total !== w.total) e++;
    if (e === 0) exact++;
    n += e;
  }
  return { edits: n, exact, of: want.length };
}
