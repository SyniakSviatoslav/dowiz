// A QUANTITY ON THE WIRE, ONE ADAPTER (lane W-TKUI, 2026-10-12, operator TZ 1;
// the format is /root/lanes/CONTRACT-QTY.md section 1). The editor types a
// decimal ("0,5", "40.5", "1,5 kg"); the Worker parses the TEXT into integer
// milli-units (thousandths of the supply's base unit), never through f64.
// So does this file: a quantity is held as an integer `milli` in JS, shown as
// a short decimal, sent as a decimal STRING (or the integer when whole).
// Every arithmetic the editor previews (cost, grams) takes `milli`, so no
// `Number(qty) * x` drifts. No DOM, no network (`qty-adapter.test.mjs`).
// ASCII QUOTES ONLY in this file (DOWIZ-COMMON-RULES rule 11).

export const MILLI = 1000;
const SCALE = { g: 1, ml: 1, kg: 1000, l: 1000 };

/// Typed text -> integer milli base units, or null (empty / not a number /
/// more than 3 decimals / a kg|l word on a piece supply / negative).
/// `unit` is the SUPPLY's base unit: 'g' | 'ml' | 'unit'.
export function parseMilli(raw, unit = 'g'){
  const s = String(raw ?? '').trim().toLowerCase().replace(/\s+/g, ' ');
  if (!s) return null;
  const m = /^(\d+(?:[.,]\d+)?|[.,]\d+)\s*(kg|g|ml|l)?$/.exec(s.replace(/(\d) (?=\d{3}\b)/g, '$1'));
  if (!m) return null;
  if (m[2] && unit === 'unit') return null;
  const scale = m[2] ? SCALE[m[2]] : 1;
  const [whole = '', frac = ''] = m[1].replace(',', '.').split('.');
  if (frac.length > 3 && /[1-9]/.test(frac.slice(3))) return null;
  const f3 = (frac + '000').slice(0, 3);
  const milli = BigInt(whole || '0') * 1000n * BigInt(scale) + BigInt(f3) * BigInt(scale);
  const n = Number(milli);
  return Number.isSafeInteger(n) ? n : null;
}

/// Milli -> the number the hub answers as `qty` (a decimal with <=3 places).
export const fromMilli = milli => milli / MILLI;
/// The hub's `qty` (40 or 40.5) or `qtyMilli` -> milli, exactly (via the text).
export const toMilli = (qty, qtyMilli = null) => (qtyMilli != null ? Math.round(Number(qtyMilli)) : parseMilli(String(qty ?? '')));

/// Milli as the short decimal the owner reads: 40500 -> "40.5", 40000 -> "40", 500 -> "0.5".
export function showMilli(milli){
  if (milli == null || !Number.isFinite(milli)) return '';
  const neg = milli < 0 ? '-' : '', a = Math.abs(Math.round(milli));
  const frac = String(a % MILLI).padStart(3, '0').replace(/0+$/, '');
  return `${neg}${Math.floor(a / MILLI)}${frac ? '.' + frac : ''}`;
}

/// What goes on the wire: the integer when whole, else the decimal as a string.
export const wireQty = milli => (milli % MILLI === 0 ? milli / MILLI : showMilli(milli));

/// A step of `stepMilli`, never below zero.
export const stepMilli = (milli, by) => Math.max(0, milli + by);
